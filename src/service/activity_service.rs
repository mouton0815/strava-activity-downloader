use std::collections::HashMap;
use axum::BoxError;
use log::{debug, info, warn};
use rusqlite::Connection;
use crate::database::activity_table::ActivityTable;
use crate::database::maptile_table::MapTileTable;
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::activity_stream::ActivityStream;
use crate::domain::map_tile::MapTile;
use crate::domain::track_store_state::TrackStoreState;
use crate::domain::map_zoom::MapZoom;
use crate::track::write_track::write_track;

type TileTableMap = HashMap<MapZoom, MapTileTable>;

pub struct ActivityService {
    connection: Connection,
    tile_tables: Option<TileTableMap>,
}

impl ActivityService {
    pub fn new(db_path: &str, store_tiles: bool) -> Result<Self, BoxError> {
        let connection = Connection::open(db_path)?;
        ActivityTable::create_table(&connection)?;
        let mut tile_tables: Option<TileTableMap> = None;
        if store_tiles {
            let mut tables: TileTableMap = HashMap::new();
            for zoom in MapZoom::VALUES {
                let table = MapTileTable::new(zoom);
                table.create_table(&connection)?;
                tables.insert(zoom, table);
            }
            tile_tables = Some(tables);
        }
        Ok(Self{ connection, tile_tables })
    }

    /// Adds all activities to the database and returns the computed [ActivityStats]
    /// for **these** inserted activities (**not** for the entire database table).
    pub fn add(&mut self, activities: &ActivityVec) -> Result<ActivityStats, BoxError> {
        info!("Add {} activities to database", activities.len());
        let tx = self.connection.transaction()?;
        let act_count = activities.len() as u32;
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
        Ok(ActivityStats::new(act_count, min_time, max_time, 0, None))
    }

    pub fn get_stats(&mut self) -> Result<ActivityStats, BoxError> {
        let tx = self.connection.transaction()?;
        let stats = ActivityTable::select_stats(&tx)?;
        tx.commit()?;
        debug!("Read activity stats {:?} from database", stats);
        Ok(stats)
    }

    pub fn get_earliest_without_track(&mut self) -> Result<Option<Activity>, BoxError> {
        let tx = self.connection.transaction()?;
        let activity = ActivityTable::select_earliest_without_track(&tx)?;
        tx.commit()?;
        debug!("Earliest activity without track: {:?}", activity);
        Ok(activity)
    }

    pub fn get_all_with_track(&mut self) -> Result<ActivityVec, BoxError> {
        let tx = self.connection.transaction()?;
        let activities = ActivityTable::select_all_with_track(&tx)?;
        tx.commit()?;
        debug!("Number of activities with track: {:?}", activities.len());
        Ok(activities)
    }

    pub fn store_track(&mut self, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
        // Store GPX file ...
        write_track(activity, stream)?;
        // ... then mark the corresponding activity
        self.mark_fetched(activity, TrackStoreState::Stored)?;
        // ... finally compute the tiles and store them
        self.put_tiles(activity, stream)?;
        Ok(())
    }

    pub fn mark_fetched(&mut self, activity: &Activity, state: TrackStoreState) -> Result<(), BoxError> {
        let tx = self.connection.transaction()?;
        let result = ActivityTable::update_fetched_column(&tx, activity.id, state)?;
        debug!("Marked 'GPX fetched' for activity {} with result {result}", activity.id);
        tx.commit()?;
        Ok(())
    }

    pub fn put_tiles(&mut self, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
        if let Some(tile_tables) = &self.tile_tables {
            let tx = self.connection.transaction()?;
            for zoom in MapZoom::VALUES {
                let table = &tile_tables[&zoom];
                let tiles = stream.to_tiles(zoom)?;
                debug!("Save {} tiles with zoom level {} for activity {}", tiles.len(), zoom.value(), activity.id);
                for tile in tiles {
                    table.upsert(&tx, &tile, activity.id)?;
                }
            }
            tx.commit()?;
        }
        Ok(())
    }

