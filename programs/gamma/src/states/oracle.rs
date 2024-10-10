/// Oracle provides price data useful for a wide variety of system designs
///
use anchor_lang::prelude::*;
use crate::error::GammaError;
/// Seed to derive account address and signature
pub const OBSERVATION_SEED: &str = "observation";
// Number of ObservationState element
pub const OBSERVATION_NUM: usize = 100;

/// The element of observations in ObservationState
#[zero_copy(unsafe)]
#[repr(packed)]
#[derive(Default, Debug)]
pub struct Observation {
    /// The block timestamp of the observation
    pub block_timestamp: u64,
    /// The cumulative of token0 price during the duration time, Q32.32, the remaining 64 bit for overflow
    pub cumulative_token_0_price_x32: u128,
    /// The cumulative of token1 price during the duration time, Q32.32, the remaining 64 bit for overflow
    pub cumulative_token_1_price_x32: u128,
}
impl Observation {
    pub const LEN: usize = 8 + 16 + 16;
}

#[account(zero_copy(unsafe))]
#[repr(packed)]
#[cfg_attr(any(feature = "client", feature = "test-sbf"), derive(Debug))]
pub struct ObservationState {
    /// Whether the ObservationState is enabled
    pub initialized: bool,
    /// The most recently updated index of the observations array
    pub observation_index: u16,
    pub pool_id: Pubkey,
    /// observation array
    pub observations: [Observation; OBSERVATION_NUM],
    /// padding
    pub padding: [u64; 4],
}

impl Default for ObservationState {
    #[inline]
    fn default() -> ObservationState {
        ObservationState {
            initialized: false,
            observation_index: 0,
            pool_id: Pubkey::default(),
            observations: [Observation::default(); OBSERVATION_NUM],
            padding: [0u64; 4],
        }
    }
}

impl ObservationState {
    pub const LEN: usize = 8 + 1 + 2 + 32 + (OBSERVATION_NUM * Observation::LEN) + 4 * 8;

    // Writes an oracle observation to the account, returning the next observation_index.
    /// Writable at most once per 15 seconds. Index represents the most recently written element.
    /// If the index is at the end of the allowable array length (100 - 1), the next index will turn to 0.
    ///
    /// # Arguments
    ///
    /// * `self` - The ObservationState account to write in
    /// * `block_timestamp` - The current timestamp of to update
    /// * `token_0_price_x32` - The token_0_price_x32 at the time of the new observation
    /// * `token_1_price_x32` - The token_1_price_x32 at the time of the new observation
    ///

    pub fn update(
        &mut self,
        block_timestamp: u64,
        token_0_price_x32: u128,
        token_1_price_x32: u128,
    ) -> Result<()> {
        let observation_index = self.observation_index;
        if !self.initialized {
            self.initialized = true;
            self.observations[observation_index as usize].block_timestamp = block_timestamp;
            self.observations[observation_index as usize].cumulative_token_0_price_x32 = 0;
            self.observations[observation_index as usize].cumulative_token_1_price_x32 = 0;
            Ok(())
        } else {
            let last_observation = self.observations[observation_index as usize];
            let delta_time = block_timestamp.saturating_sub(last_observation.block_timestamp);
            if delta_time == 0 {
                return Ok(());
            }
            let delta_token_0_price_x32 = token_0_price_x32.checked_mul(delta_time.into()).ok_or(GammaError::MathOverflow)?;
            let delta_token_1_price_x32 = token_1_price_x32.checked_mul(delta_time.into()).ok_or(GammaError::MathOverflow)?;
            let next_observation_index = if observation_index as usize == OBSERVATION_NUM - 1 {
                0
            } else {
                observation_index + 1
            };
            self.observations[next_observation_index as usize].block_timestamp = block_timestamp;
            // cumulative_token_price_x32 only occupies the first 64 bits, and the remaining 64 bits are used to store overflow data
            self.observations[next_observation_index as usize].cumulative_token_0_price_x32 =
                last_observation
                    .cumulative_token_0_price_x32
                    .wrapping_add(delta_token_0_price_x32);
            self.observations[next_observation_index as usize].cumulative_token_1_price_x32 =
                last_observation
                    .cumulative_token_1_price_x32
                    .wrapping_add(delta_token_1_price_x32);
            self.observation_index = next_observation_index;
            Ok(())
        }
    }
}

/// Returns the block timestamp truncated to 32 bits, i.e. mod 2**32
///
pub fn block_timestamp() -> Result<u64> {
    let clock = match Clock::get() {
        Ok(clock) => clock,
        Err(_) => return err!(GammaError::ClockError),
    };
    Ok(clock.unix_timestamp as u64)
}