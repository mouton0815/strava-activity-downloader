use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Write;
use std::io::Read;
use axum::BoxError;
use serde::Deserialize;
use gpx::{Gpx, read, Track, TrackSegment};
use iso8601_timestamp::time::OffsetDateTime;
use crate::domain::map_tile::MapTile;
use crate::util::iso8601::{secs_to_string, string_to_secs};

type LatLon = (f64, f64); // TODO: Use crate geo_types!!

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
            coords.push((point.point().y(), point.point().x())); // TODO: Push point directly once change to geo_types is done
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

    pub fn to_gpx(&self, activity_id: u64, activity_name: &str, start_time: &str) -> Result<String, BoxError> {
        if self.latlng.data.len() != self.time.data.len() ||
            self.time.data.len() != self.altitude.data.len() {
            Err("Streams have different lengths".into())
        } else {
            let start_time = string_to_secs(start_time);
            self.to_gpx_internal(activity_id, activity_name, start_time).map_err(|_| "Formatting error".into())
        }
    }

    fn to_gpx_internal(&self, activity_id: u64, activity_name: &str, start_time: i64) -> Result<String, fmt::Error> {
        // Escape name according to https://stackoverflow.com/questions/21758345/what-are-the-official-xml-reserved-characters
        let name = activity_name.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
        let mut s = String::new();
        writeln!(&mut s, "<?xml version='1.0' encoding='UTF-8'?>")?;
        writeln!(&mut s, "<gpx xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' xmlns='http://www.topografix.com/GPX/1/1' xsi:schemaLocation='http://www.topografix.com/GPX/1/1 http://www.topografix.com/GPX/1/1/gpx.xsd' version='1.1' creator='http://strava.com/'>")?;
        writeln!(&mut s, "  <metadata>")?;
        writeln!(&mut s, "    <name>{}</name>", name)?;
        writeln!(&mut s, "    <link href='https://www.strava.com/api/v3/activities/{}'>", activity_id)?;
        writeln!(&mut s, "      <text>{}</text>", name)?;
        writeln!(&mut s, "    </link>")?;
        writeln!(&mut s, "  </metadata>")?;
        writeln!(&mut s, "  <trk>")?;
        writeln!(&mut s, "    <name>{}</name>", name)?;
        writeln!(&mut s, "    <trkseg>")?;
        for i in 0..self.latlng.data.len() {
            let (lat, lon) = &self.latlng.data[i.clone()];
            let altitude = &self.altitude.data[i.clone()];
            let time = self.time.data[i].clone() as i64;
            writeln!(&mut s, "      <trkpt lat='{}' lon='{}'>", lat, lon)?;
            writeln!(&mut s, "        <ele>{:?}</ele>", altitude)?;
            writeln!(&mut s, "        <time>{}</time>", secs_to_string(start_time + time))?;
            writeln!(&mut s, "      </trkpt>")?;
        }
        writeln!(&mut s, "    </trkseg>")?;
        writeln!(&mut s, "  </trk>")?;
        writeln!(&mut s, "</gpx>")?;
        Ok(s)
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
    static INPUT: &str = r#"{
  "latlng":{"data":[[51.318165,12.375655],[51.318213,12.395588],[51.318213,12.375588]]},
  "altitude":{"data":[123.456,120.0,100.0]},
  "distance":{"data":[0,1.3,3.7]},
  "time":{"data":[0,3,7]}
}"#;

    static GPX_REF: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<gpx xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' xmlns='http://www.topografix.com/GPX/1/1' xsi:schemaLocation='http://www.topografix.com/GPX/1/1 http://www.topografix.com/GPX/1/1/gpx.xsd' version='1.1' creator='http://strava.com/'>
  <metadata>
    <name>Foo Bar</name>
    <link href='https://www.strava.com/api/v3/activities/12345'>
      <text>Foo Bar</text>
    </link>
  </metadata>
  <trk>
    <name>Foo Bar</name>
    <trkseg>
      <trkpt lat='51.318165' lon='12.375655'>
        <ele>123.456</ele>
        <time>2024-01-01T00:00:00Z</time>
      </trkpt>
      <trkpt lat='51.318213' lon='12.395588'>
        <ele>120.0</ele>
        <time>2024-01-01T00:00:03Z</time>
      </trkpt>
      <trkpt lat='51.318213' lon='12.375588'>
        <ele>100.0</ele>
        <time>2024-01-01T00:00:07Z</time>
      </trkpt>
    </trkseg>
  </trk>
</gpx>
"#;

    #[test]
    fn test_to_gpx() {
        let stream : serde_json::Result<ActivityStream> = serde_json::from_str(INPUT);
        assert!(stream.is_ok());
        let result = stream.unwrap().to_gpx(12345, "Foo Bar", "2024-01-01T00:00:00Z");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), GPX_REF);
    }

    #[test]
    fn test_from_gpx() {
        let reader = Cursor::new(GPX_REF.as_bytes());
        let result = ActivityStream::from_gpx(reader);
        assert!(result.is_ok());

        let reference = ActivityStream::new(
            vec![(51.318165,12.375655),(51.318213,12.395588),(51.318213,12.375588)],
            vec![123.456,120.0,100.0],
            vec![0,3,7]
        );
        assert_eq!(result.unwrap(), reference);
    }

    #[test]
    fn test_to_tiles() {
        let stream : serde_json::Result<ActivityStream> = serde_json::from_str(INPUT);
        assert!(stream.is_ok());
        let result = stream.unwrap().to_tiles(14);
        assert!(result.is_ok());
        let reference = vec!(MapTile::new(8755, 5461), MapTile::new(8756, 5461));
        assert_eq!(result.unwrap(), reference);
    }
}