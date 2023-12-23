use std::error::Error;
use axum::{Json, middleware, Router};
use axum::extract::Extension;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_macros::debug_handler;
use log::{debug, info};
use crate::oauth::client::{AUTH_CALLBACK, OAuthClient};
use crate::oauth::SharedState;
use crate::oauth::token::{Bearer, TokenHolder};

mod oauth;

const HOST : &'static str = "localhost";
const PORT : &'static str = "3000";

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
    /*
    let client = OAuthClient::new(HOST, PORT)?;
    let token = client.authorize_password_grant("fred", "fred").await?;
    auth::token::validate(token.token)?;
    */

    let shared_state = SharedState::new(OAuthClient::new(HOST, PORT)?);

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route(AUTH_CALLBACK, get(oauth::callback))
        .route_layer(middleware::from_fn_with_state(shared_state.clone(), oauth::middleware))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;
    Ok(axum::serve(listener, app).await?)
}
