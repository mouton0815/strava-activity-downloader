use std::sync::Arc;
use tokio::sync::Mutex;
use crate::activity_service::ActivityService;
use crate::OAuthClient;

pub struct SharedState {
    pub oauth: OAuthClient,
    pub service: ActivityService,
    pub scheduler_running: bool, // TODO: Make "pub" private and use functions instead?
    pub min_activity_time: Option<i64>
}

pub type MutexSharedState = Arc<Mutex<SharedState>>;

impl SharedState {
    pub fn new(oauth: OAuthClient, service: ActivityService) -> MutexSharedState {
        Arc::new(Mutex::new(Self { oauth, service, scheduler_running: false, min_activity_time: None }))
    }
}
