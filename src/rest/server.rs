use axum::{middleware, Router};
use axum::routing::get;
use log::{debug, info};
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use crate::AUTH_CALLBACK;
use crate::rest::handlers::{retrieve, status, toggle};
use crate::rest::oauth::{callback, middleware};
use crate::state::shared_state::MutexSharedState;

pub fn spawn_http_server(listener: TcpListener, state: MutexSharedState, mut rx: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn HTTP server");

    let router = Router::new()
        .route("/status", get(status))
        .route("/retrieve", get(retrieve))
        .route("/toggle", get(toggle))
        .route(AUTH_CALLBACK, get(callback))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware))
        .with_state(state);

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                rx.recv().await.unwrap();
                debug!("Termination signal received, leave HTTP server");
            })
            .await
            .unwrap() // May panic
    })
}
