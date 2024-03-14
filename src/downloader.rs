use log::{debug, info, warn};
use std::time::Duration;
use axum::BoxError;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::{ActivityStream, Bearer};
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::download_state::DownloadState;
use crate::state::shared_state::MutexSharedState;

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

async fn mark_gpx(state: &MutexSharedState, activity: &Activity) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    (*guard).service.mark_gpx(activity)?;
    (*guard).merge_activity_stats(&ActivityStats::new(0, 1, None, None));
    Ok(())
}

/// Downloads activities from Strava and stores them in the database
async fn activity_task(state: &MutexSharedState, strava_url: &str, bearer: String) -> Result<(), BoxError> {
    let (max_time, per_page) = get_query_params(state).await?;
    let query = vec![("after", max_time),("per_page", per_page)];

    let response = reqwest::Client::new()
        .get(format!("{strava_url}/athlete/activities"))
        .header(reqwest::header::AUTHORIZATION, bearer)
        .query(&query)
        .send().await?
        .error_for_status();

    if response.as_ref().is_err_and(|e| e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)) {
        warn!("Strava API limits reached, stop downloading (can be re-enabled)");
        set_download_state(state, DownloadState::LimitReached).await;
        return Ok(())
    }

    let activities= response?.json::<ActivityVec>().await?;
    if activities.len() == 0 {
        info!("No further activities, start downloading activity streams from oldest to youngest");
        set_download_state(state, DownloadState::Tracks).await;
    } else {
        add_activities(&state, &activities).await?;
    }

    Ok(())
}

/// Downloads an activity stream from Strava, transforms it to GPX, and stores it as file
async fn stream_task(state: &MutexSharedState, strava_url: &str, bearer: String) -> Result<(), BoxError> {
    match get_earliest_activity_without_gpx(state).await? {
        Some(activity) => {
            let url = format!("{strava_url}/activities/{}/streams?keys=time,latlng,altitude&key_by_type=true", activity.id);
            let response = reqwest::Client::new()
                .get(&url)
                .header(reqwest::header::AUTHORIZATION, bearer)
                .send().await?
                .error_for_status();

            if let Err(error) = response.as_ref() {
                if error.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                    warn!("Activity {} has no track", activity.id);
                    mark_gpx(state, &activity).await?;
                    return Ok(())
                }
                if error.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
                    warn!("Strava API limits reached, stop downloading (can be re-enabled)");
                    set_download_state(state, DownloadState::LimitReached).await;
                    return Ok(())
                }
            }

            let stream = response?.json::<ActivityStream>().await?;
            store_gpx(state, &activity, &stream).await?;
        }
        None => {
            info!("No further activities without GPX, stop downloading (can be re-enabled)");
            set_download_state(state, DownloadState::NoResults).await;
        }
    }
    Ok(())
}

async fn try_task(state: &MutexSharedState, strava_url: &str) -> Result<(), BoxError> {
    let download_state = get_download_state(state).await;
    if download_state.is_active() {
        match get_bearer(&state).await? {
            Some(bearer) => {
                if download_state == DownloadState::Activities {
                    activity_task(state, strava_url, bearer.into()).await?;
                } else if download_state == DownloadState::Tracks {
                    stream_task(state, strava_url, bearer.into()).await?;
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
    } else {
        debug!("Download disabled, skip task execution");
    }
    Ok(())
}

// Must be async as required by tokio::select!
async fn repeat(state: MutexSharedState, strava_url: &str, period: Duration, mut rx: Receiver<()>) {
    let mut interval = time::interval(period);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = try_task(&state, strava_url).await {
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

pub fn spawn_download_scheduler(state: MutexSharedState, rx: Receiver<()>, strava_url: String, period: Duration) -> JoinHandle<()> {
    info!("Spawn download scheduler");
    tokio::spawn(async move {
        repeat(state, &strava_url, period, rx).await;
    })
}
