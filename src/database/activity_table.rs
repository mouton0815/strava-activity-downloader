use const_format::concatcp;
use log::debug;
use rusqlite::{Connection, OptionalExtension, params, Result, Row, Transaction};
use crate::domain::activity::Activity;
use crate::domain::activity_map::ActivityMap;
use crate::domain::activity_stats::ActivityStats;

const CREATE_ACTIVITY_TABLE : &'static str =
    "CREATE TABLE IF NOT EXISTS activity (
        id INTEGER NOT NULL PRIMARY KEY,
        name TEXT NOT NULL,
        sport_type TEXT NOT NULL,
        start_date TEXT NOT NULL,
        distance INTEGER,
        moving_time INTEGER,
        total_elevation_gain INTEGER,
        average_speed INTEGER,
        kudos_count INTEGER
    )";

const UPSERT_ACTIVITY : &'static str =
    "INSERT INTO activity (id, name, sport_type, start_date, distance, moving_time, total_elevation_gain, average_speed, kudos_count) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
     ON CONFLICT(id) DO \
     UPDATE SET \
       name = excluded.name, \
       sport_type = excluded.sport_type, \
       start_date = excluded.start_date, \
       distance = excluded.distance, \
       moving_time = excluded.moving_time, \
       total_elevation_gain = excluded.total_elevation_gain, \
       average_speed = excluded.average_speed, \
       kudos_count = excluded.kudos_count";

const DELETE_ACTIVITY : &'static str =
    "DELETE FROM activity WHERE id = ?";

const SELECT_ACTIVITIES : &'static str =
    "SELECT id, name, sport_type, start_date, distance, moving_time, total_elevation_gain, average_speed, kudos_count FROM activity";

const SELECT_ACTIVITY : &'static str =
    concatcp!(SELECT_ACTIVITIES, " WHERE id = ?");

const SELECT_STATS : &'static str =
    "SELECT COUNT(*), MIN(start_date), MAX(start_date) FROM activity";


pub struct ActivityTable;

impl ActivityTable {
    pub fn create_table(conn: &Connection) -> Result<()> {
        debug!("Execute\n{}", CREATE_ACTIVITY_TABLE);
        conn.execute(CREATE_ACTIVITY_TABLE, [])?;
        Ok(())
    }

    pub fn upsert(tx: &Transaction, activity: &Activity) -> Result<()> {
        debug!("Execute\n{}\nwith: {:?}", UPSERT_ACTIVITY, activity);
        // Because sqlite does not support DECIMAL and stores FLOATs with many digits after the
        // dot (https://www.sqlite.org/floatingpoint.html), we need to convert the numbers to int:
        let dist_multiplied = (activity.distance.clone() * 10.0) as u64;
        let speed_multiplied = (activity.average_speed.clone() * 1000.0) as u64;
        let elev_multiplied = (activity.total_elevation_gain.clone() * 10.0) as u64;
        let values = params![
            activity.id, activity.name, activity.sport_type, activity.start_date, dist_multiplied,
            activity.moving_time, elev_multiplied, speed_multiplied, activity.kudos_count
        ];
        tx.execute(UPSERT_ACTIVITY, values)?;
        Ok(())
    }

    pub fn delete(tx: &Transaction, id: u64) -> Result<bool> {
        debug!("Execute\n{} with: {}", DELETE_ACTIVITY, id);
        let row_count = tx.execute(DELETE_ACTIVITY, params![id])?;
        Ok(row_count == 1)
    }

    pub fn select_all(tx: &Transaction) -> Result<ActivityMap> {
        debug!("Execute\n{}", SELECT_ACTIVITIES);
        let mut stmt = tx.prepare(SELECT_ACTIVITIES)?;
        let rows = stmt.query_map([], |row| {
            Self::row_to_activity(row)
        })?;
        let mut activity_map = ActivityMap::new();
        for row in rows {
            let (id, activity) = row?;
            activity_map.insert(id, activity);
        }
        Ok(activity_map)
    }

