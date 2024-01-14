use axum::{middleware, Router};
use axum::http::Method;
use axum::routing::get;
use log::{debug, info};
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use crate::rest::handlers::{status, status_events, toggle};
use crate::rest::oauth::{authorize, callback, middleware};
use crate::rest::paths::{AUTH_CALLBACK, AUTHORIZE, STATUS, STATUS_EVENTS, TOGGLE};
use crate::state::shared_state::MutexSharedState;

pub fn spawn_http_server(listener: TcpListener, state: MutexSharedState, mut rx: Receiver<()>, web_dir: &str) -> JoinHandle<()> {
    info!("Spawn HTTP server");

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::PUT])
        .allow_origin(Any);

    let router = Router::new()
        .route(STATUS, get(status))
        .route(STATUS_EVENTS, get(status_events))
        .route(TOGGLE, get(toggle))
        .route(AUTHORIZE, get(authorize))
        .route(AUTH_CALLBACK, get(callback))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware))
        .layer(ServiceBuilder::new().layer(cors))
        .nest_service("/", ServeDir::new(web_dir))
        .with_state(state);

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                rx.recv().await.unwrap();
                debug!("Termination signal received, leave HTTP server");
            })
            .await
            .unwrap() // Panic accepted
    })
}
