use log::{debug, info, warn};
use std::time::Duration;
use axum::BoxError;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::{Bearer, SharedState};
use crate::state::shared_state::MutexSharedState;

async fn is_running(state: &MutexSharedState) -> bool {
    let guard = state.lock().await;
    (*guard).scheduler_running.clone()
}

async fn get_bearer(state: &MutexSharedState) -> Result<Option<Bearer>, BoxError> {
    let mut guard = state.lock().await;
    (*guard).oauth.get_bearer().await
}

async fn task(_state: &MutexSharedState, bearer: Bearer) -> Result<(), BoxError> {
    let bearer: String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);
    Ok(())
}

async fn authorize(state: &MutexSharedState) -> Result<(), BoxError> {
    if is_running(&state).await {
        match get_bearer(&state).await? {
            Some(bearer) => {
                task(state, bearer).await?;
            }
            None => {
                // There is no way for the scheduler to do an OAuth auth code flow.
                debug!("Not authorized yet, skip task execution");
            }
        }
    } else {
        debug!("Scheduler disabled, skip task execution");
    }
    Ok(())
}

// Must be async as required by tokio::select!
async fn repeat(state: MutexSharedState, period: Duration, mut rx: Receiver<()>) {
    let mut interval = time::interval(period);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = authorize(&state).await {
                    warn!("Task failed: {:?}, leave scheduler", e);
                    break;
                }
            },
            _ = rx.recv() => {
                debug!("Termination signal received, leave scheduler");
                break;
            }
        }
    }
}

pub fn spawn_scheduler(state: MutexSharedState, rx: Receiver<()>, period: Duration) -> JoinHandle<()> {
    info!("Spawn scheduler");
    tokio::spawn(async move {
        repeat(state, period, rx).await;
    })
}
