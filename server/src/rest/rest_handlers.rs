use std::num::ParseIntError;
use axum::{BoxError, Error, Json};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Sse;
use axum::response::sse::Event;
use axum::http::Uri;
use axum_macros::debug_handler;
use futures::Stream;
use log::{debug, info, warn};
use serde::Deserialize;
use tokio::sync::broadcast::Receiver;
use crate::domain::download_state::DownloadState;
use crate::domain::server_status::ServerStatus;
use crate::domain::map_tile::MapTile;
use crate::domain::map_tile_bounds::MapTileBounds;
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

fn internal_server_error(error: BoxError) -> StatusCode {
    warn!("{}", error);
    StatusCode::INTERNAL_SERVER_ERROR
}

#[debug_handler]
pub async fn toggle_handler(State(state): State<MutexSharedState>, uri: Uri)
    -> Result<Json<DownloadState>, StatusCode> {
    debug!("Enter {uri}");
    let mut guard = state.lock().await;
    match guard.oauth.get_bearer().await.map_err(internal_server_error)? {
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
pub async fn status_handler(State(state): State<MutexSharedState>, uri: Uri)
    -> Result<Sse<impl Stream<Item = Result<Event, Error>>>, StatusCode> {
    debug!("Enter {uri}");
    let mut receiver = subscribe_and_send_first(&state).await.map_err(internal_server_error)?;
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

#[derive(Deserialize, Debug)]
pub struct TilesParams {
    bounds: Option<String>
}

#[debug_handler]
pub async fn tiles_handler(State(state): State<MutexSharedState>, uri: Uri, Path(zoom): Path<u16>, Query(params): Query<TilesParams>)
    -> Result<Json<Vec<MapTile>>, StatusCode> {
    debug!("Enter {uri}");
    let zoom = parse_zoom(zoom)?;
    let bounds = parse_bounds(params.bounds)?;
    let mut guard = state.lock().await;
    let tiles = guard.service.get_tiles(zoom, bounds).await.map_err(internal_server_error)?;
    Ok(Json(tiles))
}

fn parse_zoom(zoom: u16) -> Result<MapZoom, StatusCode> {
    match zoom {
        14 => Ok(MapZoom::Level14),
        17 => Ok(MapZoom::Level17),
        _ => Err(StatusCode::BAD_REQUEST)
    }
}

fn parse_bounds(bounds: Option<String>) -> Result<Option<MapTileBounds>, StatusCode> {
    match bounds {
        Some(bounds_str) => {
            let parsed: Result<Vec<u64>, ParseIntError> = bounds_str.split(",")
                .map(|token| token.parse::<u64>())
                .collect();
            match parsed {
                Ok(coords) => {
                    if coords.len() == 4 {
                        Ok(Some(MapTileBounds::new(coords[0], coords[1], coords[2], coords[3])))
                    } else {
                        warn!("Malformed parameter: bounds={bounds_str} (need four positions)");
                        Err(StatusCode::BAD_REQUEST)
                    }
                }
                Err(_) => {
                    warn!("Malformed parameter: bounds={bounds_str} (positions not numeric)");
                    Err(StatusCode::BAD_REQUEST)
                }
            }
        },
        None => Ok(None)
    }
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
