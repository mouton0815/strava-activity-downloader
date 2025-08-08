use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use serde::Deserialize;
use crate::state::shared_state::MutexSharedState;

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

pub async fn authorize_handler(State(state): State<MutexSharedState>) -> Result<Response, StatusCode> {
    let mut guard = state.lock().await;
    match guard.oauth.get_bearer().await {
        Ok(bearer) => {
            match bearer {
                Some(_) => {
                    Ok("authorized".into_response())
                }
                None => {
                    info!("No token, redirect to authorization endpoint");
                    let url = guard.oauth.authorize_auth_code_grant();
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
pub async fn callback_handler(State(state): State<MutexSharedState>, query: Query<CallbackQuery>) -> Result<Redirect, StatusCode> {
    debug!("Authorized with code {}", query.code);
    let mut guard = state.lock().await;
    match guard.oauth.callback_auth_code_grant(&query.code, &query.state).await {
        Ok(uri) => {
            debug!("Redirect to origin URL: {}", uri);
            Ok(Redirect::temporary(uri.to_string().as_str()))
        }
        Err(_) => {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
