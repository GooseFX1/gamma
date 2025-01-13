use super::swap_base_input::Swap;
use crate::curve::{calculator::CurveCalculator, TradeDirection};
use crate::error::GammaError;
use crate::external::dflow_segmenter::is_invoked_by_segmenter;
use crate::states::{oracle, PoolStatusBitIndex, SwapEvent};
use crate::utils::{swap_referral::*, token::*};
use crate::SwapRemainingAccounts;
use anchor_lang::prelude::*;
use anchor_lang::solana_program;

pub fn swap_base_output<'c, 'info>(
    ctx: Context<'_, '_, 'c, 'info, Swap<'info>>,
    max_amount_in: u64,
    amount_out_less_fee: u64,
) -> Result<()> {
    let swap_remaining_accounts = SwapRemainingAccounts::new(&ctx.remaining_accounts);
    let referral_info = extract_referral_info(
        ctx.accounts.input_token_mint.key(),
        ctx.accounts.amm_config.referral_project,
        &swap_remaining_accounts.referral_account,
        &swap_remaining_accounts.referral_token_account,
    )?;
    let block_timestamp = solana_program::clock::Clock::get()?.unix_timestamp as u64;
    let pool_id = ctx.accounts.pool_state.key();
    let pool_state = &mut ctx.accounts.pool_state.load_mut()?;
    if !pool_state.get_status_by_bit(PoolStatusBitIndex::Swap)
        || block_timestamp < pool_state.open_time
    {
        return err!(GammaError::NotApproved);
    }

    let (token_0_price_x64_before_swap, token_1_price_x64_before_swap) =
        if ctx.accounts.input_vault.key() == pool_state.token_0_vault
            && ctx.accounts.output_vault.key() == pool_state.token_1_vault
        {
            pool_state.token_price_x32()?
        } else if ctx.accounts.input_vault.key() == pool_state.token_1_vault
            && ctx.accounts.output_vault.key() == pool_state.token_0_vault
        {
            pool_state.token_price_x32()?
        } else {
            return err!(GammaError::InvalidVault);
        };

    let out_transfer_fee = get_transfer_inverse_fee(
        &ctx.accounts.output_token_mint.to_account_info(),
        amount_out_less_fee,
    )?;
    let actual_amount_out = amount_out_less_fee
        .checked_add(out_transfer_fee)
        .ok_or(GammaError::MathOverflow)?;

    // Calculate the trade amounts
    let (trade_direction, total_input_token_amount, total_output_token_amount) =
        if ctx.accounts.input_vault.key() == pool_state.token_0_vault
            && ctx.accounts.output_vault.key() == pool_state.token_1_vault
        {
            let (total_input_token_amount, total_output_token_amount) =
                pool_state.vault_amount_without_fee()?;

            (
                TradeDirection::ZeroForOne,
                total_input_token_amount,
                total_output_token_amount,
            )
        } else if ctx.accounts.input_vault.key() == pool_state.token_1_vault
            && ctx.accounts.output_vault.key() == pool_state.token_0_vault
        {
            let (total_output_token_amount, total_input_token_amount) =
                pool_state.vault_amount_without_fee()?;

            (
                TradeDirection::OneForZero,
                total_input_token_amount,
                total_output_token_amount,
            )
        } else {
            return err!(GammaError::InvalidVault);
        };
    let constant_before = u128::from(total_input_token_amount)
        .checked_mul(u128::from(total_output_token_amount))
        .ok_or(GammaError::MathOverflow)?;

    let mut observation_state = ctx.accounts.observation_state.load_mut()?;

    let mut is_invoked_by_signed_segmenter = false;
    if swap_remaining_accounts.registered_segmenter.is_some()
        && swap_remaining_accounts.registry.is_some()
    {
        is_invoked_by_signed_segmenter = is_invoked_by_segmenter(
            &swap_remaining_accounts.registry.as_ref().unwrap(),
            &swap_remaining_accounts
                .registered_segmenter
                .as_ref()
                .unwrap(),
        );
    }

    let result = match CurveCalculator::swap_base_output(
        u128::from(actual_amount_out),
        u128::from(total_input_token_amount),
        u128::from(total_output_token_amount),
        &ctx.accounts.amm_config,
        &pool_state,
        block_timestamp,
        &observation_state,
        is_invoked_by_signed_segmenter,
    ) {
        Ok(value) => value,
        Err(_) => return err!(GammaError::ZeroTradingTokens),
    };

    let constant_after = u128::from(
        result
            .new_swap_source_amount
            .checked_sub(result.dynamic_fee)
            .ok_or(GammaError::MathOverflow)?,
    )
    .checked_mul(u128::from(result.new_swap_destination_amount))
    .ok_or(GammaError::MathOverflow)?;

    #[cfg(feature = "enable-log")]
    msg!(
        "source_amount_swapped:{}, destination_amount_swapped:{}, dynamic_fee: {}, constant_before:{},constant_after:{}",
        result.source_amount_swapped,
        result.destination_amount_swapped,
        result.dynamic_fee,
        constant_before,
        constant_after
    );

    // Re-calculate the source amount swapped based on what the curve says
    let (mut input_transfer_amount, input_transfer_fee) = {
        let source_amount_swapped = match u64::try_from(result.source_amount_swapped) {
            Ok(value) => value,
            Err(_) => return err!(GammaError::MathOverflow),
        };
        require_gt!(source_amount_swapped, 0);
        let transfer_fee = get_transfer_inverse_fee(
            &ctx.accounts.input_token_mint.to_account_info(),
            source_amount_swapped,
        )?;
        let input_transfer_amount = source_amount_swapped
            .checked_add(transfer_fee)
            .ok_or(GammaError::MathOverflow)?;
        require_gte!(
            max_amount_in,
            input_transfer_amount,
            GammaError::ExceededSlippage
        );
        (input_transfer_amount, transfer_fee)
    };
    let destination_amount_swapped = match u64::try_from(result.destination_amount_swapped) {
        Ok(value) => value,
        Err(_) => return err!(GammaError::MathOverflow),
    };
    require_eq!(destination_amount_swapped, actual_amount_out);
    let (output_transfer_amount, output_transfer_fee) = (actual_amount_out, out_transfer_fee);

    let protocol_fee = u64::try_from(result.protocol_fee).or(err!(GammaError::MathOverflow))?;
    let fund_fee = u64::try_from(result.fund_fee).or(err!(GammaError::MathOverflow))?;
    let mut dynamic_fee = u64::try_from(result.dynamic_fee).or(err!(GammaError::MathOverflow))?;

    let mut source_amount_swapped =
        u64::try_from(result.source_amount_swapped).or(err!(GammaError::MathOverflow))?;

    if let Some(info) = referral_info {
        let referral_amount = dynamic_fee
            .saturating_sub(protocol_fee)
            .saturating_sub(fund_fee)
            .checked_mul(info.share_bps as u64)
            .ok_or(GammaError::MathOverflow)?
            .checked_div(10_000)
            .unwrap_or(0);

        let referral_transfer_fee = get_transfer_fee(
            &ctx.accounts.input_token_mint.to_account_info(),
            referral_amount,
        )?;

        // We are aware of the fact that when referral fees are very small the referee will not get any tokens
        if referral_amount != 0 && referral_transfer_fee < referral_amount {
            // subtract referral amount from dynamic fee and transfer amount
            dynamic_fee = dynamic_fee
                .checked_sub(referral_amount)
                .ok_or(GammaError::MathError)?;
            input_transfer_amount = input_transfer_amount
                .checked_sub(referral_amount)
                .ok_or(GammaError::MathError)?;
            source_amount_swapped = source_amount_swapped
                .checked_sub(referral_amount)
                .ok_or(GammaError::MathError)?;

            anchor_spl::token_2022::transfer_checked(
                CpiContext::new(
                    ctx.accounts.input_token_program.to_account_info(),
                    anchor_spl::token_2022::TransferChecked {
                        from: ctx.accounts.input_token_account.to_account_info(),
                        to: info.referral_token_account.to_account_info(),
                        authority: ctx.accounts.payer.to_account_info(),
                        mint: ctx.accounts.input_token_mint.to_account_info(),
                    },
                ),
                referral_amount,
                ctx.accounts.input_token_mint.decimals,
            )?;
        }
    }

    // Save fees metric for the pool partners.
    let mut partners = pool_state.partners;
    for partner in partners.iter_mut() {
        // we multiply by 100000 to keep decimals.
        let decimal_number = 100000;
        let tvl_share = partner
            .lp_token_linked_with_partner
            .checked_mul(decimal_number)
            .ok_or(GammaError::MathOverflow)?
            .checked_div(pool_state.lp_supply)
            .ok_or(GammaError::MathOverflow)?;

        let partner_fee = protocol_fee
            .checked_mul(tvl_share)
            .ok_or(GammaError::MathOverflow)?
            .checked_div(decimal_number)
            .ok_or(GammaError::MathOverflow)?;

        match trade_direction {
            TradeDirection::ZeroForOne => {
                partner.cumulative_fee_total_times_tvl_share_token_0 = partner
                    .cumulative_fee_total_times_tvl_share_token_0
                    .checked_add(partner_fee)
                    .ok_or(GammaError::MathOverflow)?;
            }
            TradeDirection::OneForZero => {
                partner.cumulative_fee_total_times_tvl_share_token_1 = partner
                    .cumulative_fee_total_times_tvl_share_token_1
                    .checked_add(partner_fee)
                    .ok_or(GammaError::MathOverflow)?;
            }
        }
    }
    pool_state.partners = partners;

    match trade_direction {
        TradeDirection::ZeroForOne => {
            pool_state.protocol_fees_token_0 = pool_state
                .protocol_fees_token_0
                .checked_add(protocol_fee)
                .ok_or(GammaError::MathOverflow)?;
            pool_state.fund_fees_token_0 = pool_state
                .fund_fees_token_0
                .checked_add(fund_fee)
                .ok_or(GammaError::MathOverflow)?;
            pool_state.cumulative_trade_fees_token_0 = pool_state
                .cumulative_trade_fees_token_0
                .checked_add(dynamic_fee as u128)
                .ok_or(GammaError::MathOverflow)?;
            pool_state.cumulative_volume_token_0 = pool_state
                .cumulative_volume_token_0
                .checked_add(source_amount_swapped as u128)
                .ok_or(GammaError::MathOverflow)?;

            pool_state.token_0_vault_amount = pool_state
                .token_0_vault_amount
                .checked_add(source_amount_swapped)
                .ok_or(GammaError::MathOverflow)?
                .checked_sub(fund_fee)
                .ok_or(GammaError::MathOverflow)?
                .checked_sub(protocol_fee)
                .ok_or(GammaError::MathOverflow)?;

            pool_state.token_1_vault_amount = pool_state
                .token_1_vault_amount
                .checked_sub(output_transfer_amount)
                .ok_or(GammaError::MathOverflow)?;
        }
        TradeDirection::OneForZero => {
            pool_state.protocol_fees_token_1 = pool_state
                .protocol_fees_token_1
                .checked_add(protocol_fee)
                .ok_or(GammaError::MathOverflow)?;
            pool_state.fund_fees_token_1 = pool_state
                .fund_fees_token_1
                .checked_add(fund_fee)
                .ok_or(GammaError::MathOverflow)?;
            pool_state.cumulative_trade_fees_token_1 = pool_state
                .cumulative_trade_fees_token_1
                .checked_add(dynamic_fee as u128)
                .ok_or(GammaError::MathOverflow)?;
            pool_state.cumulative_volume_token_1 = pool_state
                .cumulative_volume_token_1
                .checked_add(source_amount_swapped as u128)
                .ok_or(GammaError::MathOverflow)?;

            pool_state.token_1_vault_amount = pool_state
                .token_1_vault_amount
                .checked_add(source_amount_swapped)
                .ok_or(GammaError::MathOverflow)?
                .checked_sub(fund_fee)
                .ok_or(GammaError::MathOverflow)?
                .checked_sub(protocol_fee)
                .ok_or(GammaError::MathOverflow)?;

            pool_state.token_0_vault_amount = pool_state
                .token_0_vault_amount
                .checked_sub(output_transfer_amount)
                .ok_or(GammaError::MathOverflow)?;
        }
    };
    pool_state.latest_dynamic_fee_rate = result.dynamic_fee_rate;

    emit!(SwapEvent {
        pool_id,
        input_vault_before: total_input_token_amount,
        output_vault_before: total_output_token_amount,
        input_amount: match u64::try_from(result.source_amount_swapped) {
            Ok(value) => value,
            Err(_) => return err!(GammaError::MathOverflow),
        },
        output_amount: match u64::try_from(result.destination_amount_swapped) {
            Ok(value) => value,
            Err(_) => return err!(GammaError::MathOverflow),
        },
        input_transfer_fee,
        output_transfer_fee,
        base_input: false,
        dynamic_fee: result.dynamic_fee,
    });
    require_gte!(constant_after, constant_before);

    transfer_from_user_to_pool_vault(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.input_token_account.to_account_info(),
        ctx.accounts.input_vault.to_account_info(),
        ctx.accounts.input_token_mint.to_account_info(),
        ctx.accounts.input_token_program.to_account_info(),
        input_transfer_amount,
        ctx.accounts.input_token_mint.decimals,
    )?;

    transfer_from_pool_vault_to_user(
        ctx.accounts.authority.to_account_info(),
        ctx.accounts.output_vault.to_account_info(),
        ctx.accounts.output_token_account.to_account_info(),
        ctx.accounts.output_token_mint.to_account_info(),
        ctx.accounts.output_token_program.to_account_info(),
        output_transfer_amount,
        ctx.accounts.output_token_mint.decimals,
        &[&[crate::AUTH_SEED.as_bytes(), &[pool_state.auth_bump]]],
    )?;

    observation_state.update(
        oracle::block_timestamp()?,
        token_0_price_x64_before_swap,
        token_1_price_x64_before_swap,
    )?;
    pool_state.recent_epoch = Clock::get()?.epoch;

    Ok(())
}
