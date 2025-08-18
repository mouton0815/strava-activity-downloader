use serde::Serialize;
use crate::domain::activity_stats::ActivityStats;
use crate::domain::download_state::DownloadState;

/// Object passed from downloader to SSE handler and result returned by the /status endpoint
#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct ServerStatus {
    authorized: bool,
    download_state: DownloadState,
    activity_stats: ActivityStats
}

impl ServerStatus {
    pub fn new(authorized: bool, download_state: DownloadState, activity_stats: ActivityStats) -> Self {
        Self { authorized, download_state, activity_stats }
    }
}