use std::sync::Arc;
use axum::{BoxError, Json, Router};
use axum::http::{Method, StatusCode};
use axum::routing::get;
use axum::extract::State;
use axum_macros::debug_handler;
use config::{Config, File};
use log::{debug, info, warn};
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use strava_activity_downloader::domain::map_tile::MapTile;
use strava_activity_downloader::domain::map_zoom::MapZoom;
use strava_activity_downloader::rest::rest_paths::TILES;
use strava_activity_downloader::service::activity_service::ActivityService;

type MutexService = Arc<Mutex<ActivityService>>;

const CONFIG_YAML : &str = "conf/application.yaml";
const ACTIVITY_DB: &str = "activity.db"; // TODO: Move to application.yaml

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    env_logger::init();

    let config = Config::builder()
        .add_source(File::with_name(CONFIG_YAML))
        .build()?;

    //let service = Arc::new(Mutex::new(ActivityService::new(ACTIVITY_DB, true)?));
    let service = ActivityService::new(ACTIVITY_DB, true)?;
    let state = Arc::new(Mutex::new(service));

    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any);

    let router = Router::new()
        .route(TILES, get(tiles))
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(state);

    let host = config.get_string("server.host").unwrap_or("localhost".to_string());
    let port = config.get_int("server.port").unwrap_or(3000) as u16;
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;

    info!("Tile server running at http://{host}:{port}{TILES}");
    Ok(axum::serve(listener, router).await?)
}

#[debug_handler]
async fn tiles(State(state): State<MutexService>) -> Result<Json<Vec<MapTile>>, StatusCode> {
    debug!("Enter {}", TILES);
    let mut guard = state.lock().await;
    let tiles = guard.get_tiles(MapZoom::Level14).map_err(error_to_status)?; // TODO: Pass zoom as parameter
    Ok(Json(tiles))
}

fn error_to_status(error: BoxError) -> StatusCode {
    warn!("{}", error);
    StatusCode::INTERNAL_SERVER_ERROR
}
