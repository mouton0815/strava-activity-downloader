use std::{env, fs};
use std::time::Duration;
use axum::BoxError;
use config::{Config, File};
use log::info;
use tokio::join;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use strava_activity_downloader::domain::server_status::ServerStatus;
use strava_activity_downloader::oauth::oauth_client::OAuthClient;
use strava_activity_downloader::rest::http_server::spawn_http_server;
use strava_activity_downloader::rest::rest_paths::{AUTH_CALLBACK, MAP_DIR, STATUS, WEB_DIR};
use strava_activity_downloader::service::activity_service::ActivityService;
use strava_activity_downloader::service::download_scheduler::spawn_download_scheduler;
use strava_activity_downloader::state::shared_state::SharedState;
use strava_activity_downloader::track::track_storage::TrackStorage;
use strava_activity_downloader::util::shutdown_signal::shutdown_signal;

const CONFIG_YAML : &str = "conf/application.yaml";
const ACTIVITY_DB: &str = "activity.db";

const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 2525;
const DEFAULT_DATA_DIR: &str = "data";

#[tokio::main]
async fn main() -> Result<(), BoxError>  {
    env_logger::init();

    let config = Config::builder()
        .add_source(File::with_name(CONFIG_YAML))
        .build()?;

    let host = env::var("HOST")
        .unwrap_or_else(|_| config.get_string("server.host")
            .unwrap_or(DEFAULT_HOST.to_string()));

    let port = env::var("PORT") // Environment precedes config
        .unwrap_or_else(|_| config.get_string("server.port")
            .unwrap_or(DEFAULT_PORT.to_string()))
        .parse::<u16>()
        .expect("The port must be numeric");

    let strava_url = config.get_string("strava.api_url").unwrap_or("https://www.strava.com/api/v3".to_string());
    let request_period = config.get_int("strava.request_period").unwrap_or(10) as u64;
    let activities_per_page = config.get_int("strava.activities_per_page").unwrap_or(30) as u16;

    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| config.get_string("oauth.redirect_url")
            .unwrap_or(format!("http://{host}:{port}")));

    let scopes : Vec<String> = config.get_array("oauth.scopes").unwrap_or(Vec::new())
        .iter().map(|v| v.clone().into_string().expect(CONFIG_YAML)).collect();

    let client = OAuthClient::new(
        config.get_string("oauth.client_id").expect(CONFIG_YAML),
        config.get_string("oauth.client_secret").expect(CONFIG_YAML),
        config.get_string("oauth.auth_url").expect(CONFIG_YAML),
        config.get_string("oauth.token_url").expect(CONFIG_YAML),
        config.get_string("oauth.target_url").unwrap_or(STATUS.to_string()),
        format!("{redirect_url}{AUTH_CALLBACK}"),
        scopes)?;

    let base_path = env::var("DATA_DIR") // Environment precedes config
        .unwrap_or_else(|_| config.get_string("service.data_dir")
            .unwrap_or(DEFAULT_DATA_DIR.to_string()));
    info!("Data base path: {base_path}");
    fs::create_dir_all(base_path.as_str())?;

    let db_path = format!("{base_path}/{ACTIVITY_DB}");
    let store_tiles = config.get_bool("service.store_tiles").unwrap_or(false);
    let service = ActivityService::new(db_path.as_str(), store_tiles)?;

    let tracks = TrackStorage::new(base_path.as_str());

    // Channel for distributing the termination signal to the treads
    let (tx_term, rx_term1) = broadcast::channel(1);
    let rx_term2 = tx_term.subscribe();

    // Channel for sending data from the producer to the SSE handler
    let (tx_data, _rx_data) = broadcast::channel::<ServerStatus>(3);

    let state = SharedState::new(client, service, tracks, tx_data, tx_term.clone(), activities_per_page);

    let request_period = Duration::from_secs(request_period);
    let downloader = spawn_download_scheduler(state.clone(), rx_term1, strava_url, request_period);

    let addr = format!("{host}:{port}");
    info!("Server listening on http://{addr}");
    let listener = TcpListener::bind(addr).await?;
    let http_server = spawn_http_server(listener, state.clone(), rx_term2, &WEB_DIR, &MAP_DIR);

    shutdown_signal().await;
    info!("Termination signal received");
    tx_term.send(())?;

    let (_,_) = join!(downloader, http_server);
    info!("Downloader terminated");
    info!("HTTP Server terminated");

    Ok(())
}
