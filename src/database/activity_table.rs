use const_format::concatcp;
use log::debug;
use rusqlite::{Connection, OptionalExtension, params, Result, Row, Transaction};
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::gpx_store_state::GpxStoreState;

/// See [GpxStoreState] for the meaning of gpx_fetched values
const CREATE_ACTIVITY_TABLE : &'static str =
    "CREATE TABLE IF NOT EXISTS activity (
        id INTEGER NOT NULL PRIMARY KEY,
        name TEXT NOT NULL,
        sport_type TEXT NOT NULL,
        start_date TEXT NOT NULL,
        distance INTEGER NOT NULL,
        moving_time INTEGER NOT NULL,
        total_elevation_gain INTEGER NOT NULL,
        average_speed INTEGER NOT NULL,
        kudos_count INTEGER NOT NULL,
        gpx_fetched INTEGER DEFAULT 0 NOT NULL CHECK (gpx_fetched IN (0, 1, 2))
    )";

const INSERT_ACTIVITY : &'static str =
    "INSERT INTO activity (id, name, sport_type, start_date, distance, moving_time, total_elevation_gain, average_speed, kudos_count) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

const UPSERT_ACTIVITY : &'static str =
    concatcp!(INSERT_ACTIVITY, " \
     ON CONFLICT(id) DO \
     UPDATE SET \
       name = excluded.name, \
       sport_type = excluded.sport_type, \
       start_date = excluded.start_date, \
       distance = excluded.distance, \
       moving_time = excluded.moving_time, \
       total_elevation_gain = excluded.total_elevation_gain, \
       average_speed = excluded.average_speed, \
       kudos_count = excluded.kudos_count"); // Do NOT update column gpx_fetched

const DELETE_ACTIVITY : &'static str =
    "DELETE FROM activity WHERE id = ?";

const UPDATE_GPX_COLUMN : &'static str =
    "UPDATE activity SET gpx_fetched = ? WHERE id = ?";

const SELECT_ACTIVITIES : &'static str =
    "SELECT id, name, sport_type, start_date, distance, moving_time, total_elevation_gain, average_speed, kudos_count FROM activity";

const SELECT_ACTIVITY : &'static str =
    concatcp!(SELECT_ACTIVITIES, " WHERE id = ?");

const SELECT_EARLIEST_ACTIVITY_WITHOUT_GPX : &'static str =
    concatcp!(SELECT_ACTIVITIES, " WHERE gpx_fetched = 0 and start_date = (SELECT MIN(start_date) from activity WHERE gpx_fetched = 0)");

const SELECT_ACTIVITIES_WITH_GPX : &'static str =
    concatcp!(SELECT_ACTIVITIES, " WHERE gpx_fetched = 1 ORDER BY start_date ASC");

const SELECT_ACTIVITY_STATS: &'static str =
    "SELECT COUNT(id), MIN(start_date), MAX(start_date) FROM activity";

const SELECT_TRACK_STATS: &'static str =
    "SELECT COUNT(id), MAX(start_date) FROM activity where gpx_fetched = 1";


pub struct ActivityTable;

#[allow(dead_code)]
impl ActivityTable {
    pub fn create_table(conn: &Connection) -> Result<()> {
        debug!("Execute\n{}", CREATE_ACTIVITY_TABLE);
        conn.execute(CREATE_ACTIVITY_TABLE, [])?;
        Ok(())
    }

    pub fn insert(tx: &Transaction, activity: &Activity) -> Result<()> {
        Self::execute_for_activity(tx, INSERT_ACTIVITY, activity)
    }

    pub fn upsert(tx: &Transaction, activity: &Activity) -> Result<()> {
        Self::execute_for_activity(tx, UPSERT_ACTIVITY, activity)
    }

    pub fn delete(tx: &Transaction, id: u64) -> Result<bool> {
        debug!("Execute\n{} with: {}", DELETE_ACTIVITY, id);
        let row_count = tx.execute(DELETE_ACTIVITY, params![id])?;
        Ok(row_count == 1)
    }

