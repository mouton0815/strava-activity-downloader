use axum::BoxError;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use serde::Deserialize;
use crate::Bearer;
use crate::state::shared_state::MutexSharedState;

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

async fn get_bearer(state: &MutexSharedState) -> Result<Option<Bearer>, BoxError> {
    let mut guard = state.lock().await;
    (*guard).oauth.get_bearer().await
}

pub async fn authorize(State(state): State<MutexSharedState>) -> Result<Response, StatusCode> {
    let mut guard = state.lock().await;
    match (*guard).oauth.get_bearer().await {
        Ok(bearer) => {
            match bearer {
                Some(_) => {
                    Ok("authorized".into_response())
                }
                None => {
                    info!("No token, redirect to authorization endpoint");
                    let url = (*guard).oauth.authorize_auth_code_grant();
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

pub async fn middleware(State(state): State<MutexSharedState>, mut request: Request, next: Next) -> Result<Response, StatusCode> {
    debug!("Request URI: {}", request.uri());
    if true { // request.uri().path().starts_with(AUTH_CALLBACK)
        debug!("Bypass middleware");
        let response = next.run(request).await;
        debug!("Response status from next layer: {}", response.status());
        return Ok(response);
    }
    match get_bearer(&state).await {
        Ok(bearer) => {
            match bearer {
                Some(bearer) => {
                    request.extensions_mut().insert(bearer);
                    debug!("Delegate to next layer");
                    let response = next.run(request).await;
                    debug!("Response status from next layer: {}", response.status());
                    Ok(response)
                }
                None => {
                    info!("No token, return 401 Unauthorized");
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        }
        Err(error) => {
            warn!("Error: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