    pub fn select_by_id(tx: &Transaction, id: u64) -> Result<Option<Activity>> {
        debug!("Execute\n{} with: {}", SELECT_ACTIVITY, id);
        let mut stmt = tx.prepare(SELECT_ACTIVITY)?;
        stmt.query_row([id], |row | {
            Ok(Self::row_to_activity(row)?.1)
        }).optional()
    }

    pub fn select_stats(tx: &Transaction) -> Result<ActivityStats> {
        debug!("Execute\n{}", SELECT_STATS);
        let mut stmt = tx.prepare(SELECT_STATS)?;
        stmt.query_row([], |row | {
            Ok(ActivityStats::new(row.get(0)?, row.get(1)?, row.get(2)?))
        })
    }

    // TODO: Tuple in result is only needed for a function that is not needed
    fn row_to_activity(row: &Row) -> Result<(u64, Activity)> {
        // Reverse the conversion of floats to integers done in function upsert:
        Ok((row.get(0)?, Activity {
            id: row.get(0)?,
            name: row.get(1)?,
            sport_type: row.get(2)?,
            start_date: row.get(3)?,
            distance: (row.get::<_, u64>(4)? as f32 / 10.0),
            moving_time: row.get(5)?,
            total_elevation_gain: (row.get::<_, u64>(6)? as f32 / 10.0),
            average_speed: (row.get::<_, u64>(7)? as f32 / 1000.0),
            kudos_count: row.get(8)?
        }))
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::database::activity_table::ActivityTable;
    use crate::domain::activity::Activity;
    use crate::domain::activity_stats::ActivityStats;

    #[test]
    fn test_upsert() {
        let activity1 = Activity::dummy(1, "foo");
        let activity2 = Activity::dummy(2, "bar");
        let activity3 = Activity::dummy(1, "baz");

        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::upsert(&tx, &activity1).is_ok());
        assert!(ActivityTable::upsert(&tx, &activity2).is_ok());
        assert!(ActivityTable::upsert(&tx, &activity3).is_ok());
        assert!(tx.commit().is_ok());

        let ref_activities = [
            (1, &activity3), // activity3 overwrites activity1
            (2, &activity2)
        ];
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
    fn select_maximum_start_date() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(1, "2018-02-20T18:02:13Z")).unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(3, "2018-02-20T18:02:15Z")).unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(2, "2018-02-20T18:02:12Z")).unwrap();
        ActivityTable::upsert(&tx, &Activity::dummy(1, "2018-02-20T18:02:11Z")).unwrap(); // Note: ID overwrite

        let result = ActivityTable::select_stats(&tx);
        assert!(result.is_ok());
        let reference = ActivityStats::new(3, Some("2018-02-20T18:02:11Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()));
        assert_eq!(result.unwrap(), reference);
    }

    #[test]
    fn select_stats_missing() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();

        let result = ActivityTable::select_stats(&tx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ActivityStats::new(0, None, None));
    }

    fn create_connection_and_table() -> Connection {
        let conn = Connection::open(":memory:");
        assert!(conn.is_ok());
        let conn = conn.unwrap();
        assert!(ActivityTable::create_table(&conn).is_ok());
        conn
    }

    fn check_results(conn: &mut Connection, ref_activities: &[(u64, &Activity)]) {
        let tx = conn.transaction().unwrap();

        let activities = ActivityTable::select_all(&tx);
        assert!(activities.is_ok());
        assert!(tx.commit().is_ok());

        let activities = activities.unwrap();
        assert_eq!(activities.len(), ref_activities.len());

        for (_, &ref_activity) in ref_activities.iter().enumerate() {
            let (activity_id, activity_data) = ref_activity;
            let activity = activities.get(&activity_id);
            assert_eq!(activity, Some(activity_data));
        }
    }

    fn check_single_result(conn: &mut Connection, ref_activity: (u64, &Activity)) {
        let tx = conn.transaction().unwrap();

        let activity = ActivityTable::select_by_id(&tx, ref_activity.0);
        assert!(activity.is_ok());
        assert!(tx.commit().is_ok());

        let activity = activity.unwrap();
        assert!(activity.is_some());
        assert_eq!(activity.unwrap(), *ref_activity.1);
    }
}
