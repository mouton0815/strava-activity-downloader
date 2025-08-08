use axum::Router;
use axum::http::Method;
use axum::routing::get;
use log::{debug, info};
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use crate::rest::rest_handlers::{status, tiles, toggle};
use crate::rest::oauth_handlers::{authorize, callback};
use crate::rest::rest_paths::{AUTH_CALLBACK, AUTHORIZE, StaticDir, STATUS, TILES, TOGGLE};
use crate::state::shared_state::MutexSharedState;

pub fn spawn_http_server(
    listener: TcpListener,
    state: MutexSharedState,
    mut rx_term: Receiver<()>,
    web_dir: &StaticDir,
    map_dir: &StaticDir) -> JoinHandle<()> {
    info!("Spawn HTTP server");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::PUT])
        .allow_origin(Any);

    let router = Router::new()
        .route(STATUS, get(status))
        .route(TOGGLE, get(toggle))
        .route(AUTHORIZE, get(authorize))
        .route(AUTH_CALLBACK, get(callback))
        .route(TILES, get(tiles))
        .layer(ServiceBuilder::new().layer(cors))
        .nest_service(web_dir.rest_path, ServeDir::new(web_dir.file_path))
        .nest_service(map_dir.rest_path, ServeDir::new(map_dir.file_path))
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
    use tokio::sync::broadcast;
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_zoom::MapZoom;
    use crate::domain::server_status::ServerStatus;
    use crate::oauth::oauth_client::OAuthClient;
    use crate::rest::rest_paths::TILES;
    use crate::rest::rest_handlers::tiles;
    use crate::service::activity_service::ActivityService;
    use crate::state::shared_state::SharedState;
    use crate::track::track_storage::TrackStorage;

    #[tokio::test]
    async fn test_tiles() {
        let activity_id = 5;
        let activities = vec![Activity::dummy(activity_id, "2018-02-20T18:02:13Z")];
        let tiles_14: Vec<MapTile> = vec![MapTile::new(1, 1), MapTile::new(2, 2)];

        let mut service = ActivityService::new(":memory:", true).unwrap();
        assert!(service.add(&activities).is_ok());
        assert!(service.put_tiles(MapZoom::Level14, activity_id, &tiles_14).is_ok());

        let client = OAuthClient::dummy();
        let tracks = TrackStorage::new(""); // TODO: Allow disabling track storage
        let (tx_data, _) = broadcast::channel::<ServerStatus>(3);
        let (tx_term, _) = broadcast::channel(1);
        let state = SharedState::new(client, service, tracks, tx_data, tx_term, 0);

        let router = Router::new()
            .route(TILES, get(tiles))
            .with_state(state);

        let server = TestServer::new(router).unwrap();
        let text = server.get("/tiles/14").await.json::<Vec<MapTile>>();
        assert_eq!(text, tiles_14);
    }
}