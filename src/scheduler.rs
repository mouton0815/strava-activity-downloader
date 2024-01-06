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

// Try to take the time from the state object.
// If it is not part of the state (on server startup), then get it from database.
// If no activity records exist in the database, then return 0.
async fn get_max_time(state: &MutexSharedState) -> Result<i64, BoxError> {
    let mut guard = state.lock().await;
    match (*guard).max_activity_time.as_ref() {
        Some(time) => Ok(time.clone()),
        None => {
            match (*guard).service.get_max_start_time()? {
                Some(time) => Ok(time),
                None => Ok(0)
            }
        }
    }
}

async fn get_activities_per_page(state: &MutexSharedState) -> u16 {
    let mut guard = state.lock().await;
    (*guard).activities_per_page.clone()
}

async fn add_activities(state: &MutexSharedState, activities: &ActivityVec) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    let max_time = (*guard).service.add(activities)?;
    (*guard).max_activity_time = max_time;
    Ok(())
}

async fn task(state: &MutexSharedState, bearer: String) -> Result<(), BoxError> {
    let after = get_max_time(state).await?;
    let per_page = get_activities_per_page(state).await as i64;
    let query = vec![("after", after),("per_page", per_page)];

    let activities : ActivityVec = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete/activities")
        .header(reqwest::header::AUTHORIZATION, bearer)
        .query(&query)
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
