use log::{debug, info, warn};
use std::time::Duration;
use axum::BoxError;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::{ActivityStream, Bearer};
use crate::domain::activity::{Activity, ActivityVec};
use crate::state::shared_state::{MutexSharedState, SchedulerState};

const BASE_URL : &'static str = "https://www.strava.com/api/v3";

async fn get_scheduler_state(state: &MutexSharedState) -> SchedulerState {
    let guard = state.lock().await;
    (*guard).scheduler_state.clone()
}

async fn set_scheduler_state(state: &MutexSharedState, scheduler_state: SchedulerState) {
    let mut guard = state.lock().await;
    (*guard).scheduler_state = scheduler_state;
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

async fn get_earliest_activity_without_gpx(state: &MutexSharedState) -> Result<Option<Activity>, BoxError> {
    let mut guard = state.lock().await;
    (*guard).service.get_earliest_without_gpx()
}

async fn store_gpx(state: &MutexSharedState, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    (*guard).service.store_gpx(activity, stream)
}


/// Downloads activities from Strava and stores them in the database
#[allow(dead_code)]
async fn activity_task(state: &MutexSharedState, bearer: String) -> Result<(), BoxError> {
    let (max_time, per_page) = get_query_params(state).await?;
    let query = vec![("after", max_time),("per_page", per_page)];

    let activities : ActivityVec = reqwest::Client::new()
        .get(format!("{BASE_URL}/athlete/activities"))
        .header(reqwest::header::AUTHORIZATION, bearer)
        .query(&query)
        .send().await?
        .error_for_status()?
        .json::<ActivityVec>().await?;

    if activities.len() == 0 {
        info!("No further activities, start downloading activity streams from oldest to youngest");
        set_scheduler_state(state, SchedulerState::DownloadStreams).await;
    } else {
        add_activities(&state, &activities).await?;
    }

    Ok(())
}

/// Downloads an activity stream from Strava, transforms it to GPX, and stores it as file
async fn stream_task(state: &MutexSharedState, bearer: String) -> Result<(), BoxError> {
    match get_earliest_activity_without_gpx(state).await? {
        Some(activity) => {
            let url = format!("{BASE_URL}/activities/{}/streams?keys=time,latlng,altitude&key_by_type=true", activity.id);
            let stream : ActivityStream = reqwest::Client::new()
                .get(&url)
                .header(reqwest::header::AUTHORIZATION, bearer)
                .send().await?
                .error_for_status()?
                .json::<ActivityStream>().await?;

            store_gpx(state, &activity, &stream).await?;
        }
        None => {
            info!("No further activities without GPX, stop executing tasks (can be re-enabled)");
            set_scheduler_state(state, SchedulerState::Inactive).await;
        }
    }
    Ok(())
}

async fn try_task(state: &MutexSharedState) -> Result<(), BoxError> {
    let scheduler_state = get_scheduler_state(state).await;
    if scheduler_state == SchedulerState::Inactive {
        debug!("Scheduler disabled, skip task execution");
    } else {
        match get_bearer(&state).await? {
            Some(bearer) => {
                if scheduler_state == SchedulerState::DownloadActivities {
                    activity_task(state, bearer.into()).await?;
                } else {
                    stream_task(state, bearer.into()).await?;
                }
                // In all cases send a status event to update the frontend
                send_status_event(state).await?;
            }
            None => {
                // This should not happen because the REST API allows enabling the scheduler only if
                // authenticated. There is no way for the scheduler to do an OAuth auth code flow.
                warn!("Not authorized, skip task execution");
            }
        }
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
