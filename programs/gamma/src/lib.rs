pub mod curve;
pub mod error;
mod external;
pub mod fees;
pub mod instructions;
pub mod migration;
pub mod states;
pub mod utils;

use anchor_lang::prelude::*;
use instructions::*;
use migration::*;

declare_id!("GAMMA7meSFWaBXF25oSUgmGRwaW6sCMFLmBNiMSdbHVT");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "gamma",
    project_url: "https://goosefx.io",
    contacts: "https://docs.goosefx.io/",
    policy: "https://docs.goosefx.io/",
    source_code: "https://github.com/GooseFX1/gamma",
    preferred_languages: "en",
    auditors: "https://docs.goosefx.io/goosefx-amm/gamma/audit"
}

pub mod admin {
    use anchor_lang::prelude::declare_id;

    #[cfg(feature = "test-sbf")]
    declare_id!("CixMbUaUgLUg9REWvnwKDi1pqPMCT2oFfQ2SG4BMeBkZ");
    #[cfg(not(feature = "test-sbf"))]
    declare_id!("9QcHinaHcJFdzSHeiF1yGchcuQk3qPFNV13q6dZJbAny");
}

pub mod create_pool_fee_reveiver {
    use anchor_lang::prelude::declare_id;
    declare_id!("8PhehuioLjhJ35A5eavazJSwoXcA4J7WwzgoWDBDFSuY");
}

pub const AUTH_SEED: &str = "vault_and_lp_mint_auth_seed";
pub const GLOBAL_REWARD_INFO_SEED: &str = "global_reward_info_seed";
pub const REWARD_VAULT_SEED: &str = "reward_vault_seed";
pub const REWARD_INFO_SEED: &str = "reward_info_seed";

#[program]
pub mod gamma {
    use super::*;
    use crate::fees::FEE_RATE_DENOMINATOR_VALUE;

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
    /// * `param`- The param of pool status
    /// * `status` - The value
    ///
    pub fn update_pool(ctx: Context<UpdatePool>, param: u32, value: u64) -> Result<()> {
        instructions::update_pool(ctx, param, value)
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
    /// * `max_trade_fee_rate` - The maximum trade fee that can be charged on swaps
    /// * `volatility_factor` - The volatility factor of the pool to determine the trade fee
    ///
    pub fn initialize(
        ctx: Context<Initialize>,
        init_amount_0: u64,
        init_amount_1: u64,
        open_time: u64,
        max_trade_fee_rate: u64,
        volatility_factor: u64,
    ) -> Result<()> {
        instructions::initialize(
            ctx,
            init_amount_0,
            init_amount_1,
            open_time,
            max_trade_fee_rate,
            volatility_factor,
        )
    }

    pub fn init_user_pool_liquidity(
        ctx: Context<InitUserPoolLiquidity>,
        partner: Option<String>,
    ) -> Result<()> {
        instructions::init_user_pool_liquidity(ctx, partner)
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

    pub fn create_rewards(
        ctx: Context<CreateRewards>,
        start_time: u64,
        end_time: u64,
        reward_amount: u64,
    ) -> Result<()> {
        instructions::create_rewards(ctx, start_time, end_time, reward_amount)
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

    /********************* Migration Instructions *********************/

    /// Migrate from Meteora Dlmm to Gamma

    pub fn migrate_meteora_dlmm_to_gamma<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, MeteoraDlmmToGamma<'info>>,
        bin_liquidity_reduction: Vec<crate::external::dlmm::lb_clmm::types::BinLiquidityReduction>,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        migration::meteora::meteora_dlmm_to_gamma(
            ctx,
            bin_liquidity_reduction,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    /// Migrate from Orca Whirlpool to Gamma for token 2022

    pub fn migrate_orca_whirlpool_to_gamma_v2<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, OrcaWhirlpoolToGammaV2<'info>>,
        liquidity_amount: u128,
        token_min_a: u64,
        token_min_b: u64,
        remaining_accounts: Option<
            crate::external::whirlpool::whirlpool::types::RemainingAccountsInfo,
        >,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        migration::orca::orca_whirlpool_to_gamma_v2(
            ctx,
            liquidity_amount,
            token_min_a,
            token_min_b,
            remaining_accounts,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    /// Migrate from Orca Whirlpool to Gamma for simple spl tokens

    pub fn migrate_orca_whirlpool_to_gamma<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, OrcaWhirlpoolToGamma<'info>>,
        liquidity_amount: u128,
        token_min_a: u64,
        token_min_b: u64,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        migration::orca::orca_whirlpool_to_gamma(
            ctx,
            liquidity_amount,
            token_min_a,
            token_min_b,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    /// Migrate from Raydium Clmm to Gamma

    pub fn migrate_raydium_clmm_to_gamma<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RaydiumClmmToGamma<'info>>,
        liquidity: u128,
        amount_0_min: u64,
        amount_1_min: u64,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        migration::raydium::raydium_clmm_to_gamma(
            ctx,
            liquidity,
            amount_0_min,
            amount_1_min,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    /// Migrate from Raydium Clmm to Gamma for token 2022

    pub fn migrate_raydium_clmm_to_gamma_v2<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RaydiumClmmToGammaV2<'info>>,
        liquidity: u128,
        amount_0_min: u64,
        amount_1_min: u64,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        migration::raydium::raydium_clmm_to_gamma_v2(
            ctx,
            liquidity,
            amount_0_min,
            amount_1_min,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    /// Migrate from Raydium Cpmm Swap to Gamma

    pub fn migrate_raydium_cp_swap_to_gamma<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RaydiumCpSwapToGamma<'info>>,
        lp_token_amount_withdraw: u64,
        minimum_token_0_amount: u64,
        minimum_token_1_amount: u64,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        migration::raydium::raydium_cp_swap_to_gamma(
            ctx,
            lp_token_amount_withdraw,
            minimum_token_0_amount,
            minimum_token_1_amount,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    pub fn rebalance_kamino<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, Rebalance<'info>>,
    ) -> Result<()> {
        instructions::rebalance_kamino(ctx)
    }
}
