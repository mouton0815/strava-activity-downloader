use iso8601_timestamp::Timestamp;

pub fn string_to_secs(str: &String) -> i64 {
    parse_internal(str).unwrap_or_else(|| panic!("Invalid timestamp: '{}'", str))
}

pub fn timestamp_to_secs(ts: Timestamp) -> i64 {
    ts.duration_since(Timestamp::UNIX_EPOCH).whole_seconds()
}

fn parse_internal(str: &str) -> Option<i64> {
    Timestamp::parse(str).map(timestamp_to_secs)
}

#[cfg(test)]
mod tests {
    use crate::util::iso8601::parse_internal;

    #[test]
    fn test_parse() {
        assert_eq!(parse_internal("2018-02-20T18:02:12Z"), Some(1519149732));
        assert_eq!(parse_internal("foo bar"), None);
    }
}