use axum::{BoxError, Error, Json};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Sse;
use axum::response::sse::Event;
use axum_macros::debug_handler;
use futures::Stream;
use log::{info, warn};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt; // Enable Iterator trait for BroadcastStream
use crate::domain::download_state::DownloadState;
use crate::domain::server_status::ServerStatus;
use crate::rest::rest_paths::{STATUS, TOGGLE};
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
pub async fn toggle(State(state): State<MutexSharedState>) -> Result<Json<DownloadState>, StatusCode> {
    info!("Enter {}", TOGGLE);
    let mut guard = state.lock().await;
    match (*guard).oauth.get_bearer().await.map_err(service_error)? {
        Some(_) => {
            (*guard).download_state = (*guard).download_state.toggle();
            Ok(Json((*guard).download_state.clone()))
        },
        None => {
            info!("Unauthorized, cannot enable the download scheduler");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[debug_handler]
pub async fn status(State(state): State<MutexSharedState>)
    -> Result<Sse<impl Stream<Item = Result<Event, Error>>>, StatusCode> {
    info!("Enter {}", STATUS);
    let stream = subscribe_and_send_first(&state).await.map_err(service_error)?;
    let stream = stream.map(move |item| {
        Event::default().json_data(item.unwrap())
    });
    Ok(Sse::new(stream))
}

async fn subscribe_and_send_first(state: &MutexSharedState) -> Result<BroadcastStream<ServerStatus>, BoxError> {
    let mut guard = state.lock().await;
    let receiver = (*guard).sender.subscribe();
    let status = (*guard).get_server_status().await?;
    (*guard).sender.send(status)?;
    Ok(BroadcastStream::new(receiver))
}
