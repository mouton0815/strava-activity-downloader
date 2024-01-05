use log::{debug, info, warn};
use std::time::Duration;
use axum::BoxError;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::Bearer;
use crate::domain::activity::ActivityVec;
use crate::state::shared_state::MutexSharedState;

async fn is_running(state: &MutexSharedState) -> bool {
    let guard = state.lock().await;
    (*guard).scheduler_running.clone()
}

async fn get_bearer(state: &MutexSharedState) -> Result<Option<Bearer>, BoxError> {
    let mut guard = state.lock().await;
    (*guard).oauth.get_bearer().await
}

async fn get_min_time(state: &MutexSharedState) -> Result<Option<i64>, BoxError> {
    let mut guard = state.lock().await;
    match (*guard).min_activity_time.as_ref() {
        Some(time) => Ok(Some(time.clone())),
        None => (*guard).service.get_min_start_time()
    }
}

async fn add_activities(state: &MutexSharedState, activities: &ActivityVec) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    let min_time = (*guard).service.add(activities)?;
    (*guard).min_activity_time = min_time;
    Ok(())
}

async fn task(state: &MutexSharedState, bearer: String) -> Result<(), BoxError> {
    let mut client = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete/activities")
        .header(reqwest::header::AUTHORIZATION, bearer);

    if let Some(before) = get_min_time(state).await? {
        let query = vec![("before", before)];
        client = client.query(&query);
    }

    let activities : ActivityVec = client
        .send().await?
        .error_for_status()?
        .json::<ActivityVec>().await?;

    debug!("--r--> {:?}", activities);
    add_activities(&state, &activities).await?;
    Ok(())
}

async fn authorize(state: &MutexSharedState) -> Result<(), BoxError> {
    if is_running(&state).await {
        match get_bearer(&state).await? {
            Some(bearer) => {
                // TODO: Remove next two lines
                let bearer: String = bearer.into();
                debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);
                task(state, bearer.into()).await?;
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