    /// Returns all tiles for the given zoom level
    pub fn get_tiles(&mut self, zoom: MapZoom) -> Result<Vec<MapTile>, BoxError> {
        match &self.tile_tables {
            Some(tile_tables) => {
                let tx = self.connection.transaction()?;
                let results = tile_tables[&zoom]
                    .select_all(&tx)?
                    .iter()
                    .map(|t| t.get_tile().clone())
                    .collect();
                tx.commit()?;
                Ok(results)
            },
            None => {
                warn!("Tile storage disabled");
                Ok(vec![])
            }
        }
    }

    /// Deletes **all** tiles for all zoom levels
    pub fn delete_all_tiles(&mut self) -> Result<(), BoxError> {
        match &self.tile_tables {
            Some(tile_tables) => {
                let tx = self.connection.transaction()?;
                for zoom in MapZoom::VALUES {
                    let table = &tile_tables[&zoom];
                    table.delete_all(&tx)?;
                }
                tx.commit()?;
            },
            None => {
                warn!("Tile storage disabled");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::maptile_table::MapTileRow;
    use crate::domain::activity::{Activity, ActivityVec};
    use crate::domain::activity_stats::ActivityStats;
    use crate::domain::activity_stream::ActivityStream;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_zoom::MapZoom;
    use crate::service::activity_service::ActivityService;

    #[test]
    fn test_add_none() {
        let vec = ActivityVec::new();
        let mut service = create_service();
        let stats = service.add(&vec);
        assert!(stats.is_ok());
        assert_eq!(stats.unwrap(), ActivityStats::new(0, None, None, 0, None));
    }

    #[test]
    fn test_add_some() {
        let activities = vec![
            Activity::dummy(2, "2018-02-20T18:02:13Z"),
            Activity::dummy(1, "2018-02-20T18:02:15Z"),
            Activity::dummy(3, "2018-02-20T18:02:12Z"),
        ];
        let mut service = create_service();
        let stats = service.add(&activities);
        assert!(stats.is_ok());
        assert_eq!(stats.unwrap(), ActivityStats::new(
            3, Some("2018-02-20T18:02:12Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()), 0, None));
        //assert_eq!(stats.unwrap(), Some(1519149735)); // 2018-02-20T18:02:12Z
    }

    #[test]
    fn test_store_tiles() {
        let activities = vec![
            Activity::dummy(5, "2018-02-20T18:02:13Z"),
            Activity::dummy(7, "2018-02-20T18:02:15Z")
        ];

        // Contains a duplicate tile, which is filtered out by ActivityStream::to_tiles():
        let stream1 = ActivityStream::new(vec![(1.0, 1.0),(3.0, 3.0),(1.0, 1.0)], vec![], vec![]);
        // Contains a tile that is also part of stream1. It will be deduplicated by database upsert:
        let stream2 = ActivityStream::new(vec![(2.0, 2.0),(1.0, 1.0)], vec![], vec![]);

        let mut service = create_service();
        assert!(service.add(&activities).is_ok());
        assert!(service.put_tiles(&activities[0], &stream1).is_ok());
        assert!(service.put_tiles(&activities[1], &stream2).is_ok());

        let results = service.get_tiles(MapZoom::Level14);
        assert!(results.is_ok());
        assert_eq!(results.unwrap(), vec![
            MapTileRow::new(MapTile::new(8237, 8146), 5, 2), // [1.0, 1.0]
            MapTileRow::new(MapTile::new(8283, 8100), 7, 1), // [2.0, 2.0]
            MapTileRow::new(MapTile::new(8328, 8055), 5, 1)  // [3.0, 3.0]
        ]);
    }

    fn create_service() -> ActivityService {
        let service = ActivityService::new(":memory:", true);
        assert!(service.is_ok());
        service.unwrap()
    }
}