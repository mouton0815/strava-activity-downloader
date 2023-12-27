use std::sync::Arc;
use axum::extract::{Query, Request, State};
use axum::http::{StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use serde::Deserialize;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use crate::{AUTH_CALLBACK, OAuthClient, SchedulerCommand, TokenHolder};

pub mod client;
pub mod token;

pub struct OAuthState {
    client: OAuthClient,
    state: Option<String>,
    origin: Option<Uri>, // REST endpoint that triggered the authentication
    token: Option<TokenHolder>
}

impl OAuthState {
    pub fn new(client: OAuthClient) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            client,
            state: None,
            origin: None,
            token: None
        }))
    }
}

//type MutexState = Arc<Mutex<OAuthState>>;

#[derive(Clone)]
pub struct FullState {
    oauth: Arc<Mutex<OAuthState>>,
    pub sender: Sender<SchedulerCommand> // TODO: Remove pub and implement send() method
}

impl FullState {
    pub fn new(oauth: Arc<Mutex<OAuthState>>, sender: Sender<SchedulerCommand>) -> Self {
        Self { oauth, sender }
    }
}

//#[debug_handler]
pub async fn middleware(State(state): State<FullState>, mut request: Request, next: Next) -> Result<Response, StatusCode> {
    debug!("[oauth middleware] Request URI: {}", request.uri());
    // Do no apply middleware to auth callback route
    if request.uri().path().starts_with(AUTH_CALLBACK) ||
        request.uri().path().starts_with("/toggle") { // TODO: Remove line
        debug!("[oauth middleware] Bypass middleware for auth callback");
        let response = next.run(request).await;
        debug!("[oauth middleware] Response status from next layer: {}", response.status());
        return Ok(response);
    }
    let mut guard = state.oauth.lock().await;
    match &(*guard).token {
        Some(token_holder) => {
            debug!("[oauth middleware] Token found");
            if token::is_expired(token_holder) {
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
            info!("[oauth middleware] No token, redirect to authorization endpoint");
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
pub struct CallbackQuery {
    code: String,
    state: String,
}

#[debug_handler]
pub async fn callback(State(state): State<FullState>, query: Query<CallbackQuery>) -> Result<Redirect, StatusCode> {
    debug!("[oauth callback] Authorized with code {}", query.code);
    let mut guard = state.oauth.lock().await;
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
