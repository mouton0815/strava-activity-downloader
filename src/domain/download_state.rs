use serde::{Deserialize, Serialize};
use crate::domain::download_delay::DownloadDelay;

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum DownloadState {
    Inactive,     // Downloading was not started or manually stopped
    NoResults,    // Last Strava API request returned no results
    LimitReached, // Strava API rate limit was reached
    RequestError, // An error returned by the Strava API
    Activities,   // Activity download ongoing
    Tracks        // Track (=activity stream) download ongoing
}

impl DownloadState {
    pub fn is_active(&self) -> bool {
        match self {
            DownloadState::Inactive => false,
            DownloadState::NoResults => false,
            DownloadState::LimitReached => false,
            DownloadState::RequestError => false,
            DownloadState::Activities => true,
            DownloadState::Tracks => true
        }
    }

    /// Manual toggling
    pub fn toggle(&self) -> Self {
        match self {
            DownloadState::Inactive => DownloadState::Activities,
            DownloadState::NoResults => DownloadState::Activities,
            DownloadState::LimitReached => DownloadState::Activities,
            DownloadState::RequestError => DownloadState::Activities,
            DownloadState::Activities => DownloadState::Inactive,
            DownloadState::Tracks => DownloadState::Inactive
        }
    }

    pub fn new_delay(&self, new_state: &DownloadState) -> DownloadDelay {
        match (*new_state == DownloadState::Activities || *new_state == DownloadState::Tracks) && new_state == self {
            true => DownloadDelay::Long,
            false => DownloadDelay::Short
        }
    }
}
