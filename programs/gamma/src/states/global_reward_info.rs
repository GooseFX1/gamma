use crate::borsh::maybestd::collections::VecDeque;
use anchor_lang::prelude::*;

use super::RewardInfo;

#[account]
pub struct GlobalRewardInfo {
    // This contains the 3 active boosted rewards, i.e. all rewards that are not fully distributed
    // And the current time maybe exceeds the end time of the last boosted reward
    // There is never a proper endtime of the rewards we can even have active boosted rewards if they are not fully distributed yet.
    // Any reward that is not started yet is also consider active.
    pub active_boosted_reward_info: [Pubkey; 3],

    // This contains the minimum start time of all active boosted rewards
    pub min_start_time: u64,

    pub snapshots: VecDeque<Snapshot>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Snapshot {
    // These is to track that the amount was calculated for the boosted rewards
    // at the time of the snapshot for the lp amount
    // If lp amount_reward_0 is equal to total_lp_amount, then the reward has been fully distributed
    // and we can remove the snapshot from the queue
    pub lp_amount_reward_0: u64,
    pub lp_amount_reward_1: u64,
    pub lp_amount_reward_2: u64,
    pub total_lp_amount: u64,
    pub timestamp: u64,
}

impl GlobalRewardInfo {
    pub fn add_new_active_reward(&mut self, reward_info: Pubkey, start_time: u64) {
        for i in 0..3 {
            if self.active_boosted_reward_info[i] == Pubkey::default() {
                self.active_boosted_reward_info[i] = reward_info;
                return;
            }
        }
        self.min_start_time = self.min_start_time.min(start_time);
    }

    pub fn add_snapshot(&mut self, total_lp_amount: u64, timestamp: u64) {
        self.snapshots.push_back(Snapshot {
            total_lp_amount,
            timestamp,
            lp_amount_reward_0: 0,
            lp_amount_reward_1: 0,
            lp_amount_reward_2: 0,
        });
    }

    pub fn remove_inactive_rewards(&mut self, reward_info: Account<RewardInfo>, current_time: u64) {
        for i in 0..3 {
            if self.active_boosted_reward_info[i] == Pubkey::default() {
                continue;
            }

            if self.active_boosted_reward_info[i] != reward_info.key() {
                continue;
            }

            if !reward_info.is_active(current_time) {
                msg!(
                    "Removing reward info as it is inactive and reward info is {}",
                    reward_info.key()
                );
                self.active_boosted_reward_info[i] = Pubkey::default();
            }
        }
    }

    pub fn remove_all_inactive_snapshots(&mut self) {
        let is_reward_one_initialized = self.active_boosted_reward_info[0] != Pubkey::default();
        let is_reward_two_initialized = self.active_boosted_reward_info[1] != Pubkey::default();
        let is_reward_three_initialized = self.active_boosted_reward_info[2] != Pubkey::default();

        if !is_reward_one_initialized && !is_reward_two_initialized && !is_reward_three_initialized
        {
            self.snapshots.clear();
            return;
        }
        // TODO: also drop any snapshot that is before the start time of the reward.

        while let Some(snapshot) = self.snapshots.front() {
            let is_reward_one_fully_distributed_until_this_snapshot =
                snapshot.total_lp_amount == snapshot.lp_amount_reward_0;
            let is_reward_two_fully_distributed_until_this_snapshot =
                snapshot.total_lp_amount == snapshot.lp_amount_reward_1;
            let is_reward_three_fully_distributed_until_this_snapshot =
                snapshot.total_lp_amount == snapshot.lp_amount_reward_2;

            let snapshot_is_required_for_reward_one =
                is_reward_one_initialized && !is_reward_one_fully_distributed_until_this_snapshot;
            let snapshot_is_required_for_reward_two =
                is_reward_two_initialized && !is_reward_two_fully_distributed_until_this_snapshot;
            let snapshot_is_required_for_reward_three = is_reward_three_initialized
                && !is_reward_three_fully_distributed_until_this_snapshot;

            if snapshot_is_required_for_reward_one
                || snapshot_is_required_for_reward_two
                || snapshot_is_required_for_reward_three
            {
                break;
            }

            self.snapshots.pop_front();
        }
    }
}
