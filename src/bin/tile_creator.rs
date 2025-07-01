use std::error::Error;
use axum::BoxError;
use strava_gpx_downloader::service::activity_service::ActivityService;
use strava_gpx_downloader::track::read_track::read_track;

const ACTIVITY_DB: &'static str = "activity.db";

fn exec(mut service: ActivityService) -> Result<(), BoxError> {
    // Iterate over all activities with tracks by increasing start_date
    //   Load the corresponding track GPX file
    //   Generate and write the tiles for the corresponding activity
    let activities = service.get_all_with_gpx()?;
    println!("Have {} activities", activities.len());
    for activity in activities {
        let stream = read_track(&activity)?;
        service.put_tiles(&activity, &stream)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Generate tiles for older activities");
    let service = ActivityService::new(ACTIVITY_DB, true)?;
    exec(service).map_err(|e| e as Box<dyn Error>)
}