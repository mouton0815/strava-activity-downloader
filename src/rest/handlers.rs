use axum::{BoxError, Extension, Json};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use crate::Bearer;
use crate::domain::activity::ActivityVec;
use crate::state::shared_state::MutexSharedState;

fn reqwest_error(error: reqwest::Error) -> StatusCode {
    warn!("{}", error);
    // Need to map reqwest::StatusCode to axum::http::StatusCode.
    // Note that both types are actually aliases of http::StatusCode, but Rust complains.
    let status = error.status().map(|e| e.as_u16()).unwrap_or(500 as u16);
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn service_error(error: BoxError) -> StatusCode {
    warn!("{}", error);
    StatusCode::INTERNAL_SERVER_ERROR
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

#[debug_handler]
pub async fn retrieve(State(state): State<MutexSharedState>, Extension(bearer): Extension<Bearer>) -> Result<Response, StatusCode> {
    info!("Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);

    let mut client = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete/activities")
        .header(reqwest::header::AUTHORIZATION, bearer);

    if let Some(before) = get_min_time(&state).await.map_err(service_error)? {
        let query = vec![("before", before)];
        client = client.query(&query);
    }

    let activities : ActivityVec = client
        .send().await.map_err(reqwest_error)?
        .error_for_status().map_err(reqwest_error)?
        .json::<ActivityVec>().await.map_err(reqwest_error)?;

    info!("--r--> {:?}", activities);
    add_activities(&state, &activities).await.map_err(service_error)?;

    Ok(Json(activities).into_response()) // TODO: Do someting with the result
}

#[debug_handler]
pub async fn toggle_scheduler(State(state): State<MutexSharedState>) -> Result<Response, StatusCode> {
    info!("Enter /toggle_scheduler");
    let mut guard = state.lock().await;
    let old_value = (*guard).scheduler_running.clone();
    (*guard).scheduler_running = !old_value;
    Ok(old_value.to_string().into_response())
}
