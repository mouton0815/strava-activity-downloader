use std::error::Error;
use std::time::Duration;
use config::{Config, File};
use log::info;
use tokio::join;
use tokio::sync::broadcast;
use crate::domain::activity_stream::ActivityStream;
use crate::domain::server_status::ServerStatus;
use crate::oauth::client::OAuthClient;
use crate::oauth::token::{Bearer, TokenHolder};
use crate::rest::rest_paths::{AUTH_CALLBACK, STATUS};
use crate::rest::http_server::spawn_http_server;
use crate::downloader::spawn_download_scheduler;
use crate::service::activity_service::ActivityService;
use crate::state::shared_state::SharedState;
use crate::util::shutdown_signal::shutdown_signal;
use crate::util::write_gpx::write_gpx;

mod oauth;
mod rest;
mod downloader;
mod database;
mod domain;
mod util;
mod state;
mod service;

const CONFIG_YAML : &'static str = "conf/application.yaml";

/*
#[tokio::main]
async fn main() -> Result<(), BoxError> {
    env_logger::init();
    let in_file = std::fs::File::open("ermlich.stream")?;
    let reader = BufReader::new(in_file);
    let stream: ActivityStream = serde_json::from_reader(reader)?;
    let activity = Activity::new(12345, "2024-01-01T00:00:00Z");
    write_gpx(12345, "Foo", "2024-01-01T00:00:00Z", &stream)
}
 */

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
    let period = config.get_int("downloader.period").unwrap_or(10) as u64;
    let activities_per_page = config.get_int("strava.activities_per_page").unwrap_or(30) as u16;
    let web_dir = format!("{}/web/dist", std::env::var("CARGO_MANIFEST_DIR").unwrap());

    let client = OAuthClient::new(
        config.get_string("oauth.client_id").expect(CONFIG_YAML),
        config.get_string("oauth.client_secret").expect(CONFIG_YAML),
        config.get_string("oauth.auth_url").expect(CONFIG_YAML),
        config.get_string("oauth.token_url").expect(CONFIG_YAML),
        format!("http://{}:{}{}", host, port, AUTH_CALLBACK),
        config.get_string("oauth.target_url").unwrap_or(STATUS.to_string()),
        scopes)?;

    let service = ActivityService::new("strava.db")?;

    // Channel for distributing the termination signal to the treads
    let (tx_term, rx_term1) = broadcast::channel(1);
    let rx_term2 = tx_term.subscribe();

    // Channel for sending data from the producer to the SSE handler
    let (tx_data, _rx_data) = broadcast::channel::<ServerStatus>(3);

    let state = SharedState::new(client, service, tx_data, activities_per_page);

    let period = Duration::from_secs(period);
    let downloader = spawn_download_scheduler(state.clone(), rx_term1, period);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    let http_server = spawn_http_server(listener, state.clone(), rx_term2, &web_dir);

    shutdown_signal().await;
    info!("Termination signal received");
    tx_term.send(())?;

    let (_,_) = join!(downloader, http_server);
    info!("Downloader terminated");
    info!("HTTP Server terminated");

    Ok(())
}
