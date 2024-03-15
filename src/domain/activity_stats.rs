use serde::Serialize;
use crate::util::iso8601;

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct ActivityStats {
    act_count: u32,
    act_min_time: Option<String>,
    act_max_time: Option<String>,
    trk_count: u32,
    trk_max_time: Option<String>
}

impl ActivityStats {
    pub fn new(act_count: u32, act_min_time: Option<String>, act_max_time: Option<String>, trk_count: u32, trk_max_time: Option<String>) -> Self {
        Self { act_count, act_min_time, act_max_time, trk_count, trk_max_time }
    }

    pub fn act_max_time_as_secs(&self) -> Option<i64> {
        self.act_max_time.as_ref().map(|s| iso8601::string_to_secs(s))
    }

    pub fn merge(&mut self, stats: &ActivityStats) {
        self.act_count += stats.act_count.clone();
        self.act_min_time = ActivityStats::min(&self.act_min_time, &stats.act_min_time);
        self.act_max_time = ActivityStats::max(&self.act_max_time, &stats.act_max_time);
        self.trk_count += stats.trk_count.clone();
        self.trk_max_time = ActivityStats::max(&self.trk_max_time, &stats.trk_max_time);
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
        let mut this = ActivityStats::new(0, None, None, 0, None);
        let other = ActivityStats::new(0, None, None, 0, None);
        this.merge(&other);
        assert_eq!(this, other);
    }

    #[test]
    fn test_merge_none_some() {
        let mut this = ActivityStats::new(0, None, None, 0, None);
        let other = ActivityStats::new(5, Some("a".to_string()), Some("b".to_string()), 3,  Some("c".to_string()));
        this.merge(&other);
        assert_eq!(this, other);
    }

    #[test]
    fn test_merge_some_some() {
        let mut this = ActivityStats::new(3, Some("b".to_string()), Some("a".to_string()), 2, Some("b".to_string()));
        let other = ActivityStats::new(5, Some("a".to_string()), Some("b".to_string()), 4, Some("c".to_string()));
        this.merge(&other);
        assert_eq!(this, ActivityStats::new(8, other.act_min_time, other.act_max_time, 6, other.trk_max_time));
    }

    #[test]
    fn test_merge_some_keep() {
        let mut this = ActivityStats::new(5, Some("a".to_string()), Some("b".to_string()), 4, Some("c".to_string()));
        let other = ActivityStats::new(3, Some("b".to_string()), Some("a".to_string()), 2, Some("b".to_string()));
        this.merge(&other);
        assert_eq!(this, ActivityStats::new(8, this.act_min_time.clone(), this.act_max_time.clone(), 6, this.trk_max_time.clone()));
    }

    #[test]
    fn test_merge_some_none() {
        let mut this = ActivityStats::new(5, Some("a".to_string()), Some("b".to_string()), 4, Some("c".to_string()));
        let other = ActivityStats::new(3, None, None, 2, None);
        this.merge(&other);
        assert_eq!(this, ActivityStats::new(8, this.act_min_time.clone(), this.act_max_time.clone(), 6, this.trk_max_time.clone()));
    }
}
