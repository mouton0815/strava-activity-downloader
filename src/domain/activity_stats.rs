use serde::{Deserialize, Serialize};
use crate::util::iso8601;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ActivityStats {
    count: u32,
    min_time: Option<String>,
    max_time: Option<String>
}

impl ActivityStats {
    pub fn new(count: u32, min_time: Option<String>, max_time: Option<String>) -> Self {
        Self { count, min_time, max_time }
    }

    pub fn max_time_as_secs(&self) -> Option<i64> {
        self.max_time.as_ref().map(|s| iso8601::string_to_secs(s))
    }

    pub fn merge(&mut self, stats: &ActivityStats) {
        self.count += stats.count.clone();
        self.min_time = ActivityStats::min(&self.min_time, &stats.min_time);
        self.max_time = ActivityStats::max(&self.max_time, &stats.max_time);
    }

    fn min(a: &Option<String>, b: &Option<String>) -> Option<String> {
        // std::cmp::min for Option treats None as minimal value, but we need the timestamp if one of the args is Some
        std::cmp::min(a.as_ref().or(b.as_ref()), b.as_ref().or(a.as_ref())).map(String::from)
    }

    fn max(a: &Option<String>, b: &Option<String>) -> Option<String> {
        std::cmp::max(a.as_ref(), b.as_ref()).map(String::from)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::activity_stats::ActivityStats;

    #[test]
    fn test_merge_none_none() {
        let mut this = ActivityStats::new(0, None, None);
        let other = ActivityStats::new(0, None, None);
        this.merge(&other);
        assert_eq!(this, other);
    }

    #[test]
    fn test_merge_none_some() {
        let mut this = ActivityStats::new(0, None, None);
        let other = ActivityStats::new(5, Some("2018-02-20T18:02:12Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()));
        this.merge(&other);
        assert_eq!(this, other);
    }

    #[test]
    fn test_merge_some_some() {
        let mut this = ActivityStats::new(3, Some("2018-02-20T18:02:12Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()));
        let other = ActivityStats::new(5, Some("2018-02-20T18:02:10Z".to_string()), Some("2018-02-20T18:02:17Z".to_string()));
        this.merge(&other);
        assert_eq!(this, ActivityStats::new(8, other.min_time, other.max_time));
    }

    #[test]
    fn test_merge_some_keep() {
        let mut this = ActivityStats::new(5, Some("2018-02-20T18:02:10Z".to_string()), Some("2018-02-20T18:02:17Z".to_string()));
        let other = ActivityStats::new(3, Some("2018-02-20T18:02:12Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()));
        this.merge(&other);
        assert_eq!(this, ActivityStats::new(8, this.min_time.clone(), this.max_time.clone()));
    }
}
