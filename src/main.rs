use std::error::Error;
use std::time::Duration;
use axum::{middleware, Router};
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_macros::debug_handler;
use config::{Config, File};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{join, signal};
use tokio::sync::broadcast;
use crate::oauth::client::{AUTH_CALLBACK, OAuthClient};
use crate::oauth::token::{Bearer, TokenHolder};
use crate::scheduler::spawn_scheduler;
use crate::state::{MutexSharedState, SharedState};

mod oauth;
mod state;
mod scheduler;

const CONFIG_YAML : &'static str = "conf/application.yaml";

fn log_error(error: reqwest::Error) -> StatusCode {
    warn!("{}", error);
    // Need to map reqwest::StatusCode to axum::http::StatusCode.
    // Note that both types are actually aliases of http::StatusCode, but Rust complains.
    let status = error.status().map(|e| e.as_u16()).unwrap_or(500 as u16);
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Debug, Deserialize, Serialize)]
struct Activity {
    name: String,
    sport_type: String,
    start_date_local: String, // TODO: Parse into Datetime or smth
    distance: f64,
    kudos_count: u64
}

type Activities = Vec<Activity>;

#[debug_handler]
async fn retrieve(Extension(bearer): Extension<Bearer>) -> Result<Response, StatusCode> {
    info!("Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);
    /*
    // let query = vec![("after", "1701388800")];
    let result = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete/activities")
        .header(reqwest::header::AUTHORIZATION, bearer)
        //.query(&query)
        .send().await.map_err(log_error)?
        .error_for_status().map_err(log_error)?
        .json::<Activities>().await.map_err(log_error)?;

    info!("--r--> {:?}", result);
    Ok(Json(result).into_response())
    */
    Ok("Hallo Welt".into_response())
}

#[debug_handler]
async fn toggle(State(state): State<MutexSharedState>) -> Result<Response, StatusCode> {
    info!("Enter /toggle");
    let mut guard = state.lock().await;
    let old_value = (*guard).scheduler_running.clone();
    (*guard).scheduler_running = !old_value;
    Ok(old_value.to_string().into_response())
}

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Cannot obtain or refresh token")]
    TokenError
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();
    let config = Config::builder()
        .add_source(File::with_name(CONFIG_YAML))
        .build()?;

    let host = config.get_string("server.host").unwrap_or("localhost".to_string());
    let port = config.get_int("server.port").unwrap_or(3000) as u16;
    let scopes : Vec<String> = config.get_array("oauth.scopes").unwrap_or(Vec::new())
        .iter().map(|v| v.clone().into_string().expect(CONFIG_YAML)).collect();

    let client = OAuthClient::new(&host, port,
        config.get_string("oauth.client_id").expect(CONFIG_YAML),
        config.get_string("oauth.client_secret").expect(CONFIG_YAML),
        config.get_string("oauth.auth_url").expect(CONFIG_YAML),
        config.get_string("oauth.token_url").expect(CONFIG_YAML),
        scopes)?;

    let (tx, rx1) = broadcast::channel(1);
    let mut rx2 = tx.subscribe();

    let period = Duration::from_secs(10);
    let state = SharedState::new(client, true);
    let delete_scheduler = spawn_scheduler(state.clone(), rx1, period);

    let router = Router::new()
        .route("/retrieve", get(retrieve))
        .route("/toggle", get(toggle))
        .route(AUTH_CALLBACK, get(oauth::callback))
        .route_layer(middleware::from_fn_with_state(state.clone(), oauth::middleware))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    let http_server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                rx2.recv().await.unwrap();
                info!("Termination signal received, leave HTTP server");
            })
            .await
    });

    await_shutdown().await;
    debug!("Termination signal received");
    tx.send(())?;

    let (_,_) = join!(delete_scheduler, http_server);
    info!("Deletion scheduler terminated");
    info!("HTTP Server terminated");

    Ok(())
}

// See https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn await_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}