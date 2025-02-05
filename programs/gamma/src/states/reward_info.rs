use anchor_lang::prelude::*;

#[account]
pub struct RewardInfo {
    pub start_at: u64, // Start time for the reward UNIX timestamp.
    pub end_rewards_at: u64,
    pub mint: Pubkey,
    pub total_to_disburse: u64, // Total rewards to distribute in this unix timestamp.
    pub emission_per_second: u64, // Stored for easier maths in the program.
    pub total_left_in_escrow: u64, // Total rewards left in escrow.
    pub rewarded_by: Pubkey,    // The reward given by
}

impl RewardInfo {
    // Even if the current time is less than the start time, the reward is still considered active.
    pub fn is_active(&self, current_time: u64) -> bool {
        self.end_rewards_at > current_time || self.total_left_in_escrow > 0
    }
}