    pub fn update_gpx_column(tx: &Transaction, id: u64, state: GpxStoreState) -> Result<bool> {
        let value = state as i32;
        debug!("Execute\n{} with: {} {}", UPDATE_GPX_COLUMN, id, value);
        let row_count = tx.execute(UPDATE_GPX_COLUMN, params![value, id])?;
        Ok(row_count == 1)
    }

    pub fn select_all(tx: &Transaction) -> Result<ActivityVec> {
        debug!("Execute\n{}", SELECT_ACTIVITIES);
        let mut stmt = tx.prepare(SELECT_ACTIVITIES)?;
        let activity_iter = stmt.query_map([], |row| {
            Self::row_to_activity(row)
        })?;
        let mut activity_vec = ActivityVec::new();
        for activity in activity_iter {
            activity_vec.push(activity?)
        }
        Ok(activity_vec)
    }

    pub fn select_by_id(tx: &Transaction, id: u64) -> Result<Option<Activity>> {
        debug!("Execute\n{} with: {}", SELECT_ACTIVITY, id);
        let mut stmt = tx.prepare(SELECT_ACTIVITY)?;
        stmt.query_row([id], |row | {
            Ok(Self::row_to_activity(row)?)
        }).optional()
    }

    pub fn select_all_with_gpx(tx: &Transaction) -> Result<ActivityVec> {
        debug!("Execute\n{}", SELECT_ACTIVITIES_WITH_GPX);
        let mut stmt = tx.prepare(SELECT_ACTIVITIES_WITH_GPX)?;
        let activity_iter = stmt.query_map([], |row| {
            Self::row_to_activity(row)
        })?;
        let mut activity_vec = ActivityVec::new();
        for activity in activity_iter { // TODO: Can this be done as stream transformation?
            activity_vec.push(activity?)
        }
        Ok(activity_vec)
    }

    pub fn select_earliest_without_gpx(tx: &Transaction) -> Result<Option<Activity>> {
        debug!("Execute\n{}", SELECT_EARLIEST_ACTIVITY_WITHOUT_GPX);
        let mut stmt = tx.prepare(SELECT_EARLIEST_ACTIVITY_WITHOUT_GPX)?;
        stmt.query_row([], |row | {
            Ok(Self::row_to_activity(row)?)
        }).optional()
    }

    pub fn select_stats(tx: &Transaction) -> Result<ActivityStats> {
        debug!("Execute\n{}", SELECT_ACTIVITY_STATS);
        let mut stmt = tx.prepare(SELECT_ACTIVITY_STATS)?;
        let (act_cnt, act_min, act_max) = stmt.query_row([], |row | {
            let act_cnt : u32 = row.get(0)?;
            let act_min : Option<String> = row.get(1)?;
            let act_max : Option<String> = row.get(2)?;
            Ok((act_cnt, act_min, act_max))
        })?;
        debug!("Execute\n{}", SELECT_TRACK_STATS);
        let mut stmt = tx.prepare(SELECT_TRACK_STATS)?;
        let (trk_cnt, trk_max) = stmt.query_row([], |row | {
            let trk_cnt : u32 = row.get(0)?;
            let trk_max : Option<String> = row.get(1)?;
            Ok((trk_cnt, trk_max))
        })?;
        Ok(ActivityStats::new(act_cnt, act_min, act_max, trk_cnt, trk_max))
    }

