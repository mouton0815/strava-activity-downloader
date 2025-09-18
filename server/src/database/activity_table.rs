use const_format::concatcp;
use log::debug;
use sqlx::{query, Result, Row};
use crate::database::db_executor::DbExecutor;
use crate::database::db_types::DBRow;
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::track_store_state::TrackStoreState;

/// See [TrackStoreState] for the meaning of gpx_fetched values
const CREATE_ACTIVITY_TABLE : &str =
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

const INSERT_ACTIVITY : &str =
    "INSERT INTO activity (id, name, sport_type, start_date, distance, moving_time, total_elevation_gain, average_speed, kudos_count) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)";

const UPSERT_ACTIVITY : &str =
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

const DELETE_ACTIVITY : &str =
    "DELETE FROM activity WHERE id = ?";

const UPDATE_FETCHED_COLUMN: &str =
    "UPDATE activity SET gpx_fetched = ? WHERE id = ?";

const SELECT_ACTIVITIES : &str =
    "SELECT id, name, sport_type, start_date, distance, moving_time, total_elevation_gain, average_speed, kudos_count FROM activity";

const SELECT_ACTIVITY : &str =
    concatcp!(SELECT_ACTIVITIES, " WHERE id = ?");

const SELECT_EARLIEST_ACTIVITY_WITHOUT_TRACK: &str =
    concatcp!(SELECT_ACTIVITIES, " WHERE gpx_fetched = 0 and start_date = (SELECT MIN(start_date) from activity WHERE gpx_fetched = 0)");

const SELECT_ACTIVITIES_WITH_TRACK: &str =
    concatcp!(SELECT_ACTIVITIES, " WHERE gpx_fetched = 1 ORDER BY start_date ASC");

const SELECT_ACTIVITY_STATS: &str =
    "SELECT \
      COUNT(id), \
      MIN(start_date), \
      MAX(start_date), \
      COUNT(id) FILTER (where gpx_fetched = 1), \
      MAX(start_date) FILTER (where gpx_fetched = 1) \
    FROM activity";

pub struct ActivityTable;

