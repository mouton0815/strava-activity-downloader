use serde::{Deserialize, Serialize};
use crate::domain::activity_stats::ActivityStats;
use crate::state::shared_state::SchedulerState;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ServerStatus {
    authorized: bool,
    scheduler_state: SchedulerState,
    activity_stats: ActivityStats
}

impl ServerStatus {
    pub fn new(authorized: bool, scheduler_state: SchedulerState, activity_stats: ActivityStats) -> Self {
        Self { authorized, scheduler_state, activity_stats }
    }
}