// TODO: Move to service package or similar

use std::error::Error;
use axum::BoxError;
use iso8601_timestamp::Timestamp;
use rusqlite::Connection;
use crate::database::activity_table::ActivityTable;
use crate::database::state_table::StateTable;
use crate::domain::activity::ActivityVec;

pub struct ActivityService {
    connection: Connection,
    min_time: Option<Timestamp>
}

impl ActivityService {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let connection = Connection::open(db_path)?;
        ActivityTable::create_table(&connection)?;
        StateTable::create_table(&connection)?;
        Ok(Self{ connection, min_time: None })
    }

    /// Adds all activities to the database and returns the minimum start_date as epoch timestamp.
    pub fn add(&mut self, activities: &ActivityVec) -> Result<Option<i64>, BoxError> {
        let tx = self.connection.transaction()?;
        // TODO: Select min_time from database if not existing
        for activity in activities {
            ActivityTable::upsert(&tx, activity)?;
            let time = Timestamp::parse(activity.start_date.as_str());
            println!("-----> {:?} - {:?}", time, self.min_time);
            if time.is_some() && (self.min_time.is_none() || time.unwrap() < self.min_time.unwrap()) {
                self.min_time = time;
            }
            println!("-now-> {:?}", self.min_time);
        }
        tx.commit()?;
        Ok(self.min_time_as_secs())
    }

    fn min_time_as_secs(&self) -> Option<i64> {
        self.min_time.map(|t| t.duration_since(Timestamp::UNIX_EPOCH).whole_seconds())
    }
}

#[cfg(test)]
mod tests {
    use crate::ActivityService;
    use crate::domain::activity::{Activity, ActivityVec};

    #[test]
    fn test_add_empty() {
        let vec = ActivityVec::new();
        let mut service = create_service();
        let time = service.add(&vec);
        assert!(time.is_ok());
        assert_eq!(time.unwrap(), None);
        assert_eq!(service.min_time_as_secs(), None);
    }

    #[test]
    fn test_add_simple() {
        let vec = vec![
            Activity::new(2, "bar", "hike", "2018-02-20T18:02:13Z", 1.0, 1),
            Activity::new(1, "foo", "walk", "2018-02-20T18:02:15Z", 0.3, 3),
            Activity::new(3, "baz", "bike", "2018-02-20T18:02:12Z", 0.3, 3),
        ];
        let mut service = create_service();
        let time = service.add(&vec);
        assert!(time.is_ok());
        assert_eq!(time.unwrap(), Some(1519149732)); // 2018-02-20T18:02:12Z
        assert_eq!(service.min_time_as_secs(), Some(1519149732));
    }

    #[test]
    fn test_add_repeated_transient() {
        let mut service = create_service();
        let vec = vec![Activity::new(3, "baz", "bike", "2018-02-20T18:02:12Z", 0.3, 3)];
        let time = service.add(&vec);
        assert!(time.is_ok());
        let vec = vec![Activity::new(2, "bar", "hike", "2018-02-20T18:02:13Z", 1.0, 1)];
        let time = service.add(&vec);
        assert_eq!(time.unwrap(), Some(1519149732)); // 2018-02-20T18:02:12Z
        assert_eq!(service.min_time_as_secs(), Some(1519149732));
    }

    #[test]
    fn test_add_repeated_persistent() {
        let mut service = create_service();
        let vec = vec![Activity::new(3, "baz", "bike", "2018-02-20T18:02:12Z", 0.3, 3)];
        let time = service.add(&vec);
        assert!(time.is_ok());
        service.min_time = None; // Pretend a server restart so that the the add() function re-reads the min time from database
        let vec = vec![Activity::new(2, "bar", "hike", "2018-02-20T18:02:13Z", 1.0, 1)];
        let time = service.add(&vec);
        assert_eq!(time.unwrap(), Some(1519149732)); // 2018-02-20T18:02:12Z
        assert_eq!(service.min_time_as_secs(), Some(1519149732));
    }

    fn create_service() -> ActivityService {
        let service = ActivityService::new(":memory:");
        assert!(service.is_ok());
        service.unwrap()
    }

    // fn check_min_time(min_time: Result<Option<i64>, BoxError>, ref_time: Option<i64>) {}
}