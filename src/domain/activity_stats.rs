use serde::{Deserialize, Serialize};
use crate::util::iso8601;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ActivityStats {
    count: u32,
    min_time: Option<String>,
    max_time: Option<String>
}

impl ActivityStats {
    pub fn new(count: u32, min_time: Option<String>, max_time: Option<String>) -> Self {
        Self { count, min_time, max_time }
    }

    pub fn max_time_as_secs(&self) -> Option<i64> {
        self.max_time.as_ref().map(iso8601::string_to_secs)
    }
}
