pub mod curve;
pub mod error;
pub mod fees;
pub mod instructions;
pub mod states;
pub mod utils;

use anchor_lang::prelude::*;
use instructions::*;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "gamma",
    project_url: "https://goosefx.io",
    contacts: "TODO://",
    policy: "TODO://",
    source_code: "https://github.com/GooseFX1/gamma",
    preferred_languages: "en",
    auditors: "TODO://"
}

#[cfg(feature = "devnet")]
declare_id!("DGmWUbYXnyN6YU4a4jy1CJYoPqAnNmZw7DYv32NJB3mX");
#[cfg(not(feature = "devnet"))]
declare_id!("GAMMA7meSFWaBXF25oSUgmGRwaW6sCMFLmBNiMSdbHVT");

pub mod admin {
    use anchor_lang::prelude::declare_id;
    #[cfg(feature = "devnet")]
    //TODO: change this to the correct id
    declare_id!("GAUT8jcHoYoiygCQV5MQHYceGCxc9NKMhQsDs4t9jJed");
    #[cfg(feature = "test-sbf")]
    declare_id!("CixMbUaUgLUg9REWvnwKDi1pqPMCT2oFfQ2SG4BMeBkZ");
    #[cfg(all(not(feature = "devnet"), not(feature = "test-sbf")))]
    //TODO: change this to the correct id
    declare_id!("9QcHinaHcJFdzSHeiF1yGchcuQk3qPFNV13q6dZJbAny");
}

pub mod create_pool_fee_reveiver {
    use anchor_lang::prelude::declare_id;
    #[cfg(feature = "devnet")]
    declare_id!("2EGutuSu6BMjw2fKuU5q5fVTH1S5c4tsTgEwQZDn1N45");
    #[cfg(not(feature = "devnet"))]
    declare_id!("8PhehuioLjhJ35A5eavazJSwoXcA4J7WwzgoWDBDFSuY");
}

pub const AUTH_SEED: &str = "vault_and_lp_mint_auth_seed";

#[program]
pub mod gamma {
    use crate::fees::FEE_RATE_DENOMINATOR_VALUE;

    use super::*;

    /// The configuation of AMM protocol, include trade fee and protocol fee
    /// # Arguments
    ///
    /// * `ctx`- The accounts needed by instruction.
    /// * `index` - The index of amm config, there may be multiple config.
    /// * `trade_fee_rate` - Trade fee rate, can be changed.
    /// * `protocol_fee_rate` - The rate of protocol fee within tarde fee.
    /// * `fund_fee_rate` - The rate of fund fee within tarde fee.
    ///
    pub fn create_amm_config(
        ctx: Context<CreateAmmConfig>,
        index: u16,
        trade_fee_rate: u64,
        protocol_fee_rate: u64,
        fund_fee_rate: u64,
        create_pool_fee: u64,
        max_open_time: u64,
    ) -> Result<()> {
        assert!(trade_fee_rate < FEE_RATE_DENOMINATOR_VALUE);
        assert!(protocol_fee_rate <= FEE_RATE_DENOMINATOR_VALUE);
        assert!(fund_fee_rate <= FEE_RATE_DENOMINATOR_VALUE);
        assert!(fund_fee_rate + protocol_fee_rate <= FEE_RATE_DENOMINATOR_VALUE);
        instructions::create_amm_config(
            ctx,
            index,
            trade_fee_rate,
            protocol_fee_rate,
            fund_fee_rate,
            create_pool_fee,
            max_open_time,
        )
    }

    /// Initialize swap referrals for an existing AMM config
    /// # Arguments
    ///
    /// * `ctx` - The accounts needed by the instruction, including those used by cpi to the referral program
    /// * `name` - The project name, passed to the referral program. Must be less than 50 chars in length
    /// * `default_share_bps` - Percentage share of fees to referrers. Must be less than 10_000
    pub fn create_swap_referral(
        ctx: Context<CreateReferralProject>,
        name: String,
        default_share_bps: u16,
    ) -> Result<()> {
        instructions::create_referral_project(ctx, name, default_share_bps)
    }

    /// Updates the owner of the amm config
    /// Must be called by the current owner or admin
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `trade_fee_rate`- The new trade fee rate of amm config, be set when `param` is 0
    /// * `protocol_fee_rate`- The new protocol fee rate of amm config, be set when `param` is 1
    /// * `fund_fee_rate`- The new fund fee rate of amm config, be set when `param` is 2
    /// * `new_owner`- The config's new owner, be set when `param` is 3
    /// * `new_fund_owner`- The config's new fund owner, be set when `param` is 4
    /// * `param`- The vaule can be 0 | 1 | 2 | 3 | 4, otherwise will report a error
    ///
    pub fn update_amm_config(ctx: Context<UpdateAmmConfig>, param: u16, value: u64) -> Result<()> {
        instructions::update_amm_config(ctx, param, value)
    }

