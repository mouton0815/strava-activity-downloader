use axum::{BoxError, Error, Json};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Sse;
use axum::response::sse::Event;
use axum_macros::debug_handler;
use futures::Stream;
use log::{debug, info, warn};
use tokio::sync::broadcast::Receiver;
use crate::domain::download_state::DownloadState;
use crate::domain::server_status::ServerStatus;
use crate::rest::rest_paths::{STATUS, TILES, TOGGLE};
use crate::domain::map_tile::MapTile;
use crate::domain::map_zoom::MapZoom;
use crate::state::shared_state::MutexSharedState;

#[allow(dead_code)]
fn reqwest_error(error: reqwest::Error) -> StatusCode {
    warn!("{}", error);
    // Need to map reqwest::StatusCode to axum::http::StatusCode.
    // Note that both types are actually aliases of http::StatusCode, but Rust complains.
    let status = error.status().map(|e| e.as_u16()).unwrap_or(500_u16);
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn error_to_status(error: BoxError) -> StatusCode {
    warn!("{}", error);
    StatusCode::INTERNAL_SERVER_ERROR
}

#[debug_handler]
pub async fn toggle_handler(State(state): State<MutexSharedState>)
    -> Result<Json<DownloadState>, StatusCode> {
    debug!("Enter {}", TOGGLE);
    let mut guard = state.lock().await;
    match guard.oauth.get_bearer().await.map_err(error_to_status)? {
        Some(_) => {
            guard.download_state = guard.download_state.toggle();
            Ok(Json(guard.download_state.clone()))
        },
        None => {
            info!("Unauthorized, cannot enable the download scheduler");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/*
#[debug_handler]
pub async fn status(State(state): State<MutexSharedState>) -> Result<Json<ServerStatus>, StatusCode> {
    debug!("Enter {}", STATUS);
    let mut guard = state.lock().await;
    let status = guard.get_server_status().await.map_err(service_error)?;
    Ok(Json(status))
}
*/

#[debug_handler]
pub async fn status_handler(State(state): State<MutexSharedState>)
    -> Result<Sse<impl Stream<Item = Result<Event, Error>>>, StatusCode> {
    debug!("Enter {}", STATUS);
    let mut receiver = subscribe_and_send_first(&state).await.map_err(error_to_status)?;
    let mut rx_term = subscribe_term(&state).await;
    let stream = async_stream::stream! {
        loop {
            tokio::select! {
                item = receiver.recv() => {
                    yield Event::default().json_data(item.unwrap());
                }
                _ = rx_term.recv() => {
                    debug!("Termination signal received, leave SSE handler");
                    return;
                }
            }
        }
    };
    Ok(Sse::new(stream))
}

#[debug_handler]
pub async fn tiles_handler(State(state): State<MutexSharedState>, Path(zoom): Path<u16>)
    -> Result<Json<Vec<MapTile>>, StatusCode> {
    debug!("Enter {TILES} for level {zoom}");
    let zoom = match zoom {
        14 => MapZoom::Level14,
        17 => MapZoom::Level17,
        _ => return Err(StatusCode::BAD_REQUEST)
    };
    let mut guard = state.lock().await;
    let tiles = guard.service.get_tiles(zoom).map_err(error_to_status)?;
    Ok(Json(tiles))
}

async fn subscribe_and_send_first(state: &MutexSharedState) -> Result<Receiver<ServerStatus>, BoxError> {
    let mut guard = state.lock().await;
    let receiver = guard.tx_data.subscribe();
    let status = guard.get_server_status().await?;
    guard.tx_data.send(status)?;
    Ok(receiver)
}

async fn subscribe_term(state: &MutexSharedState) -> Receiver<()> {
    let guard = state.lock().await;
    guard.tx_term.subscribe()
}
