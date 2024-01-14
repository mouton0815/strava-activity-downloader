use std::sync::Arc;
use axum::BoxError;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use crate::domain::server_status::ServerStatus;
use crate::OAuthClient;
use crate::service::activity_service::ActivityService;

/// State shared between axum handlers and scheduler
pub struct SharedState {
    pub oauth: OAuthClient,
    pub service: ActivityService,
    pub sender: Sender<String>,
    pub scheduler_running: bool,
    pub max_activity_time: Option<i64>,
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
            scheduler_running: false,
            max_activity_time: None,
            activities_per_page
        }))
    }

    pub async fn get_server_status(&mut self) -> Result<ServerStatus, BoxError> {
        let authorized = self.oauth.get_bearer().await?.is_some();
        let scheduling = self.scheduler_running.clone();
        let activity_stats = self.service.get_stats()?;
        Ok(ServerStatus::new(authorized, scheduling, activity_stats))
    }
}
