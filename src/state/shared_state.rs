use std::sync::Arc;
use axum::BoxError;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use crate::domain::activity_stats::ActivityStats;
use crate::domain::download_state::DownloadState;
use crate::domain::server_status::ServerStatus;
use crate::OAuthClient;
use crate::service::activity_service::ActivityService;

// TODO: Unit tests!

/// State shared between axum handlers and downloader
pub struct SharedState {
    pub oauth: OAuthClient,
    pub service: ActivityService,
    pub sender: Sender<String>, // Broadcast sender used by the downloader to inform the SSE endpoint
    pub activity_stats: Option<ActivityStats>, // Holds last version of DB activity stats
    pub download_state: DownloadState,
    pub activities_per_page: u16
}

pub type MutexSharedState = Arc<Mutex<SharedState>>;

impl SharedState {
    pub fn new(oauth: OAuthClient,
               service: ActivityService,
               sender: Sender<String>,
               activities_per_page: u16) -> MutexSharedState {
        Arc::new(Mutex::new(Self {
            oauth,
            service,
            sender,
            activity_stats: None,
            download_state: DownloadState::Inactive,
            activities_per_page
        }))
    }

    /// Merge only if the activity stats are already loaded. There is no need of merging
    /// otherwise, as the the stats will be loaded later, and will then contain other_stats.
    pub fn merge_activity_stats(&mut self, other_stats: &ActivityStats) {
        if let Some(stats) = self.activity_stats.as_mut() {
            stats.merge(other_stats);
        }
    }

    /// Try to take the time from the state object.
    /// If it is not part of the state (on server startup), then get it from database.
    /// If no activity records exist in the database, then return 0.
    pub async fn get_max_time(&mut self) -> Result<i64, BoxError> {
        let activity_stats = self.get_activity_stats().await?;
        match activity_stats.max_time_as_secs() {
            Some(secs) => Ok(secs),
            None => Ok(0)
        }
    }

    /// Returns a [ServerStatus] object. Takes the included [ActivityStats]
    /// from the [SharedState] or fetches them from database.
    pub async fn get_server_status(&mut self) -> Result<ServerStatus, BoxError> {
        let authorized = self.oauth.get_bearer().await?.is_some();
        let download_state = self.download_state.clone();
        let activity_stats = self.get_activity_stats().await?;
        Ok(ServerStatus::new(authorized, download_state, activity_stats))
    }

    async fn get_activity_stats(&mut self) -> Result<ActivityStats, BoxError> {
        match self.activity_stats.as_ref() {
            Some(stats) => Ok(stats.clone()),
            None => {
                let activity_stats = self.service.get_stats()?;
                self.activity_stats = Some(activity_stats.clone());
                Ok(activity_stats)
            }
        }
    }
}
