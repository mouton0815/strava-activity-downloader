use axum::{BoxError, Extension, Json};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use crate::Bearer;
use crate::domain::status::Status;
use crate::state::shared_state::MutexSharedState;

#[allow(dead_code)]
fn reqwest_error(error: reqwest::Error) -> StatusCode {
    warn!("{}", error);
    // Need to map reqwest::StatusCode to axum::http::StatusCode.
    // Note that both types are actually aliases of http::StatusCode, but Rust complains.
    let status = error.status().map(|e| e.as_u16()).unwrap_or(500 as u16);
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[allow(dead_code)]
fn service_error(error: BoxError) -> StatusCode {
    warn!("{}", error);
    StatusCode::INTERNAL_SERVER_ERROR
}

#[debug_handler]
pub async fn retrieve(State(_state): State<MutexSharedState>, Extension(bearer): Extension<Bearer>) -> Result<Response, StatusCode> {
    info!("Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);

    Ok(Json("Heho").into_response()) // TODO: Do something with the result
}

#[debug_handler]
pub async fn status(State(state): State<MutexSharedState>) -> Result<Json<Status>, StatusCode> {
    info!("Enter /status");
    let mut guard = state.lock().await;
    let authorized = (*guard).oauth.get_bearer().await.map_err(service_error)?.is_some();
    let scheduling = (*guard).scheduler_running.clone();
    let activity_stats = (*guard).service.get_stats().map_err(service_error)?;
    Ok(Json(Status::new(authorized, scheduling, activity_stats)))
}

#[debug_handler]
pub async fn toggle(State(state): State<MutexSharedState>) -> Result<Response, StatusCode> {
    info!("Enter /toggle");
    let mut guard = state.lock().await;
    let old_value = (*guard).scheduler_running.clone();
    (*guard).scheduler_running = !old_value;
    Ok((*guard).scheduler_running.to_string().into_response())
}
