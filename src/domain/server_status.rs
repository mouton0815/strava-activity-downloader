use serde::{Deserialize, Serialize};
use crate::domain::activity_stats::ActivityStats;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ServerStatus {
    authorized: bool,
    scheduling: bool,
    activity_stats: ActivityStats
}

impl ServerStatus {
    pub fn new(authorized: bool, scheduling: bool, activity_stats: ActivityStats) -> Self {
        Self { authorized, scheduling, activity_stats }
    }
}