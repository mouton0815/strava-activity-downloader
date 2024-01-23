use serde::Deserialize;

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Activity {
    pub id: u64,
    pub name: String,
    pub sport_type: String,
    pub start_date: String,
    pub distance: f32,
    pub moving_time: u64,
    pub total_elevation_gain: f32,
    pub average_speed: f32,
    pub kudos_count: u32
}

pub type ActivityVec = Vec<Activity>;

#[cfg(test)]
mod tests {
    use crate::domain::activity::Activity;

    impl Activity {
        /// Convenience function that takes &str literals
        pub fn new(id: u64, name: &str, sport_type: &str, start_date: &str, distance: f32,
                   moving_time: u64, total_elevation_gain: f32, average_speed: f32,
                   kudos_count: u32) -> Self {
            Self {
                id,
                name: String::from(name),
                sport_type: String::from(sport_type),
                start_date: String::from(start_date),
                distance,
                moving_time,
                total_elevation_gain,
                average_speed,
                kudos_count
            }
        }

        /// Fills most fields with dummy values
        pub fn dummy(id: u64, start_date: &str) -> Self {
            Self::new(id, "foo", "walk", start_date, 310.4, 1005, 100.9, 3.558, 3)
        }
    }
}