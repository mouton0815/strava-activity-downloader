use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use axum::BoxError;
use log::info;
use crate::domain::activity::Activity;
use crate::domain::activity_stream::ActivityStream;

pub struct TrackStorage {
    base_path: String
}

impl TrackStorage {
    pub fn new(base_path: &str) -> Self {
        Self { base_path: base_path.to_string() }
    }

    pub fn read(&self, activity: &Activity) -> Result<ActivityStream, BoxError> {
        let path = self.get_path(activity)?;
        info!("Read track from {path}");
        let path = Path::new(&path);
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        ActivityStream::from_gpx(reader)
    }

    pub fn write(&self, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
        let path = self.get_path(activity)?;
        info!("Write track to {path}");
        let path = Path::new(&path);
        fs::create_dir_all(path.parent().unwrap())?;
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        stream.to_gpx(writer, activity.id, &activity.name, &activity.start_date)?;
        Ok(())
    }

    fn get_path(&self, activity: &Activity) -> Result<String, BoxError> {
        let id = &activity.id;
        let year = &activity.start_date[..4];
        let month = &activity.start_date[5..7];
        Ok(format!("{}/{year}/{month}/{id}.gpx", self.base_path))
    }
}