    /// Update pool status for given vaule
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `status` - The vaule of status
    ///
    pub fn update_pool_status(ctx: Context<UpdatePoolStatus>, status: u8) -> Result<()> {
        instructions::update_pool_status(ctx, status)
    }

    /// Collect the protocol fee accrued to the pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `amount_0_requested` - The maximum amount of token_0 to send, can be 0 to collect fees in only token_1
    /// * `amount_1_requested` - The maximum amount of token_1 to send, can be 0 to collect fees in only token_0
    ///
    pub fn collect_protocol_fee(
        ctx: Context<CollectProtocolFee>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> Result<()> {
        instructions::collect_protocol_fee(ctx, amount_0_requested, amount_1_requested)
    }

    /// Collect the fund fee accrued to the pool
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `amount_0_requested` - The maximum amount of token_0 to send, can be 0 to collect fees in only token_1
    /// * `amount_1_requested` - The maximum amount of token_1 to send, can be 0 to collect fees in only token_0
    ///
    pub fn collect_fund_fee(
        ctx: Context<CollectFundFee>,
        amount_0_requested: u64,
        amount_1_requested: u64,
    ) -> Result<()> {
        instructions::collect_fund_fee(ctx, amount_0_requested, amount_1_requested)
    }

    /// Creates a pool for the given token pair and the initial price
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `init_amount_0` - the initial amount_0 to deposit
    /// * `init_amount_1` - the initial amount_1 to deposit
    /// * `open_time` - the timestamp allowed for swap
    ///
    pub fn initialize(
        ctx: Context<Initialize>,
        init_amount_0: u64,
        init_amount_1: u64,
        open_time: u64,
    ) -> Result<()> {
        instructions::initialize(ctx, init_amount_0, init_amount_1, open_time)
    }

    pub fn init_user_pool_liquidity(ctx: Context<InitUserPoolLiquidity>) -> Result<()> {
        instructions::init_user_pool_liquidity(ctx)
    }

    /// Creates a pool for the given token pair and the initial price
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `lp_token_amount` - Pool token amount to transfer. token_a and token_b amount are set by the current exchange rate and size of the pool
    /// * `maximum_token_0_amount` -  Maximum token 0 amount to deposit, prevents excessive slippage
    /// * `maximum_token_1_amount` - Maximum token 1 amount to deposit, prevents excessive slippage
    ///
    pub fn deposit(
        ctx: Context<Deposit>,
        lp_token_amount: u64,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        instructions::deposit(
            ctx,
            lp_token_amount,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    /// Withdraw lp for token0 ande token1
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `lp_token_amount` - Amount of pool tokens to burn. User receives an output of token a and b based on the percentage of the pool tokens that are returned.
    /// * `minimum_token_0_amount` -  Minimum amount of token 0 to receive, prevents excessive slippage
    /// * `minimum_token_1_amount` -  Minimum amount of token 1 to receive, prevents excessive slippage
    ///
    pub fn withdraw(
        ctx: Context<Withdraw>,
        lp_token_amount: u64,
        minimum_token_0_amount: u64,
        minimum_token_1_amount: u64,
    ) -> Result<()> {
        instructions::withdraw(
            ctx,
            lp_token_amount,
            minimum_token_0_amount,
            minimum_token_1_amount,
        )
    }

    /// Swap the tokens in the pool base input amount
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `amount_in` -  input amount to transfer, output to DESTINATION is based on the exchange rate
    /// * `minimum_amount_out` -  Minimum amount of output token, prevents excessive slippage
    ///
    pub fn swap_base_input<'c, 'info>(
        ctx: Context<'_, '_, 'c, 'info, Swap<'info>>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<()> {
        instructions::swap_base_input(ctx, amount_in, minimum_amount_out)
    }

    /// Swap the tokens in the pool base output amount
    ///
    /// # Arguments
    ///
    /// * `ctx`- The context of accounts
    /// * `max_amount_in` -  input amount prevents excessive slippage
    /// * `amount_out` -  amount of output token
    ///
    pub fn swap_base_output<'c, 'info>(
        ctx: Context<'_, '_, 'c, 'info, Swap<'info>>,
        max_amount_in: u64,
        amount_out: u64,
    ) -> Result<()> {
        instructions::swap_base_output(ctx, max_amount_in, amount_out)
    }
}