    fn execute_for_activity(tx: &Transaction, query: &str, activity: &Activity) -> Result<()> {
        debug!("Execute\n{}\nwith: {:?}", query, activity);
        // Because sqlite does not support DECIMAL and stores FLOATs with many digits after the
        // dot (https://www.sqlite.org/floatingpoint.html), we need to convert the numbers to int.
        // The inverse operations are done by row_to_activity() below.
        let dist_multiplied = (activity.distance.clone() * 10.0) as u64;
        let speed_multiplied = (activity.average_speed.clone() * 1000.0) as u64;
        let elev_multiplied = (activity.total_elevation_gain.clone() * 10.0) as u64;
        let values = params![
            activity.id, activity.name, activity.sport_type, activity.start_date, dist_multiplied,
            activity.moving_time, elev_multiplied, speed_multiplied, activity.kudos_count
        ];
        tx.execute(query, values).map(|_| ()) // Ignore returned row count
    }

    fn row_to_activity(row: &Row) -> Result<Activity> {
        // Reverse the conversion of floats to integers done in function upsert:
        Ok(Activity {
            id: row.get(0)?,
            name: row.get(1)?,
            sport_type: row.get(2)?,
            start_date: row.get(3)?,
            distance: (row.get::<_, u64>(4)? as f32 / 10.0),
            moving_time: row.get(5)?,
            total_elevation_gain: (row.get::<_, u64>(6)? as f32 / 10.0),
            average_speed: (row.get::<_, u64>(7)? as f32 / 1000.0),
            kudos_count: row.get(8)?
        })
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::database::activity_table::ActivityTable;
    use crate::domain::activity::Activity;
    use crate::domain::activity_stats::ActivityStats;
    use crate::domain::gpx_store_state::GpxStoreState;

    #[test]
    fn test_insert() {
        let activity1 = Activity::dummy(1, "foo");
        let activity2 = Activity::dummy(2, "bar");
        let activity3 = Activity::dummy(1, "baz");

        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::insert(&tx, &activity1).is_ok());
        assert!(ActivityTable::insert(&tx, &activity2).is_ok());
        assert!(ActivityTable::insert(&tx, &activity3).is_err()); // activity3 collides with activity1
        assert!(tx.commit().is_ok());

        let ref_activities = [&activity1, &activity2];
        check_results(&mut conn, &ref_activities);
        check_single_result(&mut conn, ref_activities[0]);
        check_single_result(&mut conn, ref_activities[1]);
    }

    #[test]
    fn test_upsert() {
        let activity1 = Activity::dummy(1, "foo");
        let activity2 = Activity::dummy(2, "bar");
        let activity3 = Activity::dummy(1, "baz");

        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::upsert(&tx, &activity1).is_ok());
        assert!(ActivityTable::upsert(&tx, &activity2).is_ok());
        assert!(ActivityTable::upsert(&tx, &activity3).is_ok()); // activity3 overwrites activity1
        assert!(tx.commit().is_ok());

