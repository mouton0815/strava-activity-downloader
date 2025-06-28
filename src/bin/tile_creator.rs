use std::fs::File;
use std::io::BufReader;
use gpx::{Gpx, read, Track, TrackSegment};

// TODO: Use crate geo_types!!

fn main() {
    println!("Hello");
    // Iterate over activities by increasing start_date
    //   If an activity has gpx_fetched = 1 then
    //     load the corresponding GPX file (TODO: This is not possible at the moment, see https://docs.rs/gpx/latest/gpx/)
    //     generate and write the tiles for the corresponding activityID

    let file = File::open("/Users/torsten/git/strava-activity-downloader/data/2024/07/11936054836.gpx").unwrap();
    let reader = BufReader::new(file);

    // read takes any io::Read and gives a Result<Gpx, Error>.
    let gpx: Gpx = read(reader).unwrap();

    // Each GPX file has multiple "tracks", this takes the first one.
    let track: &Track = &gpx.tracks[0];
    assert_eq!(track.name, Some(String::from("Von der Bahn zum Hotel")));

    // Each track will have different segments full of waypoints, where a
    // waypoint contains info like latitude, longitude, and elevation.
    let segment: &TrackSegment = &track.segments[0];

    // This is an example of retrieving the elevation (in meters) at certain points.
    for point in &segment.points {
        println!("{:?}", point.point().x());
    }
    println!("{:?}", segment.points[0].elevation);
}