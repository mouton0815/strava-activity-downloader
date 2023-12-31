use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_macros::debug_handler;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use crate::{Bearer, MutexSharedState};

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
    distance: f64,
    kudos_count: u64
}

type Activities = Vec<Activity>;

#[debug_handler]
pub async fn retrieve(State(mut _state): State<MutexSharedState>, Extension(bearer): Extension<Bearer>) -> Result<Response, StatusCode> {
    info!("Enter /retrieve");
    let bearer : String = bearer.into();
    debug!("--b--> {}", &bearer.as_str()[..std::cmp::min(100, bearer.as_str().len())]);
    /*
    // let query = vec![("after", "1701388800")];
    let result = reqwest::Client::new()
        .get("https://www.strava.com/api/v3/athlete/activities")
        .header(reqwest::header::AUTHORIZATION, bearer)
        //.query(&query)
        .send().await.map_err(log_error)?
        .error_for_status().map_err(log_error)?
        .json::<Activities>().await.map_err(log_error)?;

    info!("--r--> {:?}", result);
    Ok(Json(result).into_response())
    */
    Ok("Hallo Welt".into_response())
}

#[debug_handler]
pub async fn toggle(State(state): State<MutexSharedState>) -> Result<Response, StatusCode> {
    info!("Enter /toggle");
    let mut guard = state.lock().await;
    let old_value = (*guard).scheduler_running.clone();
    (*guard).scheduler_running = !old_value;
    Ok(old_value.to_string().into_response())
}
