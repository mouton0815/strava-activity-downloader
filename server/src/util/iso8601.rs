use iso8601_timestamp::{Duration, Timestamp, typenum};

pub fn string_to_secs(str: &str) -> i64 {
    parse_internal(str).unwrap_or_else(|| panic!("Invalid timestamp: '{str}'"))
}

pub fn secs_to_string(secs: i64) -> String {
    format_internal(secs).unwrap_or_else(|| panic!("Cannot convert {secs} secs to timestamp"))
}

fn timestamp_to_secs(ts: Timestamp) -> i64 {
    ts.duration_since(Timestamp::UNIX_EPOCH).whole_seconds()
}

fn timestamp_to_string(ts: Timestamp) -> String {
    ts.format_with_precision::<typenum::U0>().to_string()
}

fn parse_internal(str: &str) -> Option<i64> {
    Timestamp::parse(str).map(timestamp_to_secs)
}

fn format_internal(secs: i64) -> Option<String> {
    Timestamp::UNIX_EPOCH.checked_add(Duration::seconds(secs)).map(timestamp_to_string)
}

#[cfg(test)]
mod tests {
    use crate::util::iso8601::{format_internal, parse_internal, secs_to_string, string_to_secs};

    #[test]
    fn test_string_to_secs() {
        assert_eq!(string_to_secs("1970-01-01T00:00:00Z"), 0);
        assert_eq!(string_to_secs("2018-02-20T18:02:12Z"), 1519149732);
        assert_eq!(parse_internal("foo bar"), None);
    }

    #[test]
    fn test_secs_to_string() {
        assert_eq!(secs_to_string(0), "1970-01-01T00:00:00Z");
        assert_eq!(secs_to_string(1519149732), "2018-02-20T18:02:12Z");
        assert_eq!(format_internal(i64::MIN), None);
    }
}