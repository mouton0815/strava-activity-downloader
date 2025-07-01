use axum::BoxError;
use strava_gpx_downloader::service::activity_service::ActivityService;
use strava_gpx_downloader::track::read_track::read_track;

const ACTIVITY_DB: &'static str = "activity.db";

fn main() -> Result<(), BoxError> {
    env_logger::init();
    println!("Generate tiles for older activities (use RUST_LOG=debug for more information)");
    let mut service = ActivityService::new(ACTIVITY_DB, true)?;
    // Delete all existing tiles (otherwise the ID of the first activity would be wrong)
    service.delete_all_tiles()?;
    // Iterate over all activities with tracks by increasing start_date
    for activity in service.get_all_with_gpx()? {
        // Load the corresponding track GPX file
        let stream = read_track(&activity)?;
        // Generate and write the tiles for the corresponding activity
        service.put_tiles(&activity, &stream)?;
    }
    Ok(())
}