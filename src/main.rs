use std::error::Error;
use axum::{Json, middleware, Router};
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_macros::debug_handler;
use config::{Config, File};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use crate::oauth::client::{AUTH_CALLBACK, OAuthClient};
use crate::oauth::OAuthState;
use crate::oauth::token::{Bearer, TokenHolder};

mod oauth;

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
    kudos_count: u64
}

type Activities = Vec<Activity>;

#[debug_handler]
async fn retrieve(Extension(bearer): Extension<Bearer>) -> Result<Response, StatusCode> {
    info!("Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);

    let query = vec![("after", "1701388800")];
    let result = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete/activities")
        .query(&query)
        .header(reqwest::header::AUTHORIZATION, bearer)
        .send().await.map_err(log_error)?
        .error_for_status().map_err(log_error)?
        .json::<Activities>().await.map_err(log_error)?;

    info!("--r--> {:?}", result);
    Ok(Json(result).into_response())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();
    let config = Config::builder()
        .add_source(File::with_name(CONFIG_YAML))
        .build()?;

    let host = config.get_string("server.host").unwrap_or("localhost".to_string());
    let port = config.get_int("server.port").unwrap_or(3000) as u64;
    let scopes : Vec<String> = config.get_array("oauth.scopes").unwrap_or(Vec::new())
        .iter().map(|v| v.clone().into_string().expect(CONFIG_YAML)).collect();

    let client = OAuthClient::new(&host, port,
        config.get_string("oauth.client_id").expect(CONFIG_YAML),
        config.get_string("oauth.client_secret").expect(CONFIG_YAML),
        config.get_string("oauth.auth_url").expect(CONFIG_YAML),
        config.get_string("oauth.token_url").expect(CONFIG_YAML),
        scopes)?;
    let state = OAuthState::new(client);

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route(AUTH_CALLBACK, get(oauth::callback))
        .route_layer(middleware::from_fn_with_state(state.clone(), oauth::middleware))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;
    Ok(axum::serve(listener, app).await?)
}
