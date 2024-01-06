use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use serde::Deserialize;
use crate::AUTH_CALLBACK;
use crate::state::shared_state::MutexSharedState;

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

pub async fn middleware(State(state): State<MutexSharedState>, mut request: Request, next: Next) -> Result<Response, StatusCode> {
    debug!("Request URI: {}", request.uri());
    // Do no apply middleware to auth callback route
    if request.uri().path().starts_with(AUTH_CALLBACK) ||
        request.uri().path().starts_with("/status") || // TODO: Remove line
        request.uri().path().starts_with("/toggle") { // TODO: Remove line
    debug!("Bypass middleware for auth callback");
        let response = next.run(request).await;
        debug!("Response status from next layer: {}", response.status());
        return Ok(response);
    }
    let mut guard = state.lock().await;
    match (*guard).oauth.get_bearer().await {
        Ok(bearer) => {
            match bearer {
                Some(bearer) => {
                    request.extensions_mut().insert(bearer);
                    drop(guard); // Inner layers might also want to obtain the mutex
                    debug!("Delegate to next layer");
                    let response = next.run(request).await;
                    debug!("Response status from next layer: {}", response.status());
                    Ok(response)
                }
                None => {
                    info!("No token, redirect to authorization endpoint");
                    let url = (*guard).oauth.authorize_auth_code_grant(request.uri());
                    debug!("Redirect to {}", url);
                    Ok(Redirect::temporary(url.as_str()).into_response())
                }
            }
        }
        Err(error) => {
            warn!("Error: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[debug_handler]
pub async fn callback(State(state): State<MutexSharedState>, query: Query<CallbackQuery>) -> Result<Redirect, StatusCode> {
    debug!("Authorized with code {}", query.code);
    let mut guard = state.lock().await;
    match (*guard).oauth.callback_auth_code_grant(&query.code, &query.state).await {
        Ok(uri) => {
            debug!("Redirect to origin URL: {}", uri);
            Ok(Redirect::temporary(uri.to_string().as_str()))
        }
        Err(_) => {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

