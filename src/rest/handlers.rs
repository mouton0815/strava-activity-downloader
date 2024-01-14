use std::convert::Infallible;
use axum::{BoxError, Json};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response, Sse};
use axum::response::sse::Event;
use axum_macros::debug_handler;
use futures::Stream;
use log::{info, warn};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _; // Enable Iterator trait for BroadcastStream
use crate::domain::server_status::ServerStatus;
use crate::rest::paths::{STATUS, STATUS_EVENTS, TOGGLE};
use crate::state::shared_state::MutexSharedState;

#[allow(dead_code)]
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

#[debug_handler]
pub async fn status(State(state): State<MutexSharedState>) -> Result<Json<ServerStatus>, StatusCode> {
    info!("Enter {}", STATUS);
    let mut guard = state.lock().await;
    let server_status = (*guard).get_server_status().await.map_err(service_error)?;
    Ok(Json(server_status))
}

#[debug_handler]
pub async fn toggle(State(state): State<MutexSharedState>) -> Result<Response, StatusCode> {
    info!("Enter {}", TOGGLE);
    let mut guard = state.lock().await;
    let old_value = (*guard).scheduler_running.clone();
    (*guard).scheduler_running = !old_value;
    Ok((*guard).scheduler_running.to_string().into_response())
}

#[debug_handler]
pub async fn status_events(State(state): State<MutexSharedState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("Enter {}", STATUS_EVENTS);
    let stream = create_stream(&state).await;
    let stream = stream.map(move |item| {
        Ok::<Event, Infallible>(Event::default().data(item.unwrap()))
    });
    Sse::new(stream)
}

async fn create_stream(state: &MutexSharedState) -> BroadcastStream<String> {
    let guard = state.lock().await;
    let receiver = (*guard).sender.subscribe();
    BroadcastStream::new(receiver)
}
