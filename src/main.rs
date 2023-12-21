use std::error::Error;
use std::sync::Arc;
use axum::http::{StatusCode, Uri};
use axum::{Json, middleware, Router};
use axum::extract::{Extension, Query, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum_macros::debug_handler;
use log::{info, warn};
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;
use crate::auth::client::OAuthClient;
use crate::auth::token::{Bearer, TokenHolder};

mod auth;

const HOST : &'static str = "localhost";
const PORT : &'static str = "3000";

const AUTH_CALLBACK : &'static str = "/auth_callback";

// TODO: Note: Middleware can also return Result<Response, StatusCode> (or similar?)
async fn auth_middleware(State(state): State<MutexState>, mut request: Request, next: Next) -> Response {
    info!("--m--> Request URI: {}", request.uri());
    // Do no apply middleware to auth callback route
    if request.uri().path().starts_with(AUTH_CALLBACK) {
        info!("--m--> Bypass middleware for auth callback: {}", request.uri());
        let response = next.run(request).await;
        info!("--m--> Response status: {}", response.status());
        return response;
    }
    let mut guard = state.lock().await;
    match &(*guard).token {
        Some(token_holder) => {
            info!("--m--> Token found");
            if auth::token::is_expired(token_holder) {
                match (*guard).client.refresh_token(token_holder).await {
                    Ok(token) => {
                        (*guard).token = Some(token);
                        // TODO: Drop guard here?
                    }
                    Err(error) => {
                        return to_internal_server_error(error).into_response();
                    }
                }
            }
            let bearer = (*guard).token.as_ref().expect("Missing token").bearer();
            request.extensions_mut().insert(bearer);
            drop(guard); // Inner layers might also want to obtain the mutex
            info!("--m--> Delegate to next layer");
            let response = next.run(request).await;
            info!("--m--> Response status: {}", response.status());
            response
        }
        None => {
            info!("--m--> NO token, build authorization URL");
            match (*guard).client.authorize_auth_code_grant() {
                Ok((url, csrf_token)) => {
                    info!("--m--> Redirect to authorization URL: {}", url);
                    (*guard).state = Some(csrf_token.secret().clone());
                    (*guard).origin = Some(request.uri().clone());
                    Redirect::temporary(url.as_str()).into_response()
                }
                Err(error) => {
                    to_internal_server_error(error).into_response()
                }
            }
        }
    }
}

#[derive(Serialize, Debug, Eq, PartialEq)]
struct ErrorResult {
    error: String
}

type RestError = (StatusCode, Json<ErrorResult>);

fn to_internal_server_error(error: Box<dyn Error>) -> RestError {
    warn!("Error: {}", error);
    let message = ErrorResult { error: error.to_string() };
    (StatusCode::INTERNAL_SERVER_ERROR, Json(message))
}

#[debug_handler]
async fn retrieve(Extension(bearer): Extension<Bearer>) -> Response {
    info!("--r--> Enter /retrieve");
    let bearer : String = bearer.into();
    info!("--r--> {}", bearer);
    // TODO: Do something useful
    Json("foo bar").into_response()
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String
}

async fn auth_callback(State(state): State<MutexState>, query: Query<CallbackQuery>) -> Result<Redirect, RestError> {
    info!("--c--> Authorized with code {}", query.code);
    let mut guard = state.lock().await;
    if (*guard).origin.is_none()
        || (*guard).state.is_none()
        || (*guard).state.as_ref().unwrap() != &query.state {
        warn!("Received state {} does not match expected state {}", query.state,
            (*guard).state.as_ref().unwrap_or(&String::from("<null>")));
        return Err(to_internal_server_error("Internal error".into()))
    }
    match (*guard).client.exchange_code_for_token(&query.code).await {
        Ok(token) => {
            let uri = (*guard).origin.as_ref().unwrap().to_string();
            (*guard).token = Some(token);
            (*guard).state = None;
            (*guard).origin = None;
            info!("--c--> Redirect to origin URL: {}", uri);
            Ok(Redirect::temporary(uri.as_str()))
        }
        Err(error) => {
            warn!("Error: {}", error);
            let message = ErrorResult{ error: error.to_string() };
            Err((StatusCode::UNAUTHORIZED, Json(message)))
        }
    }
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
    let client = OAuthClient::new()?;
    let token = client.authorize_password_grant("fred", "fred").await?;
    auth::token::validate(token.token)?;

    let shared_state = Arc::new(Mutex::new(SharedState {
        client: OAuthClient::new()?,
        state: None,
        origin: None,
        token: None
    }));

    let app = Router::new()
        .route("/retrieve", get(retrieve))
        .route(AUTH_CALLBACK, get(auth_callback))
        .route_layer(middleware::from_fn_with_state(shared_state.clone(), auth_middleware))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", HOST, PORT)).await?;
    Ok(axum::serve(listener, app).await?)
}
