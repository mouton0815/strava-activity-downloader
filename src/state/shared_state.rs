use std::sync::Arc;
use tokio::sync::Mutex;
use crate::OAuthClient;
use crate::service::activity_service::ActivityService;

pub struct SharedState {
    pub oauth: OAuthClient,
    pub service: ActivityService,
    pub scheduler_running: bool,
    pub max_activity_time: Option<i64>,
    pub activities_per_page: u16
}

pub type MutexSharedState = Arc<Mutex<SharedState>>;

impl SharedState {
    pub fn new(oauth: OAuthClient, service: ActivityService, activities_per_page: u16) -> MutexSharedState {
        Arc::new(Mutex::new(Self {oauth, service, scheduler_running: false, max_activity_time: None, activities_per_page }))
    }
}
