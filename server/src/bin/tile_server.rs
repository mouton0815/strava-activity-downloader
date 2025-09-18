/////////////////////////////////////////////////////
//
// DEPRECATED, tiles are now served by the main app
//
/////////////////////////////////////////////////////

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use axum::{BoxError, Json, Router};
use axum::http::{Method, StatusCode};
use axum::routing::get;
use axum::extract::{Path, State};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use strava_activity_downloader::domain::map_tile::MapTile;
use strava_activity_downloader::domain::map_zoom::MapZoom;
use strava_activity_downloader::rest::rest_paths::TILES;
use strava_activity_downloader::service::activity_service::ActivityService;

type MutexService = Arc<Mutex<ActivityService>>;

const ACTIVITY_DB: &str = "activity.db";

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    env_logger::init();

    //let service = Arc::new(Mutex::new(ActivityService::new(ACTIVITY_DB, true)?));
    let service = ActivityService::new(ACTIVITY_DB, true).await?;
    let state = Arc::new(Mutex::new(service));

    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any);

    let router = Router::new()
        .route(TILES, get(tiles))
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(state);

    // Get port from env or default to 8080
    let port = env::var("PORT")
        .unwrap_or("2727".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Tile server listening on https://{addr}{TILES}");
    let listener = TcpListener::bind(addr).await?;
    Ok(axum::serve(listener, router).await?)
}

#[debug_handler]
async fn tiles(State(state): State<MutexService>, Path(zoom): Path<u16>) -> Result<Json<Vec<MapTile>>, StatusCode> {
    debug!("Enter {TILES} for level {zoom}");
    let zoom = match zoom {
        14 => MapZoom::Level14,
        17 => MapZoom::Level17,
        _ => return Err(StatusCode::BAD_REQUEST)
    };
    let mut guard = state.lock().await;
    let tiles = guard.get_tiles(zoom, None).await.map_err(error_to_status)?;
    Ok(Json(tiles))
}

fn error_to_status(error: BoxError) -> StatusCode {
    warn!("{}", error);
    StatusCode::INTERNAL_SERVER_ERROR
}
