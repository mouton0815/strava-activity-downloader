use std::sync::Arc;
use axum::BoxError;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use crate::domain::activity_stats::ActivityStats;
use crate::domain::download_state::DownloadState;
use crate::domain::server_status::ServerStatus;
use crate::oauth::oauth_client::OAuthClient;
use crate::service::activity_service::ActivityService;
use crate::track::track_storage::TrackStorage;

/// State shared between axum handlers and downloader
pub struct SharedState {
    pub oauth: OAuthClient,
    pub service: ActivityService,
    pub tracks: TrackStorage,
    pub tx_data: Sender<ServerStatus>, // Broadcast sender used by the downloader to inform the SSE endpoint
    pub tx_term: Sender<()>,  // Broadcast sender used by the SSE handlers to inform about server termination
    pub activity_stats: Option<ActivityStats>, // Holds last version of DB activity stats
    pub download_state: DownloadState,
    pub activities_per_page: u16
}

pub type MutexSharedState = Arc<Mutex<SharedState>>;

impl SharedState {
    pub fn new(oauth: OAuthClient,
               service: ActivityService,
               tracks: TrackStorage,
               tx_data: Sender<ServerStatus>,
               tx_term: Sender<()>,
               activities_per_page: u16) -> MutexSharedState {
        Arc::new(Mutex::new(Self {
            oauth,
            service,
            tracks,
            tx_data,
            tx_term,
            activity_stats: None,
            download_state: DownloadState::Inactive,
            activities_per_page
        }))
    }

    /// Merge only if the activity stats are already loaded. There is no need of merging
    /// otherwise, as the stats will be loaded later, and will then contain other_stats.
    pub fn merge_activity_stats(&mut self, other_stats: &ActivityStats) {
        if let Some(stats) = self.activity_stats.as_mut() {
            stats.merge(other_stats);
        }
    }

    /// Try to take the time from the state object.
    /// If it is not part of the state (on server startup), then get it from database.
    /// If no activity records exist in the database, then return 0.
    pub async fn get_activity_max_time(&mut self) -> Result<i64, BoxError> {
        let activity_stats = self.get_activity_stats().await?;
        match activity_stats.act_max_time_as_secs() {
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

    /// Returns the [ActivityStats], either from the cached value or else from the wrapped service.
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

#[cfg(test)]
mod tests {
    use tokio::sync::broadcast;
    use crate::domain::activity::Activity;
    use crate::domain::activity_stats::ActivityStats;
    use crate::domain::server_status::ServerStatus;
    use crate::oauth::oauth_client::OAuthClient;
    use crate::service::activity_service::ActivityService;
    use crate::state::shared_state::{MutexSharedState, SharedState};
    use crate::track::track_storage::TrackStorage;

    impl SharedState {
        pub fn dummy(service: ActivityService) -> MutexSharedState {
            let client = OAuthClient::dummy();
            let tracks = TrackStorage::new("");
            let (tx_data, _) = broadcast::channel::<ServerStatus>(1);
            let (tx_term, _) = broadcast::channel(1);
            SharedState::new(client, service, tracks, tx_data, tx_term, 0)
        }
    }

    #[tokio::test]
    async fn test_activity_max_time() {
        let activities = vec![Activity::dummy(5, "2018-02-20T18:02:13Z")];
        let mut service = ActivityService::new(":memory:", true).unwrap();
        service.add(&activities).unwrap();

        let state = SharedState::dummy(service);

        let mut guard = state.lock().await;
        let max_time = guard.get_activity_max_time().await;
        assert!(max_time.is_ok());
        assert_eq!(max_time.unwrap(), 1519149733);
    }

    #[tokio::test]
    async fn test_merge_stats() {
        let activities = vec![Activity::dummy(5, "2015-01-01T00:00:00Z")];
        let mut service = ActivityService::new(":memory:", true).unwrap();
        service.add(&activities).unwrap();

        let state = SharedState::dummy(service);

        let mut guard = state.lock().await;
        let new_stats = ActivityStats::new(1, None, Some("2018-02-20T18:02:13Z".to_string()), 0,  None);
        assert!(guard.get_activity_stats().await.is_ok()); // Force loading stats from service
        guard.merge_activity_stats(&new_stats);

        let max_time = guard.get_activity_max_time().await;
        assert!(max_time.is_ok());
        assert_eq!(max_time.unwrap(), 1519149733);
    }

    #[tokio::test]
    async fn test_server_status() {
        let expected = r#"{"authorized":false,"download_state":"Inactive","activity_stats":{"act_count":2,"act_min_time":"2018-02-20T18:02:13Z","act_max_time":"2020-08-21T00:00:00Z","trk_count":0,"trk_max_time":null}}"#;

        let activities = vec![
            Activity::dummy(5, "2018-02-20T18:02:13Z"),
            Activity::dummy(7, "2020-08-21T00:00:00Z")
        ];
        let mut service = ActivityService::new(":memory:", true).unwrap();
        service.add(&activities).unwrap();

        let state = SharedState::dummy(service);

        let mut guard = state.lock().await;
        let status = guard.get_server_status().await;
        assert!(status.is_ok());
        let result = serde_json::to_string::<ServerStatus>(&status.unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }
}
