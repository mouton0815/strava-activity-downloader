use std::cmp::max;
use std::error::Error;
use axum::BoxError;
use iso8601_timestamp::Timestamp;
use log::{debug, info};
use rusqlite::Connection;
use crate::database::activity_table::ActivityTable;
use crate::database::state_table::StateTable;
use crate::domain::activity::ActivityVec;
use crate::domain::activity_stats::ActivityStats;
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

    /// Adds all activities to the database and returns the maximum start_date as epoch timestamp.
    pub fn add(&mut self, activities: &ActivityVec) -> Result<Option<i64>, BoxError> {
        info!("Add {} activities to database", activities.len());
        let tx = self.connection.transaction()?;
        let mut max_time : Option<Timestamp> = None;
        for activity in activities {
            ActivityTable::upsert(&tx, activity)?;
            let time = Timestamp::parse(activity.start_date.as_str());
            max_time = max(time, max_time);
        }
        tx.commit()?;
        Ok(max_time.map(iso8601::timestamp_to_secs))
    }

    pub fn get_stats(&mut self) -> Result<ActivityStats, BoxError> {
        let tx = self.connection.transaction()?;
        let stats = ActivityTable::select_stats(&tx)?;
        tx.commit()?;
        debug!("Read activity stats {:?} from database", stats);
        Ok(stats)
    }

    pub fn get_max_start_time(&mut self) -> Result<Option<i64>, BoxError> {
        Ok(self.get_stats()?.max_time_as_secs())
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

        let time = service.get_max_start_time();
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
        assert_eq!(time.unwrap(), Some(1519149735)); // 2018-02-20T18:02:12Z

        let time = service.get_max_start_time();
        assert!(time.is_ok());
        assert_eq!(time.unwrap(), Some(1519149735));
    }

    fn create_service() -> ActivityService {
        let service = ActivityService::new(":memory:");
        assert!(service.is_ok());
        service.unwrap()
    }
}