use std::collections::BTreeMap;
use serde::{Deserialize,Serialize};
use crate::domain::activity::Activity;

///
/// A map of [Activity](crate::domain::activity::Activity) objects with their ids as keys.
/// The implementation with an encapsulated map was chosen to produce the desired json output
/// <code>{ <id>: <activity>, ... }</code>.
///
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ActivityMap(BTreeMap<u64, Activity>);

impl ActivityMap {
    pub fn new() -> Self {
        Self{ 0: BTreeMap::new() }
    }

    pub fn put(&mut self, id: u64, activity: Activity) {
        self.0.insert(id, activity);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, id: u64) -> &Activity {
        self.0.get(&id).unwrap() // Panic accepted
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::activity::Activity;
    use crate::domain::activity_map::ActivityMap;
    use crate::util::serde_and_verify::tests::serde_and_verify;

    #[test]
    fn test_put() {
        let mut map = ActivityMap::new();
        map.put(2, Activity::new(2, "bar", "hike", "def", 1.0, 1));
        map.put(1, Activity::new(1, "foo", "walk", "abc", 0.3, 3));

        let json_ref = r#"{"1":{"id":1,"name":"foo","sport_type":"walk","start_date":"abc","distance":0.3,"kudos_count":3},"2":{"id":2,"name":"bar","sport_type":"hike","start_date":"def","distance":1.0,"kudos_count":1}}"#;
        serde_and_verify(&map, json_ref);
    }

    #[test]
    fn test_empty() {
        let map = ActivityMap::new();
        let json_ref = r#"{}"#;
        serde_and_verify(&map, json_ref);
    }

    #[test]
    fn test_get_and_len() {
        let activity = Activity::new(5, "foo", "walk", "abc", 0.3, 3);
        let mut map = ActivityMap::new();
        map.put(5, activity.clone());
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(5), &activity);
    }
}