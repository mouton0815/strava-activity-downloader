use axum::{middleware, Router};
use axum::routing::get;
use log::{debug, info};
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use crate::{AUTH_CALLBACK, MutexSharedState, oauth};
use crate::rest::handlers::{retrieve, toggle};

pub fn spawn_http_server(listener: TcpListener, state: MutexSharedState, mut rx: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn HTTP server");

    let router = Router::new()
        .route("/retrieve", get(retrieve))
        .route("/toggle", get(toggle))
        .route(AUTH_CALLBACK, get(oauth::callback))
        .route_layer(middleware::from_fn_with_state(state.clone(), oauth::middleware))
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
