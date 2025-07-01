use axum::BoxError;
use log::info;
use strava_gpx_downloader::service::activity_service::ActivityService;
use strava_gpx_downloader::track::read_track::read_track;

const ACTIVITY_DB: &'static str = "activity.db";

fn main() -> Result<(), BoxError> {
    env_logger::init();
    println!("Generate tiles for older activities (use RUST_LOG=debug for more information)");
    let mut service = ActivityService::new(ACTIVITY_DB, true)?;
    let activities = service.get_all_with_gpx()?;
    info!("Have {} activities", activities.len());
    // Iterate over all activities with tracks by increasing start_date
    //   Load the corresponding track GPX file
    //   Generate and write the tiles for the corresponding activity
    for activity in activities {
        let stream = read_track(&activity)?;
        service.put_tiles(&activity, &stream)?;
    }
    Ok(())
}