use log::{debug, info, trace, warn};
use std::time::Duration;
use axum::BoxError;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::activity_stream::ActivityStream;
use crate::domain::download_delay::DownloadDelay;
use crate::domain::download_state::DownloadState;
use crate::domain::track_store_state::TrackStoreState;
use crate::oauth::token::Bearer;
use crate::state::shared_state::MutexSharedState;

async fn get_download_state(state: &MutexSharedState) -> DownloadState {
    let guard = state.lock().await;
    guard.download_state.clone()
}

async fn set_download_state(state: &MutexSharedState, download_state: DownloadState) {
    let mut guard = state.lock().await;
    guard.download_state = download_state;
}

async fn get_bearer(state: &MutexSharedState) -> Result<Option<Bearer>, BoxError> {
    let mut guard = state.lock().await;
    guard.oauth.get_bearer().await
}

async fn get_query_params(state: &MutexSharedState) -> Result<(i64, i64), BoxError> {
    let mut guard = state.lock().await;
    let max_time = guard.get_activity_max_time().await?;
    let per_page = guard.activities_per_page as i64;
    Ok((max_time, per_page))
}

async fn add_activities(state: &MutexSharedState, activities: &ActivityVec) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    let activity_stats = guard.service.add(activities).await?;
    guard.merge_activity_stats(&activity_stats);
    Ok(())
}

async fn send_status_event(state: &MutexSharedState) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    if guard.tx_data.receiver_count() > 0 {
        let status = guard.get_server_status().await?;
        guard.tx_data.send(status)?;
    }
    Ok(())
}

async fn get_earliest_activity_without_track(state: &MutexSharedState) -> Result<Option<Activity>, BoxError> {
    let mut guard = state.lock().await;
    guard.service.get_earliest_without_track().await
}

async fn store_track(state: &MutexSharedState, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    // Write the GPX file ...
    guard.tracks.write(activity, stream)?;
    // ... then mark the fetch status of the corresponding activity
    guard.service.mark_fetched(activity, TrackStoreState::Stored).await?;
    // ... next (and optionally) compute the tiles and store them
    guard.service.store_tiles(activity, stream).await?;
    // ... finally increase the in-memory stats to be sent to the UI
    guard.merge_activity_stats(&ActivityStats::new(0, None, None, 1, Some(activity.start_date.clone())));
    Ok(())
}

async fn mark_track_missing(state: &MutexSharedState, activity: &Activity) -> Result<(), BoxError> {
    let mut guard = state.lock().await;
    guard.service.mark_fetched(activity, TrackStoreState::Missing).await?;
    Ok(())
}

type TaskResult = Result<DownloadState, BoxError>;

/// Downloads activities from Strava and stores them in the database
async fn activity_task(state: &MutexSharedState, strava_url: &str, bearer: String) -> TaskResult {
    let (max_time, per_page) = get_query_params(state).await?;
    let query = vec![("after", max_time),("per_page", per_page)];

    let response = reqwest::Client::new()
        .get(format!("{strava_url}/athlete/activities"))
        .header(reqwest::header::AUTHORIZATION, bearer)
        .query(&query)
        .send().await?
        .error_for_status();

    if let Err(error) = response.as_ref() {
        if error.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
            warn!("Strava API limits reached, stop downloading (can be re-enabled)");
            return Ok(DownloadState::LimitReached)
        }
        warn!("Strava activities API returned status {:?}, stop downloading", error.status());
        return Ok(DownloadState::RequestError)
    }

    let activities= response?.json::<ActivityVec>().await?;
    if activities.is_empty() {
        info!("No further activities, start downloading activity streams from oldest to youngest");
        return Ok(DownloadState::Tracks)
    }

    add_activities(state, &activities).await?;
    Ok(DownloadState::Activities)
}

/// Downloads an activity stream from Strava, transforms it to a GPX track, and stores it as file
async fn stream_task(state: &MutexSharedState, strava_url: &str, bearer: String) -> TaskResult {
    match get_earliest_activity_without_track(state).await? {
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
                    mark_track_missing(state, &activity).await?;
                    return Ok(DownloadState::Tracks) // Downloading continues
                }
                if error.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) {
                    warn!("Strava API limits reached, stop downloading (can be re-enabled)");
                    return Ok(DownloadState::LimitReached)
                }
                warn!("Strava streams API returned status {:?}, stop downloading", error.status());
                return Ok(DownloadState::RequestError)
            }
            // match response?.text().await {
            match response?.json::<ActivityStream>().await {
                Ok(stream) => {
                    // info!("{:?}", stream);
                    store_track(state, &activity, &stream).await?;
                    Ok(DownloadState::Tracks)
                }
                Err(error) => {
                    // A known case is that the activity stream does not contain a "latlon" array
                    warn!("Failed to parse the track of activity {}: {}", activity.id, error);
                    mark_track_missing(state, &activity).await?;
                    Ok(DownloadState::Tracks) // Downloading continues
                }
            }
        }
        None => {
            info!("No further activities without track, stop downloading (can be re-enabled)");
            Ok(DownloadState::NoResults)
        }
    }
}

async fn try_task(state: &MutexSharedState, strava_url: &str) -> Result<DownloadDelay, BoxError> {
    let mut new_delay = DownloadDelay::Short;
    let download_state = get_download_state(state).await;
    if download_state.is_active() {
        match get_bearer(state).await? {
            Some(bearer) => {
                let new_state= match download_state {
                    DownloadState::Activities => activity_task(state, strava_url, bearer.into()).await?,
                    DownloadState::Tracks => stream_task(state, strava_url, bearer.into()).await?,
                    _ => download_state.clone()
                };
                new_delay = download_state.new_delay(&new_state);
                set_download_state(state, new_state).await;
                send_status_event(state).await?; // Send status event to update the frontend
            }
            None => {
                // This should not happen because the REST API allows enabling the downloader only if
                // authenticated. There is no way for the downloader to do an OAuth auth code flow.
                warn!("Not authorized, skip execution of download task");
            }
        }
    } else {
        trace!("Download disabled, skip task execution");
    }
    Ok(new_delay)
}

// Must be async as required by tokio::select!
async fn repeat(state: MutexSharedState, strava_url: &str, long_period: Duration, short_period: Duration, mut rx_term: Receiver<()>) {
    let mut curr_delay = DownloadDelay::Short;
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match try_task(&state, strava_url).await {
                    Ok(new_delay) => if new_delay != curr_delay {
                        match new_delay {
                            DownloadDelay::Long => {
                                debug!("Switch to LONG download delay");
                                interval = time::interval(long_period)
                            },
                            DownloadDelay::Short => {
                                debug!("Switch to SHORT download delay");
                                interval = time::interval(short_period)
                            }
                        }
                        interval.tick().await;
                        curr_delay = new_delay;
                    }
                    Err(e) => {
                        warn!("Task failed: {:?}, leave downloader", e);
                        break;
                    }
                }
            },
            _ = rx_term.recv() => {
                debug!("Termination signal received, leave downloader");
                break;
            }
        }
    }
}

pub fn spawn_download_scheduler(state: MutexSharedState, rx_term: Receiver<()>, strava_url: String, period: Duration) -> JoinHandle<()> {
    info!("Spawn download scheduler");
    tokio::spawn(async move {
        repeat(state, &strava_url, period, Duration::from_millis(500), rx_term).await;
    })
}
