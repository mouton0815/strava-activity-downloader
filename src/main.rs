use std::error::Error;
use axum::{middleware, Router};
use axum::body::Body;
use axum::extract::Extension;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum_macros::debug_handler;
use config::{Config, File};
use log::{debug, info, warn};
use crate::header::CONTENT_TYPE;
use crate::oauth::client::{AUTH_CALLBACK, OAuthClient};
use crate::oauth::OAuthState;
use crate::oauth::token::{Bearer, TokenHolder};

mod oauth;

const CONFIG_YAML : &'static str = "conf/application.yaml";

fn log_error(error: reqwest::Error) -> StatusCode {
    warn!("-----> {:?}", error);
    StatusCode::INTERNAL_SERVER_ERROR
}

#[debug_handler]
async fn retrieve(Extension(bearer): Extension<Bearer>) -> Result<Response, StatusCode> {
    info!("Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);

    let result : String = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete")
        .header(reqwest::header::AUTHORIZATION, bearer)
        .send().await.map_err(log_error)?
        .text().await.map_err(log_error)?;
    info!("-----> {:?}", result);

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from("Hallo Welt!"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();
    let config = Config::builder()
        .add_source(File::with_name(CONFIG_YAML))
        .build()?;

    let host = config.get_string("server.host").unwrap_or("localhost".to_string());
    let port = config.get_int("server.port").unwrap_or(3000) as u64;

    let client = OAuthClient::new(&host, port,
        &config.get_string("oauth.client_id").expect(CONFIG_YAML),
        &config.get_string("oauth.client_secret").expect(CONFIG_YAML),
        &config.get_string("oauth.auth_url").expect(CONFIG_YAML),
        &config.get_string("oauth.token_url").expect(CONFIG_YAML))?;
    let state = OAuthState::new(client);

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route(AUTH_CALLBACK, get(oauth::callback))
        .route_layer(middleware::from_fn_with_state(state.clone(), oauth::middleware))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    Ok(axum::serve(listener, app).await?)
}
