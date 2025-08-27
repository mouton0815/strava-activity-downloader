use axum::Router;
use axum::http::Method;
use axum::response::Redirect;
use axum::routing::get;
use log::{debug, info};
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use crate::rest::rest_handlers::{status_handler, tiles_handler, toggle_handler};
use crate::rest::oauth_handlers::{authorize_handler, callback_handler};
use crate::rest::rest_paths::{AUTH_CALLBACK, AUTHORIZE, StaticDir, STATUS, TILES, TOGGLE};
use crate::rest::timing_layer::TimingLayer;
use crate::state::shared_state::MutexSharedState;

pub fn spawn_http_server(
    listener: TcpListener,
    state: MutexSharedState,
    mut rx_term: Receiver<()>,
    console_dir: &StaticDir,
    tilemap_dir: &StaticDir) -> JoinHandle<()> {
    info!("Spawn HTTP server");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::PUT])
        .allow_origin(Any);

    let router = Router::new()
        .route(STATUS, get(status_handler))
        .route(TOGGLE, get(toggle_handler))
        .route(AUTHORIZE, get(authorize_handler))
        .route(AUTH_CALLBACK, get(callback_handler))
        .route(TILES, get(tiles_handler))
        .route("/", get(|| async { Redirect::permanent(console_dir.rest_path) }))
        .nest_service(console_dir.rest_path, ServeDir::new(console_dir.file_path))
        .nest_service(tilemap_dir.rest_path, ServeDir::new(tilemap_dir.file_path))
        .layer(ServiceBuilder::new().layer(cors))
        .layer(ServiceBuilder::new().layer(TimingLayer))
        .with_state(state);

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                rx_term.recv().await.unwrap();
                debug!("Termination signal received, leave HTTP server");
            })
            .await
            .unwrap() // Panic accepted
    })
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::routing::get;
    use axum_test::TestServer;
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_zoom::MapZoom;
    use crate::rest::rest_paths::TILES;
    use crate::rest::rest_handlers::tiles_handler;
    use crate::service::activity_service::ActivityService;
    use crate::state::shared_state::SharedState;

    #[tokio::test]
    async fn test_tiles() {
        let tiles: Vec<MapTile> = vec![MapTile::new(1, 1), MapTile::new(2, 2)];
        let server = create_tiles_server(&tiles, MapZoom::Level14);
        let result = server.get("/tiles/14").await.json::<Vec<MapTile>>();
        assert_eq!(result, tiles);
    }

    #[tokio::test]
    async fn test_tiles_bounds() {
        let tiles: Vec<MapTile> = vec![MapTile::new(1, 1), MapTile::new(2, 2)];
        let server = create_tiles_server(&tiles, MapZoom::Level14);
        let result1 = server.get("/tiles/14?bounds=2,2,2,2").await.json::<Vec<MapTile>>();
        assert_eq!(result1, vec![MapTile::new(2, 2)]);
    }

    fn create_tiles_server(tiles: &Vec<MapTile>, zoom: MapZoom) -> TestServer {
        let activity_id = 5;
        let activities = vec![Activity::dummy(activity_id, "2018-02-20T18:02:13Z")];

        let mut service = ActivityService::new(":memory:", true).unwrap();
        service.add(&activities).unwrap();
        service.put_tiles(zoom, activity_id, &tiles).unwrap();

        let state = SharedState::dummy(service);

        let router = Router::new()
            .route(TILES, get(tiles_handler))
            .with_state(state);

        TestServer::new(router).unwrap()
    }
}