use std::convert::Infallible;
use axum::BoxError;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Sse;
use axum::response::sse::Event;
use axum_macros::debug_handler;
use futures::Stream;
use log::{info, warn};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _; // Enable Iterator trait for BroadcastStream
use crate::rest::rest_paths::{STATUS, TOGGLE};
use crate::state::shared_state::{MutexSharedState, SchedulerState};

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

fn serialize_error(error: serde_json::Error) -> StatusCode {
    warn!("{}", error);
    StatusCode::INTERNAL_SERVER_ERROR
}

#[debug_handler]
pub async fn toggle(State(state): State<MutexSharedState>) -> Result<String, StatusCode> {
    info!("Enter {}", TOGGLE);
    let mut guard = state.lock().await;
    match (*guard).oauth.get_bearer().await.map_err(service_error)? {
        Some(_) => {
            (*guard).scheduler_state = match (*guard).scheduler_state {
                SchedulerState::Inactive => SchedulerState::DownloadActivities,
                SchedulerState::DownloadActivities => SchedulerState::Inactive,
                SchedulerState::DownloadStreams => SchedulerState::Inactive
            };
            Ok(serde_json::to_string(&(*guard).scheduler_state).map_err(serialize_error)?)
        },
        None => {
            info!("Unauthorized, cannot enable the scheduler");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[debug_handler]
pub async fn status(State(state): State<MutexSharedState>)
    -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    info!("Enter {}", STATUS);
    let stream = subscribe_and_send_first(&state).await.map_err(service_error)?;
    let stream = stream.map(move |item| {
        Ok::<Event, Infallible>(Event::default().data(item.unwrap()))
    });
    Ok(Sse::new(stream))
}

async fn subscribe_and_send_first(state: &MutexSharedState) -> Result<BroadcastStream<String>, BoxError> {
    let mut guard = state.lock().await;
    let receiver = (*guard).sender.subscribe();
    let server_status = (*guard).get_server_status().await?;
    let server_status = serde_json::to_string(&server_status)?;
    (*guard).sender.send(server_status)?;
    Ok(BroadcastStream::new(receiver))
}
