use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use axum::BoxError;
use strava_gpx_downloader::domain::activity_stream::ActivityStream;
use strava_gpx_downloader::service::activity_service::ActivityService;
use strava_gpx_downloader::util::gpx_path::gpx_path;

const ACTIVITY_DB: &'static str = "activity.db";

fn exec(mut service: ActivityService) -> Result<(), BoxError> {
    // Iterate over activities with GPX files by increasing start_date
    //   Load the corresponding GPX file
    //   Generate and write the tiles for the corresponding activity ID
    let activities = service.get_all_with_gpx()?;
    println!("Have {} activities", activities.len());
    for activity in activities {
        let data_path = gpx_path(&activity)?;
        println!("Read GPX from {data_path}");
        let data_path = Path::new(&data_path);
        let file = File::open(data_path)?;
        let reader = BufReader::new(file);
        let stream = ActivityStream::from_gpx(reader)?;
        service.put_tiles(&activity, &stream)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Generate tiles for older activities");
    let service = ActivityService::new(ACTIVITY_DB, true)?;
    exec(service).map_err(|e| e as Box<dyn Error>)
}