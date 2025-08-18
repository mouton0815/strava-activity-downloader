use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum DownloadDelay {
    Long, // Long delay between to activity or track downloads from Strava
    Short // Short delay between all other actions
}
