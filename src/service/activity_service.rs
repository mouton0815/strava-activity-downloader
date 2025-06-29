use std::collections::HashMap;
use std::error::Error;
use axum::BoxError;
use log::{debug, info, warn};
use rusqlite::Connection;
use crate::{ActivityStream, write_gpx};
use crate::database::activity_table::ActivityTable;
use crate::database::maptile_table::{MapTileRow, MapTileTable};
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::gpx_store_state::GpxStoreState;
use crate::domain::map_tile_zoom::MapTileZoom;

type TileTableMap = HashMap<MapTileZoom, MapTileTable>;

pub struct ActivityService {
    connection: Connection,
    tile_tables: Option<TileTableMap>,
}

impl ActivityService {
    pub fn new(db_path: &str, store_tiles: bool) -> Result<Self, Box<dyn Error>> {
        let connection = Connection::open(db_path)?;
        ActivityTable::create_table(&connection)?;
        let mut tile_tables: Option<TileTableMap> = None;
        if store_tiles {
            let mut tables: TileTableMap = HashMap::new();
            for zoom in MapTileZoom::VALUES {
                let table = MapTileTable::new(zoom.clone());
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
        // ... then mark the corresponding activity
        self.mark_gpx(activity, GpxStoreState::Stored)?;
        // ... finally compute the tiles and store them
        self.save_tiles(activity.id, stream)?;
        Ok(())
    }

    pub fn mark_gpx(&mut self, activity: &Activity, state: GpxStoreState) -> Result<(), BoxError> {
        let tx = self.connection.transaction()?;
        let result = ActivityTable::update_gpx_column(&tx, activity.id, state)?;
        debug!("Marked 'GPX fetched' for activity {} with result {result}", activity.id);
        tx.commit()?;
        Ok(())
    }

    fn save_tiles(&mut self, activity_id: u64, stream: &ActivityStream) -> Result<(), BoxError> {
        if let Some(tile_tables) = &self.tile_tables {
            let tx = self.connection.transaction()?;
            for zoom in MapTileZoom::VALUES {
                let table = &tile_tables[&zoom];
                for tile in stream.to_tiles(zoom.value())? {
                    table.upsert(&tx, &tile, activity_id)?;
                }
            }
            tx.commit()?;
        }
        Ok(())
    }

    pub fn get_tiles(&mut self, zoom: MapTileZoom) -> Result<Vec<MapTileRow>, BoxError> {
        match &self.tile_tables {
            Some(tile_tables) => {
                let tx = self.connection.transaction()?;
                let results = tile_tables[&zoom].select_all(&tx)?;
                tx.commit()?;
                Ok(results)
            },
            None => {
                warn!("Tile storage disabled");
                Ok(vec![])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ActivityService;
    use crate::database::maptile_table::MapTileRow;
    use crate::domain::activity::{Activity, ActivityVec};
    use crate::domain::activity_stats::ActivityStats;
    use crate::domain::activity_stream::ActivityStream;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_tile_zoom::MapTileZoom;

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
        assert!(service.save_tiles(5, &stream1).is_ok());
        assert!(service.save_tiles(7, &stream2).is_ok());

        let results = service.get_tiles(MapTileZoom::ZOOM14);
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