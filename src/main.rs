use std::error::Error;
use axum::{Json, middleware, Router};
use axum::extract::Extension;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_macros::debug_handler;
use log::{debug, info};
use crate::oauth::client::{AUTH_CALLBACK, OAuthClient};
use crate::oauth::OAuthState;
use crate::oauth::token::{Bearer, TokenHolder};

mod oauth;

const HOST : &'static str = "localhost";
const PORT : &'static str = "3000";

const CLIENT_ID : &'static str = "unite-client";
const CLIENT_SECRET : &'static str = "totally-secret";
const AUTH_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/auth";
const TOKEN_URL : &'static str = "http://localhost:8080/realms/unite/protocol/openid-connect/token";

#[debug_handler]
async fn retrieve(Extension(bearer): Extension<Bearer>) -> Response {
    info!("--r--> Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--r--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);
    // TODO: Do something useful
    Json("foo bar").into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();

    let client = OAuthClient::new(HOST, PORT, CLIENT_ID, CLIENT_SECRET, AUTH_URL, TOKEN_URL)?;
    let state = OAuthState::new(client);

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route(AUTH_CALLBACK, get(oauth::callback))
        .route_layer(middleware::from_fn_with_state(state.clone(), oauth::middleware))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;
    Ok(axum::serve(listener, app).await?)
}
