use iso8601_timestamp::Timestamp;

pub fn string_to_secs(str: String) -> i64 {
    parse_internal(&str).unwrap_or_else(|| panic!("Invalid timestamp: '{}'", str))
}

pub fn timestamp_to_secs(ts: Timestamp) -> i64 {
    ts.duration_since(Timestamp::UNIX_EPOCH).whole_seconds()
}

pub fn min_secs<T: Copy + Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    // Although std::cmp::min works with Options, it treats None as smaller value, so some "or" magic is needed:
    std::cmp::min(a.or(b), b.or(a))
}

fn parse_internal(str: &str) -> Option<i64> {
    Timestamp::parse(str).map(timestamp_to_secs)
}

#[cfg(test)]
mod tests {
    use crate::util::iso8601::{min_secs, parse_internal};

    #[test]
    fn test_parse() {
        assert_eq!(parse_internal("2018-02-20T18:02:12Z"), Some(1519149732));
        assert_eq!(parse_internal("foo bar"), None);
    }

    #[test]
    fn test_min_secs() {
        assert_eq!(min_secs::<i64>(None, None), None);
        assert_eq!(min_secs(Some(1), None), Some(1));
        assert_eq!(min_secs(None, Some(1)), Some(1));
        assert_eq!(min_secs(Some(3), Some(1)), Some(1));
        assert_eq!(min_secs(Some(1), Some(3)), Some(1));
        assert_eq!(min_secs(Some(2), Some(2)), Some(2));
    }
}