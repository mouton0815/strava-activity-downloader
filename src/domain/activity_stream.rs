use std::collections::BTreeSet;
use std::fmt;
use std::io::{Read, Write};
use axum::BoxError;
use geo_types::Point;
use serde::Deserialize;
use gpx::{Gpx, GpxVersion, Link, Metadata, read, Track, TrackSegment, Waypoint};
use iso8601_timestamp::time::OffsetDateTime;
use crate::domain::map_tile::MapTile;
use crate::util::iso8601::string_to_secs;

// Note: Cannot use geo_types::Point because it expects an object serialization
// format { x: lon, y: lat } whereas Strava delivers an array [lat, lon].
type LatLon = (f64, f64);

#[derive(Debug, Deserialize, PartialEq)]
struct LatLonVec {
    data: Vec<LatLon>
}

#[derive(Debug, Deserialize, PartialEq)]
struct AltitudeVec {
    data: Vec<f64>
}

#[derive(Debug, Deserialize, PartialEq)]
struct TimeVec {
    data: Vec<u32>
}

/// An activity stream as returned by Strava https://developers.strava.com/docs/reference/#api-Streams-getActivityStreams.
/// There are three ways to construct it:
/// * By deserializing the JSON response of the Strava API
/// * By creating it from a GPX file with [ActivityStream::from_gpx]
/// * By calling [ActivityStream::new]
#[derive(Debug, Deserialize, PartialEq)]
pub struct ActivityStream {
    latlng: LatLonVec,
    altitude: AltitudeVec,
    time: TimeVec
}

impl ActivityStream {
    /// Creates an activity stream from coordinates and leaves all other arrays empty (for testing).
    pub fn new(coords: Vec<LatLon>, altitudes: Vec<f64>, times: Vec<u32>) -> Self {
        ActivityStream {
            latlng: LatLonVec{ data: coords },
            altitude: AltitudeVec { data: altitudes },
            time: TimeVec { data: times }
        }
    }

    pub fn from_gpx<R: Read>(reader: R) -> Result<Self, BoxError> {
        let gpx: Gpx = read(reader)?;
        let track: &Track = &gpx.tracks[0];
        let segment: &TrackSegment = &track.segments[0];
        let mut coords: Vec<LatLon> = vec![];
        let mut times: Vec<u32> = vec![];
        let mut altitudes: Vec<f64> = vec![];
        let mut start_time: Option<i64> = None;
        for point in &segment.points {
            coords.push((point.point().y(), point.point().x()));
            altitudes.push(point.elevation.unwrap_or(0.0));
            if let Some(time) = point.time {
                let curr_time = OffsetDateTime::from(time).unix_timestamp();
                match start_time {
                    Some(start_time) => {
                        times.push((curr_time - start_time) as u32)
                    },
                    None => {
                        times.push(0);
                        start_time = Some(curr_time);
                    }
                }
            }
        }
        let stream = ActivityStream {
          latlng: LatLonVec { data: coords },
          altitude: AltitudeVec { data: altitudes },
          time: TimeVec { data: times }
        };
        Ok(stream)
    }

    pub fn to_gpx<W: Write>(&self, writer: W, activity_id: u64, activity_name: &str, start_time: &str) -> Result<(), BoxError> {
        if self.latlng.data.len() != self.time.data.len() ||
            self.time.data.len() != self.altitude.data.len() {
            return Err("Streams have different lengths".into());
        }
        // Escape name according to https://stackoverflow.com/questions/21758345/what-are-the-official-xml-reserved-characters
        let name = activity_name.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
        let start_time = string_to_secs(start_time);
        let mut points: Vec<Waypoint> = Vec::new();
        for i in 0..self.latlng.data.len() {
            let (lat, lon) = &self.latlng.data[i];
            let altitude = &self.altitude.data[i];
            let time = start_time + self.time.data[i] as i64;
            let time= OffsetDateTime::from_unix_timestamp(time).unwrap();
            let mut point = Waypoint::new(Point::new(*lon, *lat));
            point.elevation = Some(*altitude);
            point.time = Some(time.into());
            points.push(point);
        }
        let track_segment = TrackSegment {
            points
        };
        let track = Track {
            name: Some(name.clone()),
            comment: None,
            description: None,
            source: None,
            links: vec![],
            type_: None,
            number: None,
            segments: vec![track_segment]
        };
        let metadata = Metadata {
            name: Some(name.clone()),
            description: None,
            author: None,
            links: vec![Link {
                href: format!("https://www.strava.com/api/v3/activities/{}", activity_id),
                text: Some(name),
                type_ : None
            }],
            time: None,
            keywords: None,
            copyright: None,
            bounds: None,
        };
        let gpx = Gpx {
            version: GpxVersion::Gpx11,
            creator: Some("http://strava.com/".to_string()),
            metadata: Some(metadata),
            waypoints: vec![],
            tracks: vec![track],
            routes: vec![],
        };
        gpx::write(&gpx, writer)?;
        Ok(())
    }

