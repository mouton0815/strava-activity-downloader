use std::fs;
use std::io::BufWriter;
use std::path::Path;
use axum::BoxError;
use log::info;
use crate::domain::activity::Activity;
use crate::domain::activity_stream::ActivityStream;
use crate::util::gpx_path::gpx_path;

pub fn write_gpx(activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
    let data_path = gpx_path(activity)?;
    info!("Store GPX at {data_path}");
    let data_path = Path::new(&data_path);
    fs::create_dir_all(data_path.parent().unwrap())?;
    let gpx_file = fs::File::create(data_path)?;
    let buffer = BufWriter::new(gpx_file);
    stream.to_gpx(buffer, activity.id, &activity.name, &activity.start_date)?;
    Ok(())
}