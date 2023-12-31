use log::{debug, info, warn};
use std::time::Duration;
use axum::BoxError;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::{Bearer, MutexSharedState, SharedState};

async fn task(_state: &mut SharedState, bearer: Bearer) -> Result<(), BoxError> {
    let bearer : String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);
    Ok(())
}

async fn authorize(state: MutexSharedState) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    if (*guard).scheduler_running {
        match (*guard).oauth.get_bearer().await? {
            Some(bearer) => {
                task(&mut *guard, bearer).await?;
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
                if let Err(e) = authorize(state.clone()).await {
                    warn!("Task failed: {:?}, leave scheduler", e);
                    break;
                }
            },
            _ = rx.recv() => {
                info!("Termination signal received, leave scheduler");
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