    /// Returns the list of unique [MapTile]s touched by this activity stream.
    /// The returned list is sorted and does not contain duplicate tiles.
    pub fn to_tiles(&self, zoom: u16) -> Result<Vec<MapTile>, BoxError> {
        let coords: &Vec<(f64, f64)> = self.latlng.data.as_ref();
        let tiles = coords
            .into_iter()
            .map(|(lat, lon)| MapTile::from_coords(*lat, *lon, zoom))
            .collect::<BTreeSet<_>>()// Collect into set to remove duplicates
            .into_iter()
            .collect();
        Ok(tiles)
    }
}

impl fmt::Display for ActivityStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--->{}, {}<---", self.latlng.data.len(), self.time.data.len())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::ActivityStream;
    use crate::domain::map_tile::MapTile;

    // Activity streams from java have additional fields like "series_type". They are ignored here.
    static STREAM_STR: &str = r#"{
  "latlng":{"data":[[51.318165,12.375655],[51.318213,12.395588],[51.318213,12.375588]]},
  "altitude":{"data":[123.456,120.0,100.0]},
  "distance":{"data":[0,1.3,3.7]},
  "time":{"data":[0,3,7]}
}"#;

    static GPX_STR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" xmlns="http://www.topografix.com/GPX/1/1" creator="http://strava.com/">
  <metadata>
    <name>Foo Bar</name>
    <link href="https://www.strava.com/api/v3/activities/12345">
      <text>Foo Bar</text>
    </link>
  </metadata>
  <trk>
    <name>Foo Bar</name>
    <trkseg>
      <trkpt lat="51.318165" lon="12.375655">
        <ele>123.456</ele>
        <time>2024-01-01T00:00:00.000000000Z</time>
      </trkpt>
      <trkpt lat="51.318213" lon="12.395588">
        <ele>120</ele>
        <time>2024-01-01T00:00:03.000000000Z</time>
      </trkpt>
      <trkpt lat="51.318213" lon="12.375588">
        <ele>100</ele>
        <time>2024-01-01T00:00:07.000000000Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>"#;

    // vec! cannot be static or const, so use a function here
    fn get_stream() -> ActivityStream {
        ActivityStream::new(
            vec![(51.318165,12.375655),(51.318213,12.395588),(51.318213,12.375588)],
            vec![123.456,120.0,100.0],
            vec![0,3,7]
        )
    }

    #[test]
    fn test_deserialize() {
        let result: serde_json::Result<ActivityStream> = serde_json::from_str(STREAM_STR);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), get_stream());
    }

    #[test]
    fn test_to_gpx() {
        let stream = get_stream();
        let mut buffer: Vec<u8> = Vec::new();
        assert!(stream.to_gpx(&mut buffer, 12345, "Foo Bar", "2024-01-01T00:00:00Z").is_ok());
        let result = String::from_utf8(buffer);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), GPX_STR);
    }

    #[test]
    fn test_from_gpx() {
        let reader = Cursor::new(GPX_STR.as_bytes());
        let result = ActivityStream::from_gpx(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), get_stream());
    }

    #[test]
    fn test_to_tiles() {
        let stream : serde_json::Result<ActivityStream> = serde_json::from_str(STREAM_STR);
        assert!(stream.is_ok());
        let result = stream.unwrap().to_tiles(14);
        assert!(result.is_ok());
        let reference = vec!(MapTile::new(8755, 5461), MapTile::new(8756, 5461));
        assert_eq!(result.unwrap(), reference);
    }
}