        let ref_activities = [&activity3, &activity2];
        check_results(&mut conn, &ref_activities);
        check_single_result(&mut conn, ref_activities[0]);
        check_single_result(&mut conn, ref_activities[1]);
    }

    #[test]
    fn test_delete() {
        let activity = Activity::dummy(1, "n/a");

        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::upsert(&tx, &activity).is_ok());
        let result = ActivityTable::delete(&tx, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert!(tx.commit().is_ok());

        check_results(&mut conn, &[]);
    }

    #[test]
    fn test_delete_missing() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        let result = ActivityTable::delete(&tx, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
        assert!(tx.commit().is_ok());
    }

    #[test]
    fn test_update_gpx_column() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).is_ok());
        let result = ActivityTable::update_gpx_column(&tx, 1, GpxStoreState::Stored);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        assert!(tx.commit().is_ok());
    }

    #[test]
    fn test_update_gpx_column_missing() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        let result = ActivityTable::update_gpx_column(&tx, 1, GpxStoreState::Stored);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
        assert!(tx.commit().is_ok());
    }

    #[test]
    fn test_earliest_without_gpx() {
        let activity1 = Activity::dummy(1, "2018-02-20T18:02:13Z");
        let activity2 = Activity::dummy(2, "2018-02-20T18:02:12Z");
        let activity3 = Activity::dummy(3, "2018-02-20T18:02:11Z");

        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        ActivityTable::upsert(&tx, &activity1).unwrap();
        ActivityTable::upsert(&tx, &activity2).unwrap();
        ActivityTable::upsert(&tx, &activity3).unwrap();
        ActivityTable::update_gpx_column(&tx, 3, GpxStoreState::Stored).unwrap(); // Earliest activity already has a GPX track
        ActivityTable::update_gpx_column(&tx, 2, GpxStoreState::Missing).unwrap(); // Same for missing track

        let result = ActivityTable::select_earliest_without_gpx(&tx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(activity1));
        assert!(tx.commit().is_ok());
    }

    #[test]
    fn test_earliest_without_gpx_missing() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();

        let result = ActivityTable::select_earliest_without_gpx(&tx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
        assert!(tx.commit().is_ok());
    }

    #[test]
    fn test_all_with_gpx() {
        // Note: Inverse timely order:
        let activity1 = Activity::dummy(3, "2018-02-20T18:02:15Z");
        let activity2 = Activity::dummy(5, "2018-02-20T18:02:13Z");
        let activity3 = Activity::dummy(7, "2018-02-20T18:02:11Z");

        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        ActivityTable::upsert(&tx, &activity1).unwrap();
        ActivityTable::upsert(&tx, &activity2).unwrap();
        ActivityTable::upsert(&tx, &activity3).unwrap();
        ActivityTable::update_gpx_column(&tx, 3, GpxStoreState::Stored).unwrap();
        ActivityTable::update_gpx_column(&tx, 7, GpxStoreState::Stored).unwrap();

        let result = ActivityTable::select_all_with_gpx(&tx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![activity3, activity1]);
        assert!(tx.commit().is_ok());
    }

    #[test]
    fn test_select_stats() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(1, "2018-02-20T18:02:13Z")).unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(3, "2018-02-20T18:02:15Z")).unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(2, "2018-02-20T18:02:12Z")).unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(1, "2018-02-20T18:02:11Z")).unwrap(); // Note: ID overwrite
        ActivityTable::update_gpx_column(&tx, 1, GpxStoreState::Stored).unwrap();

        let result = ActivityTable::select_stats(&tx);
        assert!(result.is_ok());
        let reference = ActivityStats::new(3, Some("2018-02-20T18:02:11Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()), 1, Some("2018-02-20T18:02:11Z".to_string()));
        assert_eq!(result.unwrap(), reference);
    }

    #[test]
    fn test_select_stats_missing() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();

        let result = ActivityTable::select_stats(&tx);
        match result {
            Ok(_) => {}
            Err(e) => println!("{:?}", e)
        }
        //assert!(result.is_ok());
        //assert_eq!(result.unwrap(), ActivityStats::new(0, 0, None, None));
    }

    fn create_connection_and_table() -> Connection {
        let conn = Connection::open(":memory:");
        assert!(conn.is_ok());
        let conn = conn.unwrap();
        assert!(ActivityTable::create_table(&conn).is_ok());
        conn
    }

    fn check_results(conn: &mut Connection, ref_activities: &[&Activity]) {
        let tx = conn.transaction().unwrap();

        let activities = ActivityTable::select_all(&tx);
        assert!(activities.is_ok());
        assert!(tx.commit().is_ok());

        let activities = activities.unwrap();
        assert_eq!(activities.len(), ref_activities.len());

        for (index, &ref_activity) in ref_activities.iter().enumerate() {
            let activity = activities.get(index);
            assert_eq!(activity, Some(ref_activity));
        }
    }

    fn check_single_result(conn: &mut Connection, ref_activity: &Activity) {
        let tx = conn.transaction().unwrap();

        let activity = ActivityTable::select_by_id(&tx, ref_activity.id.clone());
        assert!(activity.is_ok());
        assert!(tx.commit().is_ok());

        let activity = activity.unwrap();
        assert!(activity.is_some());
        assert_eq!(activity.unwrap(), *ref_activity);
    }
}
