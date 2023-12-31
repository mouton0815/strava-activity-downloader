use std::sync::Arc;
use tokio::sync::Mutex;
use crate::OAuthClient;

pub struct SharedState {
    pub oauth: OAuthClient,
    pub scheduler_running: bool // TODO: Make "pub" private and use functions instead?
}

pub type MutexSharedState = Arc<Mutex<SharedState>>;

impl SharedState {
    pub fn new(oauth: OAuthClient) -> MutexSharedState {
        Arc::new(Mutex::new(Self { oauth, scheduler_running: false }))
    }
}
