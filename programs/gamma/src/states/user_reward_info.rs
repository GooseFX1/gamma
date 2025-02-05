use anchor_lang::prelude::*;

use crate::error::GammaError;

use super::{GlobalRewardInfo, RewardInfo};

#[account]
pub struct UserRewardInfoPerMint {
    pub user_pool_lp_account: Pubkey, // The user’s LP account.
    pub reward_info: Pubkey,          // The reward info account.
    pub total_claimed: u64,           // Total rewards claimed by the user.
    pub total_rewards: u64,           // Accumulated rewards yet to be claimed.
    pub updated_at: u64, // Last time this account was updated. i.e the amount was calculate at.
}

impl UserRewardInfoPerMint {
    pub fn calculate_claimable_rewards<'info>(
        &mut self,
        lp_owned_by_user: u64,
        current_lp_supply: u64,
        global_rewards: Account<'info, GlobalRewardInfo>,
        reward_info: Account<'info, RewardInfo>,
    ) -> Result<()> {
        let mut last_disbursed_till = reward_info.start_at.max(self.updated_at);

        for snapshot in &global_rewards.snapshots {
            if last_disbursed_till > snapshot.timestamp {
                continue;
            }
            if reward_info.end_rewards_at < snapshot.timestamp {
                break;
            }

            let duration = snapshot
                .timestamp
                .checked_sub(last_disbursed_till)
                .ok_or(GammaError::MathOverflow)?;

            self.total_rewards = self
                .total_rewards
                .checked_add(
                    reward_info
                        .emission_per_second
                        .checked_mul(duration)
                        .ok_or(GammaError::MathOverflow)?
                        .checked_mul(lp_owned_by_user)
                        .ok_or(GammaError::MathOverflow)?
                        .checked_div(current_lp_supply)
                        .ok_or(GammaError::MathOverflow)?,
                )
                .ok_or(GammaError::MathOverflow)?;

            last_disbursed_till = snapshot.timestamp;
        }

        let clock_current_time = Clock::get()?.unix_timestamp as u64;

        let duration = clock_current_time
            .checked_sub(last_disbursed_till)
            .ok_or(GammaError::MathOverflow)?;

        self.total_rewards = self
            .total_rewards
            .checked_add(
                reward_info
                    .emission_per_second
                    .checked_mul(duration)
                    .ok_or(GammaError::MathOverflow)?
                    .checked_mul(lp_owned_by_user)
                    .ok_or(GammaError::MathOverflow)?
                    .checked_div(current_lp_supply)
                    .ok_or(GammaError::MathOverflow)?,
            )
            .ok_or(GammaError::MathOverflow)?;

        self.updated_at = clock_current_time;

        Ok(())
    }
}
