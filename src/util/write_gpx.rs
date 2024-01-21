use std::{env, fs};
use std::path::Path;
use axum::BoxError;
use log::info;
use crate::ActivityStream;
use crate::domain::activity::Activity;

pub fn write_gpx(activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
    let id = &activity.id;
    let year = &activity.start_date[..4];
    let month = &activity.start_date[5..7];
    let gpx = stream.to_gpx(id.clone(), &activity.name, &activity.start_date)?;
    let data_path = format!("{}/data/{year}/{month}/{id}.gpx", env::var("CARGO_MANIFEST_DIR")?);
    info!("Store GPX at {data_path}");
    let data_path = Path::new(&data_path);
    fs::create_dir_all(data_path.parent().unwrap())?;
    fs::File::create(data_path)?;
    fs::write(data_path, gpx)?;
    Ok(())
}