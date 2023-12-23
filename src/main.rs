use std::error::Error;
use std::sync::Arc;
use axum::http::{StatusCode, Uri};
use axum::{Json, middleware, Router};
use axum::extract::{Extension, Query, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_macros::debug_handler;
use log::{debug, info, warn};
use serde::Deserialize;
use tokio::sync::Mutex;
use crate::auth::client::{AUTH_CALLBACK, OAuthClient};
use crate::auth::token::{Bearer, TokenHolder};

mod auth;

const HOST : &'static str = "localhost";
const PORT : &'static str = "3000";

async fn oauth_middleware(State(state): State<MutexState>, mut request: Request, next: Next) -> Result<Response, StatusCode> {
    debug!("[oauth middleware] Request URI: {}", request.uri());
    // Do no apply middleware to auth callback route
    if request.uri().path().starts_with(AUTH_CALLBACK) {
        debug!("[oauth middleware] Bypass middleware for auth callback");
        let response = next.run(request).await;
        debug!("[oauth middleware] Response status from next layer: {}", response.status());
        return Ok(response);
    }
    let mut guard = state.lock().await;
    match &(*guard).token {
        Some(token_holder) => {
            debug!("[oauth middleware] Token found");
            if auth::token::is_expired(token_holder) {
                match (*guard).client.refresh_token(token_holder).await {
                    Ok(token) => {
                        (*guard).token = Some(token);
                    }
                    Err(error) => {
                        warn!("[oauth middleware] Error: {}", error);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
            let bearer = (*guard).token.as_ref().expect("Missing token").bearer();
            request.extensions_mut().insert(bearer);
            drop(guard); // Inner layers might also want to obtain the mutex
            debug!("[oauth middleware] Delegate to next layer");
            let response = next.run(request).await;
            debug!("[oauth middleware] Response status from next layer: {}", response.status());
            Ok(response)
        }
        None => {
            info!("[oauth middleware] NO token, redirect to authorization endpoint");
            match (*guard).client.authorize_auth_code_grant() {
                Ok((url, csrf_token)) => {
                    debug!("[oauth middleware] Redirect to {}", url);
                    (*guard).state = Some(csrf_token.secret().clone());
                    (*guard).origin = Some(request.uri().clone());
                    Ok(Redirect::temporary(url.as_str()).into_response())
                }
                Err(error) => {
                    warn!("[oauth middleware] Error: {}", error);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String
}

async fn auth_callback(State(state): State<MutexState>, query: Query<CallbackQuery>) -> Result<Redirect, StatusCode> {
    debug!("[oauth callback] Authorized with code {}", query.code);
    let mut guard = state.lock().await;
    if (*guard).origin.is_none()
        || (*guard).state.is_none()
        || (*guard).state.as_ref().unwrap() != &query.state {
        warn!("[oauth callback] Received state {} does not match expected state {}", query.state,
            (*guard).state.as_ref().unwrap_or(&String::from("<null>")));
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    //Err(StatusCode::INSUFFICIENT_STORAGE)
    match (*guard).client.exchange_code_for_token(&query.code).await {
        Ok(token) => {
            let uri = (*guard).origin.as_ref().unwrap().to_string();
            (*guard).token = Some(token);
            (*guard).state = None;
            (*guard).origin = None;
            debug!("[oauth callback] Redirect to origin URL: {}", uri);
            Ok(Redirect::temporary(uri.as_str()))
        }
        Err(error) => {
            warn!("[oauth callback] Error: {:?}", error);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[debug_handler]
async fn retrieve(Extension(bearer): Extension<Bearer>) -> Response {
    info!("--r--> Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--r--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);
    // TODO: Do something useful
    Json("foo bar").into_response()
}

// #[derive(Clone)]
struct SharedState {
    client: OAuthClient,
    state: Option<String>,
    origin: Option<Uri>, // REST endpoint that triggered the authentication
    token: Option<TokenHolder>
}

type MutexState = Arc<Mutex<SharedState>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    env_logger::init();
    /*
    let client = OAuthClient::new(HOST, PORT)?;
    let token = client.authorize_password_grant("fred", "fred").await?;
    auth::token::validate(token.token)?;
    */

    let shared_state = Arc::new(Mutex::new(SharedState {
        client: OAuthClient::new(HOST, PORT)?,
        state: None,
        origin: None,
        token: None
    }));

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route(AUTH_CALLBACK, get(auth_callback))
        .route_layer(middleware::from_fn_with_state(shared_state.clone(), oauth_middleware))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;
    Ok(axum::serve(listener, app).await?)
}
