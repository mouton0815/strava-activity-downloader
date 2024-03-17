use std::fmt;
use std::fmt::Write;
use axum::BoxError;
use serde::Deserialize;
use crate::util::iso8601::{secs_to_string, string_to_secs};

#[derive(Deserialize)]
struct LatitudeLongitude {
    data: Vec<(f32,f32)>
}

#[derive(Deserialize)]
struct Altitude {
    data: Vec<f32>
}

// Distances are always included in the activity stream
#[derive(Deserialize)]
struct Distance {
    data: Vec<f32>
}

#[derive(Deserialize)]
struct Time {
    data: Vec<u32>
}

#[derive(Deserialize)]
pub struct ActivityStream {
    latlng: LatitudeLongitude,
    altitude: Altitude,
    distance: Distance,
    time: Time
}

impl ActivityStream {
    pub fn to_gpx(&self, activity_id: u64, activity_name: &str, start_time: &str) -> Result<String, BoxError> {
        if self.latlng.data.len() != self.time.data.len() ||
            self.time.data.len() != self.distance.data.len() ||
            self.distance.data.len() != self.altitude.data.len() {
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
}

impl fmt::Display for ActivityStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--->{}, {}, {}<---", self.latlng.data.len(), self.distance.data.len(), self.time.data.len())
    }
}

#[cfg(test)]
mod tests {
    use crate::ActivityStream;

    // Activity streams from java have additional fields like "series_type". They are ignored here.
    static INPUT: &str = r#"{
  "latlng":{"data":[[51.318165,12.375655],[51.318213,12.375588]],"series_type":"foo","original_size":1,"resolution":"bar"},
  "altitude":{"data":[123.456,100.0],"series_type":"foo","original_size":1,"resolution":"bar"},
  "distance":{"data":[0,3.7],"series_type":"foo","original_size":1,"resolution":"bar"},
  "time":{"data":[1,3],"series_type":"foo","original_size":1,"resolution":"bar"}
}"#;

    static REFERENCE: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
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
        <time>2024-01-01T00:00:01Z</time>
      </trkpt>
      <trkpt lat='51.318214' lon='12.375588'>
        <ele>100.0</ele>
        <time>2024-01-01T00:00:03Z</time>
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
        assert_eq!(result.unwrap(), REFERENCE);
    }
}