#[allow(dead_code)]
impl ActivityTable {
    pub async fn create_table<'e, E>(executor: E) -> Result<()>
        where E: DbExecutor<'e> {
        debug!("Execute\n{}", CREATE_ACTIVITY_TABLE);
        query(CREATE_ACTIVITY_TABLE).execute(executor).await?;
        Ok(())
    }

    pub async fn insert<'e, E>(executor: E, activity: &Activity) -> Result<()>
        where E: DbExecutor<'e> {
        Self::execute_for_activity(executor, INSERT_ACTIVITY, activity).await
    }

    pub async fn upsert<'e, E>(executor: E, activity: &Activity) -> Result<()>
        where E: DbExecutor<'e> {
        Self::execute_for_activity(executor, UPSERT_ACTIVITY, activity).await
    }

    pub async fn delete<'e, E>(executor: E, id: u64) -> Result<bool>
        where E: DbExecutor<'e> {
        debug!("Execute\n{} with: {}", DELETE_ACTIVITY, id);
        let result = query(DELETE_ACTIVITY)
            .bind(id as i64) // sqlx::sqlite cannot encode u64
            .execute(executor)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn update_fetched_column<'e, E>(executor: E, id: u64, state: TrackStoreState) -> Result<bool>
        where E: DbExecutor<'e> {
        let value = state as i32;
        debug!("Execute\n{} with: {} {}", UPDATE_FETCHED_COLUMN, id, value);
        let result = query(UPDATE_FETCHED_COLUMN)
            .bind(value)
            .bind(id as i64)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn select_all<'e, E>(executor: E) -> Result<ActivityVec>
        where E: DbExecutor<'e> {
        debug!("Execute\n{}", SELECT_ACTIVITIES);
        query(SELECT_ACTIVITIES)
            .map(|row: DBRow| Self::row_to_activity(&row))
            .fetch_all(executor)
            .await
    }

    pub async fn select_by_id<'e, E>(executor: E, id: u64) -> Result<Option<Activity>>
        where E: DbExecutor<'e> {
        debug!("Execute\n{} with: {}", SELECT_ACTIVITY, id);
        query(SELECT_ACTIVITY)
            .bind(id as i64)
            .map(|row: DBRow| Self::row_to_activity(&row))
            .fetch_optional(executor)
            .await
    }

    pub async fn select_all_with_track<'e, E>(executor: E) -> Result<ActivityVec>
        where E: DbExecutor<'e> {
        debug!("Execute\n{}", SELECT_ACTIVITIES_WITH_TRACK);
        query(SELECT_ACTIVITIES_WITH_TRACK)
            .map(|row: DBRow| Self::row_to_activity(&row))
            .fetch_all(executor)
            .await
    }

    pub async fn select_earliest_without_track<'e, E>(executor: E) -> Result<Option<Activity>>
        where E: DbExecutor<'e> {
        debug!("Execute\n{}", SELECT_EARLIEST_ACTIVITY_WITHOUT_TRACK);
        query(SELECT_EARLIEST_ACTIVITY_WITHOUT_TRACK)
            .map(|row: DBRow| Self::row_to_activity(&row))
            .fetch_optional(executor)
            .await
    }

     pub async fn select_stats<'e, E>(executor: E) -> Result<ActivityStats>
         where E: DbExecutor<'e> {
         debug!("Execute\n{}", SELECT_ACTIVITY_STATS);
         query(SELECT_ACTIVITY_STATS)
             .map(|row: DBRow| {
                 let act_cnt : u32 = row.get(0);
                 let act_min : Option<String> = row.get(1);
                 let act_max : Option<String> = row.get(2);
                 let trk_cnt: u32 = row.get(3);
                 let trk_max: Option<String> = row.get(4);
                 ActivityStats::new(act_cnt, act_min, act_max, trk_cnt, trk_max)
             })
             .fetch_one(executor)
             .await
    }

    async fn execute_for_activity<'e, E>(executor: E, sql: &str, activity: &Activity) -> Result<()>
        where E: DbExecutor<'e> {
        debug!("Execute\n{}\nwith: {:?}", sql, activity);
        // Because sqlite does not support DECIMAL and stores FLOATs with many digits after the
        // dot (https://www.sqlite.org/floatingpoint.html), we need to convert the numbers to int.
        // The inverse operations are done by row_to_activity() below.
        query(sql)
            .bind(activity.id as i64) // sqlx::sqlite cannot encode u64
            .bind(activity.name.clone())
            .bind(activity.sport_type.clone())
            .bind(activity.start_date.clone())
            .bind((activity.distance * 10.0) as i64)
            .bind(activity.moving_time as i64)
            .bind((activity.total_elevation_gain * 10.0) as i64)
            .bind((activity.average_speed * 1000.0) as i64)
            .bind(activity.kudos_count)
            .execute(executor)
            .await
            .map(|_| ()) // Ignore returned row count
    }

    fn row_to_activity(row: &DBRow) -> Activity {
        // Reverse the conversion of floats to integers done in function upsert:
        Activity {
            id: row.get(0),
            name: row.get(1),
            sport_type: row.get(2),
            start_date: row.get(3),
            distance: (row.get::<i64, _>(4) as f32 / 10.0),
            moving_time: row.get(5),
            total_elevation_gain: (row.get::<i64, _>(6) as f32 / 10.0),
            average_speed: (row.get::<i64, _>(7) as f32 / 1000.0),
            kudos_count: row.get(8)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::activity_table::ActivityTable;
    use crate::database::db_types::DBPool;
    use crate::domain::activity::Activity;
    use crate::domain::activity_stats::ActivityStats;
    use crate::domain::track_store_state::TrackStoreState;

    #[tokio::test]
    async fn test_insert() {
        let activity1 = Activity::dummy(1, "foo");
        let activity2 = Activity::dummy(2, "bar");
        let activity3 = Activity::dummy(1, "baz");

        let pool = create_connection_and_table().await;
        assert!(ActivityTable::insert(&pool, &activity1).await.is_ok());
        assert!(ActivityTable::insert(&pool, &activity2).await.is_ok());
        assert!(ActivityTable::insert(&pool, &activity3).await.is_err()); // activity3 collides with activity1

        let ref_activities = [&activity1, &activity2];
        check_results(&pool, &ref_activities).await;
        check_single_result(&pool, ref_activities[0]).await;
        check_single_result(&pool, ref_activities[1]).await;
    }

    #[tokio::test]
    async fn test_upsert() {
        let activity1 = Activity::dummy(1, "foo");
        let activity2 = Activity::dummy(2, "bar");
        let activity3 = Activity::dummy(1, "baz");

        let pool = create_connection_and_table().await;
        assert!(ActivityTable::upsert(&pool, &activity1).await.is_ok());
        assert!(ActivityTable::upsert(&pool, &activity2).await.is_ok());
        assert!(ActivityTable::upsert(&pool, &activity3).await.is_ok()); // activity3 overwrites activity1

        let ref_activities = [&activity3, &activity2];
        check_results(&pool, &ref_activities).await;
        check_single_result(&pool, ref_activities[0]).await;
        check_single_result(&pool, ref_activities[1]).await;
    }

    #[tokio::test]
    async fn test_delete() {
        let activity = Activity::dummy(1, "n/a");

        let pool = create_connection_and_table().await;
        assert!(ActivityTable::upsert(&pool, &activity).await.is_ok());
        let result = ActivityTable::delete(&pool, 1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        check_results(&pool, &[]).await;
    }

    #[tokio::test]
    async fn test_delete_missing() {
        let pool = create_connection_and_table().await;
        let result = ActivityTable::delete(&pool, 1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn test_update_fetched_column() {
        let pool = create_connection_and_table().await;
        assert!(ActivityTable::insert(&pool, &Activity::dummy(1, "foo")).await.is_ok());
        let result = ActivityTable::update_fetched_column(&pool, 1, TrackStoreState::Stored).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_update_fetched_column_missing() {
        let pool = create_connection_and_table().await;
        let result = ActivityTable::update_fetched_column(&pool, 1, TrackStoreState::Stored).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn test_earliest_without_track() {
        let activity1 = Activity::dummy(1, "2018-02-20T18:02:13Z");
        let activity2 = Activity::dummy(2, "2018-02-20T18:02:12Z");
        let activity3 = Activity::dummy(3, "2018-02-20T18:02:11Z");

        let pool = create_connection_and_table().await;
        ActivityTable::upsert(&pool, &activity1).await.unwrap();
        ActivityTable::upsert(&pool, &activity2).await.unwrap();
        ActivityTable::upsert(&pool, &activity3).await.unwrap();
        ActivityTable::update_fetched_column(&pool, 3, TrackStoreState::Stored).await.unwrap(); // Earliest activity already has a track
        ActivityTable::update_fetched_column(&pool, 2, TrackStoreState::Missing).await.unwrap(); // Same for missing track

        let result = ActivityTable::select_earliest_without_track(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(activity1));
    }

    #[tokio::test]
    async fn test_earliest_without_track_missing() {
        let pool = create_connection_and_table().await;
        let result = ActivityTable::select_earliest_without_track(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_all_with_track() {
        // Note: Inverse timely order:
        let activity1 = Activity::dummy(3, "2018-02-20T18:02:15Z");
        let activity2 = Activity::dummy(5, "2018-02-20T18:02:13Z");
        let activity3 = Activity::dummy(7, "2018-02-20T18:02:11Z");

        let pool = create_connection_and_table().await;
        ActivityTable::upsert(&pool, &activity1).await.unwrap();
        ActivityTable::upsert(&pool, &activity2).await.unwrap();
        ActivityTable::upsert(&pool, &activity3).await.unwrap();
        ActivityTable::update_fetched_column(&pool, 3, TrackStoreState::Stored).await.unwrap();
        ActivityTable::update_fetched_column(&pool, 7, TrackStoreState::Stored).await.unwrap();

        let result = ActivityTable::select_all_with_track(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![activity3, activity1]);
    }

    #[tokio::test]
    async fn test_select_stats() {
        let pool = create_connection_and_table().await;
        let mut tx = pool.begin().await.unwrap();
        ActivityTable::upsert(&pool, &Activity::dummy(1, "2018-02-20T18:02:13Z")).await.unwrap();
        ActivityTable::upsert(&pool, &Activity::dummy(3, "2018-02-20T18:02:15Z")).await.unwrap();
        ActivityTable::upsert(&pool, &Activity::dummy(2, "2018-02-20T18:02:12Z")).await.unwrap();
        ActivityTable::upsert(&pool, &Activity::dummy(1, "2018-02-20T18:02:11Z")).await.unwrap(); // Note: ID overwrite
        ActivityTable::update_fetched_column(&pool, 1, TrackStoreState::Stored).await.unwrap();

        let result = ActivityTable::select_stats(&mut *tx).await;
        assert!(result.is_ok());
        let reference = ActivityStats::new(3, Some("2018-02-20T18:02:11Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()), 1, Some("2018-02-20T18:02:11Z".to_string()));
        assert_eq!(result.unwrap(), reference);
    }

    #[tokio::test]
    async fn test_select_stats_missing() {
        let pool = create_connection_and_table().await;
        let result = ActivityTable::select_stats(&pool).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ActivityStats::new(0, None, None, 0, None));
    }

    async fn create_connection_and_table() -> DBPool {
        let pool = DBPool::connect("sqlite::memory:").await.unwrap();
        ActivityTable::create_table(&pool).await.unwrap();
        pool
    }

    async fn check_results(pool: &DBPool, ref_activities: &[&Activity]) {
        let activities = ActivityTable::select_all(pool).await;
        assert!(activities.is_ok());

        let activities = activities.unwrap();
        assert_eq!(activities.len(), ref_activities.len());

        for (index, &ref_activity) in ref_activities.iter().enumerate() {
            let activity = activities.get(index);
            assert_eq!(activity, Some(ref_activity));
        }
    }

    async fn check_single_result(pool: &DBPool, ref_activity: &Activity) {
        let activity = ActivityTable::select_by_id(pool, ref_activity.id).await;
        assert!(activity.is_ok());

        let activity = activity.unwrap();
        assert!(activity.is_some());
        assert_eq!(activity.unwrap(), *ref_activity);
    }
}
