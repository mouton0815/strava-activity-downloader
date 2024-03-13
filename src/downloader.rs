use log::{debug, info, warn};
use std::time::Duration;
use axum::BoxError;
use reqwest::{Error, Response};
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::{ActivityStream, Bearer};
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::download_state::DownloadState;
use crate::state::shared_state::MutexSharedState;

// TODO: Configure URLs from outside?
// const BASE_URL : &'static str = "https://www.strava.com/api/v3";
const BASE_URL : &'static str = "http://localhost:5555";

async fn get_download_state(state: &MutexSharedState) -> DownloadState {
    let guard = state.lock().await;
    (*guard).download_state.clone()
}

async fn set_download_state(state: &MutexSharedState, download_state: DownloadState) {
    let mut guard = state.lock().await;
    (*guard).download_state = download_state;
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
        let status = (*guard).get_server_status().await?;
        (*guard).sender.send(status)?;
    }
    Ok(())
}

async fn get_earliest_activity_without_gpx(state: &MutexSharedState) -> Result<Option<Activity>, BoxError> {
    let mut guard = state.lock().await;
    (*guard).service.get_earliest_without_gpx()
}

async fn store_gpx(state: &MutexSharedState, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    (*guard).service.store_gpx(activity, stream)?;
    (*guard).merge_activity_stats(&ActivityStats::new(0, 1, None, None));
    Ok(())
}

/// Downloads activities from Strava and stores them in the database
async fn activity_task(state: &MutexSharedState, bearer: String) -> Result<bool, BoxError> {
    let (max_time, per_page) = get_query_params(state).await?;
    let query = vec![("after", max_time),("per_page", per_page)];

    let response = reqwest::Client::new()
        .get(format!("{BASE_URL}/athlete/activities"))
        .header(reqwest::header::AUTHORIZATION, bearer)
        .query(&query)
        .send().await?
        .error_for_status();

    if response.as_ref().is_err_and(|e| e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)) {
        return Ok(true)
    }

    let activities= response?.json::<ActivityVec>().await?;
    if activities.len() == 0 {
        info!("No further activities, start downloading activity streams from oldest to youngest");
        set_download_state(state, DownloadState::Tracks).await;
    } else {
        add_activities(&state, &activities).await?;
    }

    Ok(false)
}

/// Downloads an activity stream from Strava, transforms it to GPX, and stores it as file
async fn stream_task(state: &MutexSharedState, bearer: String) -> Result<bool, BoxError> {
    match get_earliest_activity_without_gpx(state).await? {
        Some(activity) => {
            let url = format!("{BASE_URL}/activities/{}/streams?keys=time,latlng,altitude&key_by_type=true", activity.id);
            let response = reqwest::Client::new()
                .get(&url)
                .header(reqwest::header::AUTHORIZATION, bearer)
                .send().await?
                .error_for_status();

            if response.as_ref().is_err_and(|e| e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)) {
                return Ok(true)
            }

            let stream = response?.json::<ActivityStream>().await?;
            store_gpx(state, &activity, &stream).await?;
        }
        None => {
            info!("No further activities without GPX, stop downloading (can be re-enabled)");
            set_download_state(state, DownloadState::Inactive).await;
        }
    }
    Ok(false)
}

async fn try_task(state: &MutexSharedState) -> Result<(), BoxError> {
    let download_state = get_download_state(state).await;
    if download_state == DownloadState::Inactive {
        debug!("Download disabled, skip task execution");
    } else {
        match get_bearer(&state).await? {
            Some(bearer) => {
                let limit_reached = match download_state {
                    DownloadState::Activities => activity_task(state, bearer.into()).await?,
                    DownloadState::Tracks => stream_task(state, bearer.into()).await?,
                    DownloadState::Inactive => false // Match arm cannot be reached
                };
                if limit_reached {
                    warn!("Strava API limits reached, stop downloading (can be re-enabled)");
                    set_download_state(state, DownloadState::Inactive).await;
                }
                // In all cases send a status event to update the frontend
                send_status_event(state).await?;
            }
            None => {
                // This should not happen because the REST API allows enabling the downloader only if
                // authenticated. There is no way for the downloader to do an OAuth auth code flow.
                warn!("Not authorized, skip execution of download task");
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
                    warn!("Task failed: {:?}, leave downloader", e);
                    break;
                }
            },
            _ = rx.recv() => {
                debug!("Termination signal received, leave downloader");
                break;
            }
        }
    }
}

pub fn spawn_download_scheduler(state: MutexSharedState, rx: Receiver<()>, period: Duration) -> JoinHandle<()> {
    info!("Spawn download scheduler");
    tokio::spawn(async move {
        repeat(state, period, rx).await;
    })
}
