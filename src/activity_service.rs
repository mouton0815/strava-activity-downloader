// TODO: Move to service package or similar

use std::error::Error;
use axum::BoxError;
use iso8601_timestamp::Timestamp;
use rusqlite::Connection;
use crate::database::activity_table::ActivityTable;
use crate::database::state_table::StateTable;
use crate::domain::activity::ActivityVec;
use crate::util::iso8601;

pub struct ActivityService {
    connection: Connection
}

impl ActivityService {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let connection = Connection::open(db_path)?;
        ActivityTable::create_table(&connection)?;
        StateTable::create_table(&connection)?;
        Ok(Self{ connection })
    }

    /// Adds all activities to the database and returns the minimum start_date as epoch timestamp.
    pub fn add(&mut self, activities: &ActivityVec) -> Result<Option<i64>, BoxError> {
        let tx = self.connection.transaction()?;
        let mut min_time : Option<Timestamp> = None;
        for activity in activities {
            ActivityTable::upsert(&tx, activity)?;
            let time = Timestamp::parse(activity.start_date.as_str());
            min_time = iso8601::min_secs(time, min_time);
        }
        tx.commit()?;
        Ok(min_time.map(iso8601::timestamp_to_secs))
    }

    pub fn get_min_start_time(&mut self) -> Result<Option<i64>, BoxError> {
        let tx = self.connection.transaction()?;
        let min_time = ActivityTable::select_minimum_start_date(&tx)?;
        tx.commit()?;
        Ok(min_time.map(iso8601::string_to_secs))
    }
}

#[cfg(test)]
mod tests {
    use crate::ActivityService;
    use crate::domain::activity::{Activity, ActivityVec};

    #[test]
    fn test_add_none() {
        let vec = ActivityVec::new();
        let mut service = create_service();
        let time = service.add(&vec);
        assert!(time.is_ok());
        assert_eq!(time.unwrap(), None);

        let time = service.get_min_start_time();
        assert!(time.is_ok());
        assert_eq!(time.unwrap(), None);
    }

    #[test]
    fn test_add_some() {
        let vec = vec![
            Activity::dummy(2, "2018-02-20T18:02:13Z"),
            Activity::dummy(1, "2018-02-20T18:02:15Z"),
            Activity::dummy(3, "2018-02-20T18:02:12Z"),
        ];
        let mut service = create_service();
        let time = service.add(&vec);
        assert!(time.is_ok());
        assert_eq!(time.unwrap(), Some(1519149732)); // 2018-02-20T18:02:12Z

        let time = service.get_min_start_time();
        assert!(time.is_ok());
        assert_eq!(time.unwrap(), Some(1519149732));
    }

    fn create_service() -> ActivityService {
        let service = ActivityService::new(":memory:");
        assert!(service.is_ok());
        service.unwrap()
    }
}