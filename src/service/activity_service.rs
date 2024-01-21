use std::error::Error;
use axum::BoxError;
use log::{debug, info};
use rusqlite::Connection;
use crate::{ActivityStream, write_gpx};
use crate::database::activity_table::ActivityTable;
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;

pub struct ActivityService {
    connection: Connection
}

impl ActivityService {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let connection = Connection::open(db_path)?;
        ActivityTable::create_table(&connection)?;
        Ok(Self{ connection })
    }

    /// Adds all activities to the database and returns the computed [ActivityStats]
    /// for **these** inserted activities (**not** for the entire database table).
    pub fn add(&mut self, activities: &ActivityVec) -> Result<ActivityStats, BoxError> {
        info!("Add {} activities to database", activities.len());
        let tx = self.connection.transaction()?;
        let count = activities.len() as u32;
        let mut min_time : Option<String> = None;
        let mut max_time : Option<String> = None;
        for activity in activities {
            ActivityTable::insert(&tx, activity)?;
            // std::cmp::min for Option treats None as minimal value, but we need the timestamp
            min_time = Some(match min_time {
                Some(time) => std::cmp::min(activity.start_date.clone(), time),
                None => activity.start_date.clone()
            });
            max_time = std::cmp::max(Some(activity.start_date.clone()), max_time);
        }
        tx.commit()?;
        Ok(ActivityStats::new(count, min_time, max_time))
    }

    pub fn get_stats(&mut self) -> Result<ActivityStats, BoxError> {
        let tx = self.connection.transaction()?;
        let stats = ActivityTable::select_stats(&tx)?;
        tx.commit()?;
        debug!("Read activity stats {:?} from database", stats);
        Ok(stats)
    }

    pub fn get_earliest_without_gpx(&mut self) -> Result<Option<Activity>, BoxError> {
        let tx = self.connection.transaction()?;
        let activity = ActivityTable::select_earliest_without_gpx(&tx)?;
        tx.commit()?;
        debug!("Earliest activity without GPX: {:?}", activity);
        Ok(activity)
    }

    pub fn store_gpx(&mut self, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
        // Store GPX file ...
        write_gpx(activity, stream)?;
        // ... then mark corresponding database row
        let tx = self.connection.transaction()?;
        let result = ActivityTable::update_gpx_column(&tx, activity.id.clone())?;
        tx.commit()?;
        debug!("Marked 'GPX fetched' for activity {} with result {result}", activity.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ActivityService;
    use crate::domain::activity::{Activity, ActivityVec};
    use crate::domain::activity_stats::ActivityStats;

    #[test]
    fn test_add_none() {
        let vec = ActivityVec::new();
        let mut service = create_service();
        let stats = service.add(&vec);
        assert!(stats.is_ok());
        assert_eq!(stats.unwrap(), ActivityStats::new(0, None, None));
    }

    #[test]
    fn test_add_some() {
        let vec = vec![
            Activity::dummy(2, "2018-02-20T18:02:13Z"),
            Activity::dummy(1, "2018-02-20T18:02:15Z"),
            Activity::dummy(3, "2018-02-20T18:02:12Z"),
        ];
        let mut service = create_service();
        let stats = service.add(&vec);
        assert!(stats.is_ok());
        assert_eq!(stats.unwrap(), ActivityStats::new(
            3, Some("2018-02-20T18:02:12Z".to_string()), Some("2018-02-20T18:02:15Z".to_string())));
        //assert_eq!(stats.unwrap(), Some(1519149735)); // 2018-02-20T18:02:12Z
    }

    fn create_service() -> ActivityService {
        let service = ActivityService::new(":memory:");
        assert!(service.is_ok());
        service.unwrap()
    }
}