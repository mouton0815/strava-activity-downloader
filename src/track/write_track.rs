use std::fs;
use std::io::BufWriter;
use std::path::Path;
use axum::BoxError;
use log::info;
use crate::domain::activity::Activity;
use crate::domain::activity_stream::ActivityStream;
use crate::track::track_path::track_path;

pub fn write_track(activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
    let path = track_path(activity)?;
    info!("Write track to {path}");
    let path = Path::new(&path);
    fs::create_dir_all(path.parent().unwrap())?;
    let file = fs::File::create(path)?;
    let writer = BufWriter::new(file);
    stream.to_gpx(writer, activity.id, &activity.name, &activity.start_date)?;
    Ok(())
}