use std::collections::HashMap;
use axum::BoxError;
use log::{debug, info, warn};
use crate::database::activity_table::ActivityTable;
use crate::database::db_types::DBPool;
use crate::database::maptile_table::MapTileTable;
use crate::domain::activity::{Activity, ActivityVec};
use crate::domain::activity_stats::ActivityStats;
use crate::domain::activity_stream::ActivityStream;
use crate::domain::map_tile::MapTile;
use crate::domain::map_tile_bounds::MapTileBounds;
use crate::domain::track_store_state::TrackStoreState;
use crate::domain::map_zoom::MapZoom;

type TileTableMap = HashMap<MapZoom, MapTileTable>;

pub struct ActivityService {
    pool: DBPool,
    tile_tables: Option<TileTableMap>,
}

impl ActivityService {
    pub async fn new(db_path: &str, store_tiles: bool) -> Result<Self, BoxError> {
        let pool = DBPool::connect(db_path).await?;
        ActivityTable::create_table(&pool).await?;
        let mut tile_tables: Option<TileTableMap> = None;
        if store_tiles {
            let mut tables: TileTableMap = HashMap::new();
            for zoom in MapZoom::VALUES {
                let table = MapTileTable::new(zoom);
                table.create_table(&pool).await?;
                tables.insert(zoom, table);
            }
            tile_tables = Some(tables);
        }
        Ok(Self{ pool, tile_tables })
    }

    /// Adds all activities to the database and returns the computed [ActivityStats]
    /// for **these** inserted activities (**not** for the entire database table).
    pub async fn add(&mut self, activities: &ActivityVec) -> Result<ActivityStats, BoxError> {
        info!("Add {} activities to database", activities.len());
        let act_count = activities.len() as u32;
        let mut min_time : Option<String> = None;
        let mut max_time : Option<String> = None;
        let mut tx = self.pool.begin().await?;
        for activity in activities {
            ActivityTable::insert(&mut *tx, activity).await?;
            // std::cmp::min for Option treats None as minimal value, but we need the timestamp
            min_time = Some(match min_time {
                Some(time) => std::cmp::min(activity.start_date.clone(), time),
                None => activity.start_date.clone()
            });
            max_time = std::cmp::max(Some(activity.start_date.clone()), max_time);
        }
        tx.commit().await?;
        Ok(ActivityStats::new(act_count, min_time, max_time, 0, None))
    }

    pub async fn get_stats(&mut self) -> Result<ActivityStats, BoxError> {
        let stats = ActivityTable::select_stats(&self.pool).await?;
        debug!("Read activity stats {:?} from database", stats);
        Ok(stats)
    }

    pub async fn get_earliest_without_track(&mut self) -> Result<Option<Activity>, BoxError> {
        let activity = ActivityTable::select_earliest_without_track(&self.pool).await?;
        debug!("Earliest activity without track: {:?}", activity);
        Ok(activity)
    }

    pub async fn get_all_with_track(&mut self) -> Result<ActivityVec, BoxError> {
        let activities = ActivityTable::select_all_with_track(&self.pool).await?;
        debug!("Number of activities with track: {:?}", activities.len());
        Ok(activities)
    }

    pub async fn mark_fetched(&mut self, activity: &Activity, state: TrackStoreState) -> Result<(), BoxError> {
        let result = ActivityTable::update_fetched_column(&self.pool, activity.id, state).await?;
        debug!("Marked 'GPX fetched' for activity {} with result {result}", activity.id);
        Ok(())
    }

    /// Derives and stores the tiles for all zoom levels from the given activity stream
    pub async fn store_tiles(&mut self, activity: &Activity, stream: &ActivityStream) -> Result<(), BoxError> {
        if let Some(_) = &self.tile_tables {
            for zoom in MapZoom::VALUES {
                let tiles = stream.to_tiles(zoom)?;
                self.put_tiles(zoom, activity.id, &tiles).await?;
            }
        }
        Ok(())
    }

