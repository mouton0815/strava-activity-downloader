use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use serde::Deserialize;
use crate::AUTH_CALLBACK;
use crate::state::MutexSharedState;

pub mod client;
pub mod token;

// TODO: Move middleware to rest package
pub async fn middleware(State(state): State<MutexSharedState>, mut request: Request, next: Next) -> Result<Response, StatusCode> {
    debug!("[oauth middleware] Request URI: {}", request.uri());
    // Do no apply middleware to auth callback route
    if request.uri().path().starts_with(AUTH_CALLBACK) ||
        request.uri().path().starts_with("/toggle") { // TODO: Remove line
        debug!("[oauth middleware] Bypass middleware for auth callback");
        let response = next.run(request).await;
        debug!("[oauth middleware] Response status from next layer: {}", response.status());
        return Ok(response);
    }
    let mut guard = state.lock().await;
    match (*guard).oauth.get_bearer().await {
        Ok(bearer) => {
            match bearer {
                Some(bearer) => {
                    request.extensions_mut().insert(bearer);
                    drop(guard); // Inner layers might also want to obtain the mutex
                    debug!("[oauth middleware] Delegate to next layer");
                    let response = next.run(request).await;
                    debug!("[oauth middleware] Response status from next layer: {}", response.status());
                    Ok(response)
                }
                None => {
                    info!("[oauth middleware] No token, redirect to authorization endpoint");
                    let url = (*guard).oauth.authorize_auth_code_grant(request.uri());
                    debug!("[oauth middleware] Redirect to {}", url);
                    Ok(Redirect::temporary(url.as_str()).into_response())
                }
            }
        }
        Err(error) => {
            warn!("[oauth middleware] Error: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

#[debug_handler]
pub async fn callback(State(state): State<MutexSharedState>, query: Query<CallbackQuery>) -> Result<Redirect, StatusCode> {
    debug!("[oauth callback] Authorized with code {}", query.code);
    let mut guard = state.lock().await;
    match (*guard).oauth.callback_auth_code_grant(&query.code, &query.state).await {
        Ok(uri) => {
            debug!("[oauth callback] Redirect to origin URL: {}", uri);
            Ok(Redirect::temporary(uri.to_string().as_str()))
        }
        Err(_) => {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
