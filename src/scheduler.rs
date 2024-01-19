use log::{debug, info, warn};
use std::time::Duration;
use axum::BoxError;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::Bearer;
use crate::domain::activity::ActivityVec;
use crate::state::shared_state::MutexSharedState;

async fn is_enabled(state: &MutexSharedState) -> bool {
    let guard = state.lock().await;
    (*guard).scheduler_running.clone()
}

async fn stop_running(state: &MutexSharedState) {
    let mut guard = state.lock().await;
    (*guard).scheduler_running = false
}

async fn get_bearer(state: &MutexSharedState) -> Result<Option<Bearer>, BoxError> {
    let mut guard = state.lock().await;
    (*guard).oauth.get_bearer().await
}

async fn get_query_params(state: &MutexSharedState) -> Result<(i64, i64), BoxError> {
    let mut guard = state.lock().await;
    let max_time = (*guard).get_max_time().await?;
    let per_page = (*guard).activities_per_page.clone() as i64;
    Ok((max_time, per_page))
}

async fn add_activities(state: &MutexSharedState, activities: &ActivityVec) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    let activity_stats = (*guard).service.add(activities)?;
    (*guard).merge_activity_stats(&activity_stats);
    Ok(())
}

async fn send_status_event(state: &MutexSharedState) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    if (*guard).sender.receiver_count() > 0 {
        let server_status = (*guard).get_server_status().await?;
        let server_status = serde_json::to_string(&server_status)?;
        (*guard).sender.send(server_status)?;
    }
    Ok(())
}

async fn task(state: &MutexSharedState, bearer: String) -> Result<(), BoxError> {
    let (max_time, per_page) = get_query_params(state).await?;
    let query = vec![("after", max_time),("per_page", per_page)];

    let activities : ActivityVec = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete/activities")
        .header(reqwest::header::AUTHORIZATION, bearer)
        .query(&query)
        .send().await?
        .error_for_status()?
        .json::<ActivityVec>().await?;

    if activities.len() == 0 {
        info!("No further activities, stop executing tasks (can be re-enabled)");
        stop_running(state).await;
    } else {
        add_activities(&state, &activities).await?;
    }

    // In all cases send a status event, e.g. to inform the scheduler disabling
    send_status_event(state).await?;
    Ok(())
}

async fn try_task(state: &MutexSharedState) -> Result<(), BoxError> {
    if is_enabled(&state).await {
        match get_bearer(&state).await? {
            Some(bearer) => {
                task(state, bearer.into()).await?;
            }
            None => {
                // This should not happen because the REST API allows enabling the scheduler only if
                // authenticated. There is no way for the scheduler to do an OAuth auth code flow.
                warn!("Not authorized, skip task execution");
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
                if let Err(e) = try_task(&state).await {
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
