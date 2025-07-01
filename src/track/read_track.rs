use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use axum::BoxError;
use log::info;
use crate::domain::activity::Activity;
use crate::domain::activity_stream::ActivityStream;
use crate::track::track_path::track_path;

pub fn read_track(activity: &Activity) -> Result<ActivityStream, BoxError> {
    let path = track_path(&activity)?;
    info!("Read track from {path}");
    let path = Path::new(&path);
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    ActivityStream::from_gpx(reader)
}