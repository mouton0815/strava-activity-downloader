use serde::{Deserialize, Serialize};
use std::sync::Arc;
use axum::BoxError;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use crate::domain::activity_stats::ActivityStats;
use crate::domain::server_status::ServerStatus;
use crate::OAuthClient;
use crate::service::activity_service::ActivityService;

// TODO: Unit tests!

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum SchedulerState {
    Inactive,
    DownloadActivities,
    DownloadStreams
}

/// State shared between axum handlers and scheduler
pub struct SharedState {
    pub oauth: OAuthClient,
    pub service: ActivityService,
    pub sender: Sender<String>, // Broadcast sender used by the scheduler to inform the SSE endpoint
    pub activity_stats: Option<ActivityStats>, // Holds last version of DB activity stats
    pub scheduler_state: SchedulerState,
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
            scheduler_state: SchedulerState::Inactive,
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
        let scheduler_state = self.scheduler_state.clone();
        let activity_stats = self.get_activity_stats().await?;
        Ok(ServerStatus::new(authorized, scheduler_state, activity_stats))
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