    /// Stores tiles for the given zoom level
    pub async fn put_tiles(&mut self, zoom: MapZoom, activity_id: u64, tiles: &Vec<MapTile>) -> Result<(), BoxError> {
        match &self.tile_tables {
            Some(tile_tables) => {
                let mut tx = self.pool.begin().await?;
                let table = &tile_tables[&zoom];
                debug!("Save {} tiles with zoom level {} for activity {}", tiles.len(), zoom.value(), activity_id);
                for tile in tiles {
                    table.upsert(&mut *tx, &tile, activity_id).await?;
                }
                tx.commit().await?;
            }
            None => { // Check should happen in calling function
                warn!("Tile storage disabled");
            }
        }
        Ok(())
    }

    /// Returns all tiles for the given zoom level
    pub async fn get_tiles(&mut self, zoom: MapZoom, bounds: Option<MapTileBounds>) -> Result<Vec<MapTile>, BoxError> {
        match &self.tile_tables {
            Some(tile_tables) => {
                let results = tile_tables[&zoom]
                    .select(&self.pool, bounds).await?
                    .iter()
                    .map(|t| t.get_tile().clone())
                    .collect();
                Ok(results)
            },
            None => {
                warn!("Tile storage disabled");
                Ok(vec![])
            }
        }
    }

    /// Deletes **all** tiles for all zoom levels
    pub async fn delete_all_tiles(&mut self) -> Result<(), BoxError> {
        match &self.tile_tables {
            Some(tile_tables) => {
                for zoom in MapZoom::VALUES {
                    let table = &tile_tables[&zoom];
                    table.delete_all(&self.pool).await?;
                }
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
    use crate::domain::activity::{Activity, ActivityVec};
    use crate::domain::activity_stats::ActivityStats;
    use crate::domain::activity_stream::ActivityStream;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_zoom::MapZoom;
    use crate::service::activity_service::ActivityService;

    #[tokio::test]
    async fn test_add_none() {
        let vec = ActivityVec::new();
        let mut service = create_service().await;
        let stats = service.add(&vec).await;
        assert!(stats.is_ok());
        assert_eq!(stats.unwrap(), ActivityStats::new(0, None, None, 0, None));
    }

    #[tokio::test]
    async fn test_add_some() {
        let activities = vec![
            Activity::dummy(2, "2018-02-20T18:02:13Z"),
            Activity::dummy(1, "2018-02-20T18:02:15Z"),
            Activity::dummy(3, "2018-02-20T18:02:12Z"),
        ];
        let mut service = create_service().await;
        let stats = service.add(&activities).await;
        assert!(stats.is_ok());
        assert_eq!(stats.unwrap(), ActivityStats::new(
            3, Some("2018-02-20T18:02:12Z".to_string()), Some("2018-02-20T18:02:15Z".to_string()), 0, None));
        //assert_eq!(stats.unwrap(), Some(1519149735)); // 2018-02-20T18:02:12Z
    }

    #[tokio::test]
    async fn test_store_tiles() {
        let activities = vec![
            Activity::dummy(5, "2018-02-20T18:02:13Z"),
            Activity::dummy(7, "2018-02-20T18:02:15Z")
        ];

        // Contains a duplicate tile, which is filtered out by ActivityStream::to_tiles():
        let stream1 = ActivityStream::new(vec![(1.0, 1.0),(3.0, 3.0),(1.0, 1.0)], vec![], vec![]);
        // Contains a tile that is also part of stream1. It will be deduplicated by database upsert:
        let stream2 = ActivityStream::new(vec![(2.0, 2.0),(1.0, 1.0)], vec![], vec![]);

        let mut service = create_service().await;
        assert!(service.add(&activities).await.is_ok());
        assert!(service.store_tiles(&activities[0], &stream1).await.is_ok());
        assert!(service.store_tiles(&activities[1], &stream2).await.is_ok());

        let results = service.get_tiles(MapZoom::Level14, None).await;
        assert!(results.is_ok());
        assert_eq!(results.unwrap(), vec![
            MapTile::new(8237, 8146), // [1.0, 1.0]
            MapTile::new(8283, 8100), // [2.0, 2.0]
            MapTile::new(8328, 8055)  // [3.0, 3.0]
        ]);
    }

    async fn create_service() -> ActivityService {
        ActivityService::new("sqlite::memory:", true).await.unwrap()
    }
}