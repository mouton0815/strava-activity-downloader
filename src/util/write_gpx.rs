use std::fs;
use std::path::Path;
use axum::BoxError;
use crate::ActivityStream;

pub fn write_gpx(activity_id: u64, activity_name: &str, start_time: &str, stream: &ActivityStream) -> Result<(), BoxError> {
    let gpx = stream.to_gpx(activity_id, activity_name, start_time)?;
    let year = &start_time[..4];
    let month = &start_time[5..7];
    let data_path = format!("{}/data/{year}/{month}/{activity_id}.gpx", std::env::var("CARGO_MANIFEST_DIR")?);
    println!("{data_path}");
    let data_path = Path::new(&data_path);
    fs::create_dir_all(data_path.parent().unwrap())?;
    fs::File::create(data_path)?;
    fs::write(data_path, gpx)?;
    Ok(())
}