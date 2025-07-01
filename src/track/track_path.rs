use std::env;
use axum::BoxError;
use crate::domain::activity::Activity;

/// Builds the path for the activity's track GPX file.
pub fn track_path(activity: &Activity) -> Result<String, BoxError> {
    let id = &activity.id;
    let year = &activity.start_date[..4];
    let month = &activity.start_date[5..7];
    let base_dir = env::var("CARGO_MANIFEST_DIR")?;
    Ok(format!("{base_dir}/data/{year}/{month}/{id}.gpx"))
}