use std::error::Error;
use std::time::Duration;
use config::{Config, File};
use log::{debug, info};
use tokio::{join, signal};
use tokio::sync::broadcast;
use crate::oauth::client::{AUTH_CALLBACK, OAuthClient};
use crate::oauth::token::{Bearer, TokenHolder};
use crate::rest::server::spawn_http_server;
use crate::scheduler::spawn_scheduler;
use crate::state::{MutexSharedState, SharedState};

mod oauth;
mod state;
mod rest;
mod scheduler;
mod database;

const CONFIG_YAML : &'static str = "conf/application.yaml";

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
    let rx2 = tx.subscribe();

    let period = Duration::from_secs(10);
    let state = SharedState::new(client);
    let scheduler = spawn_scheduler(state.clone(), rx1, period);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    let http_server = spawn_http_server(listener, state.clone(), rx2);

    await_shutdown().await;
    info!("Termination signal received");
    tx.send(())?;

    let (_,_) = join!(scheduler, http_server);
    info!("Scheduler terminated");
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