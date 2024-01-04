use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Activity {
    pub id: u64,
    pub name: String,
    pub sport_type: String,
    pub start_date: String,
    pub distance: f32,
    pub kudos_count: u32
}

pub type ActivityVec = Vec<Activity>;

#[cfg(test)]
mod tests {
    use crate::domain::activity::Activity;
    use crate::util::serde_and_verify::tests::serde_and_verify;

    impl Activity {
        /// Convenience function that takes &str literals
        pub fn new(id: u64, name: &str, sport_type: &str, start_date: &str, distance: f32, kudos_count: u32) -> Self {
            Self {
                id,
                name: String::from(name),
                sport_type: String::from(sport_type),
                start_date: String::from(start_date),
                distance,
                kudos_count
            }
        }
    }

    #[test]
    fn test_serde() {
        let activity = Activity::new(1, "foo", "walk", "abc", 3.14, 3);
        let json_ref = r#"{"id":1,"name":"foo","sport_type":"walk","start_date":"abc","distance":3.14,"kudos_count":3}"#;
        serde_and_verify(&activity, json_ref);
    }
}