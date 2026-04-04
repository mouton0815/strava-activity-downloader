use const_format::str_replace;
use log::{debug, trace};
use sqlx::{query, Result};
use crate::database::db_executor::DbExecutor;
use crate::domain::map_zoom::MapZoom;
use crate::domain::map_tile::MapTile;

const CREATE_TILE_TABLE : &str =
    "CREATE TABLE IF NOT EXISTS $table_name (
        x INTEGER NOT NULL,
        y INTEGER NOT NULL,
        activity_id INTEGER NOT NULL,
        activity_count INTEGER NOT NULL,
        PRIMARY KEY (x, y)
        FOREIGN KEY(activity_id) REFERENCES activity(id)
    )";

const UPSERT_TILE: &str =
    "INSERT INTO $table_name (x, y, activity_id, activity_count) \
     VALUES (?, ?, ?, 1) \
     ON CONFLICT(x, y) DO \
     UPDATE SET activity_count = activity_count + 1";

const DELETE_TILES : &str =
    "DELETE FROM $table_name";

const TILE_TABLE_14: &str = "maptile14";
const TILE_TABLE_17: &str = "maptile17";

// "Instantiations" for all supported zoom levels
const CREATE_TILE_TABLE_14 : &str = str_replace!(CREATE_TILE_TABLE, "$table_name", TILE_TABLE_14);
const CREATE_TILE_TABLE_17 : &str = str_replace!(CREATE_TILE_TABLE, "$table_name", TILE_TABLE_17);

const UPSERT_TILE_14 : &str = str_replace!(UPSERT_TILE, "$table_name", TILE_TABLE_14);
const UPSERT_TILE_17 : &str = str_replace!(UPSERT_TILE, "$table_name", TILE_TABLE_17);

const DELETE_TILES_14 : &str = str_replace!(DELETE_TILES, "$table_name", TILE_TABLE_14);
const DELETE_TILES_17 : &str = str_replace!(DELETE_TILES, "$table_name", TILE_TABLE_17);

pub struct MapTileTable;

impl MapTileTable {
    pub async fn create_table<'e, E>(executor: E, zoom: MapZoom) -> Result<()>
    where E: DbExecutor<'e>
    {
        let sql = match zoom {
            MapZoom::Level14 => CREATE_TILE_TABLE_14,
            MapZoom::Level17 => CREATE_TILE_TABLE_17
        };
        debug!("Execute\n{sql}");
        query(sql).execute(executor).await?;
        Ok(())
    }

    pub async fn upsert<'e, E>(executor: E, zoom: MapZoom, tile: &MapTile, activity_id: u64)
        -> Result<()>
    where E: DbExecutor<'e>
    {
        let sql = match zoom {
            MapZoom::Level14 => UPSERT_TILE_14,
            MapZoom::Level17 => UPSERT_TILE_17
        };
        trace!("Execute\n{}\nwith {}, {}, {}", sql, tile.get_x(), tile.get_y(), activity_id);
        query(sql)
            .bind(tile.get_x() as i64) // sqlx::sqlite cannot encode u64
            .bind(tile.get_y() as i64) // see https://docs.rs/sqlx/latest/sqlx/sqlite/types
            .bind(activity_id as i64)
            .execute(executor)
            .await
            .map(|_| ()) // Ignore returned row count
    }

    pub async fn delete_all<'e, E>(executor: E, zoom: MapZoom) -> Result<usize>
    where E: DbExecutor<'e>
    {
        let sql = match zoom {
            MapZoom::Level14 => DELETE_TILES_14,
            MapZoom::Level17 => DELETE_TILES_17
        };
        debug!("Execute\n{sql}");
        let result = query(sql).execute(executor).await?;
        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use humantime::format_duration;
    use std::time::Instant;
    use const_format::str_replace;
    use log::debug;
    use sqlx::{query, Row};
    use crate::database::activity_table::ActivityTable;
    use crate::database::db_executor::DbExecutor;
    use crate::database::db_types::{DBPool, DBRow};
    use crate::database::maptile_table::{MapTileTable, TILE_TABLE_14, TILE_TABLE_17};
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_zoom::MapZoom;

    const SELECT_TILES : &str =
        "SELECT x, y, activity_id, activity_count FROM $table_name ORDER BY x, y";

    const SELECT_TILES_14 : &str = str_replace!(SELECT_TILES, "$table_name", TILE_TABLE_14);
    const SELECT_TILES_17 : &str = str_replace!(SELECT_TILES, "$table_name", TILE_TABLE_17);

    #[derive(Debug, PartialEq)]
    pub struct MapTileRow {
        tile: MapTile,
        activity_id: i64,
        activity_count: u32
    }

    impl MapTileRow {
        pub fn new(tile: MapTile, activity_id: i64, activity_count: u32) -> Self {
            Self { tile, activity_id, activity_count }
        }

        pub fn get_tile(&self) -> &MapTile {
            &self.tile
        }
    }

    impl MapTileTable {
        /// Fetches all tiles for the given zoom level and bounds (if any)
        pub async fn select<'e, E>(executor: E, zoom: MapZoom) -> sqlx::Result<Vec<MapTileRow>>
        where E: DbExecutor<'e> {
            let sql = match zoom {
                    MapZoom::Level14 => SELECT_TILES_14,
                    MapZoom::Level17 => SELECT_TILES_17
            };
            debug!("Execute\n{}", sql);
            let timer = Instant::now();
            let tiles: Vec<MapTileRow> = query(sql)
                .map(|row: DBRow| {
                    MapTileRow::new(
                        MapTile::new(row.get(0), row.get(1)),
                        row.get(2),
                        row.get(3))
                })
                .fetch_all(executor)
                .await?;
            debug!("Select tiles for zoom {:?} took {}", zoom, format_duration(timer.elapsed()));
            Ok(tiles)
        }
    }

    const ZOOM: MapZoom = MapZoom::Level14;

    #[tokio::test]
    async fn test_upsert() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);
        let tile3 = MapTile::new(1, 1); // Identical to tile1
        let tile4 = MapTile::new(1, 1); // Ditto

        let pool = create_pool().await;
        ActivityTable::create_table(&pool).await.unwrap();
        MapTileTable::create_table(&pool, ZOOM).await.unwrap();

        ActivityTable::insert(&pool, &Activity::dummy(1, "foo")).await.unwrap();
        ActivityTable::insert(&pool, &Activity::dummy(2, "bar")).await.unwrap();

        assert!(MapTileTable::upsert(&pool, ZOOM, &tile1, 1).await.is_ok());
        assert!(MapTileTable::upsert(&pool, ZOOM, &tile2, 2).await.is_ok());
        assert!(MapTileTable::upsert(&pool, ZOOM, &tile3, 1).await.is_ok()); // tile3 is same as tile1
        assert!(MapTileTable::upsert(&pool, ZOOM, &tile4, 1).await.is_ok()); // Ditto

        check_results(&pool, ZOOM, vec![
            MapTileRow { tile: tile1, activity_id: 1, activity_count: 3 },
            MapTileRow { tile: tile2, activity_id: 2, activity_count: 1 }
        ]).await;
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = create_pool().await;
        ActivityTable::create_table(&pool).await.unwrap();
        MapTileTable::create_table(&pool, ZOOM).await.unwrap();

        ActivityTable::insert(&pool, &Activity::dummy(1, "foo")).await.unwrap();
        ActivityTable::insert(&pool, &Activity::dummy(2, "bar")).await.unwrap();
        MapTileTable::upsert(&pool, ZOOM, &MapTile::new(1, 1), 1).await.unwrap();
        MapTileTable::upsert(&pool, ZOOM, &MapTile::new(2, 2), 2).await.unwrap();

        assert!(MapTileTable::delete_all(&pool, ZOOM).await.is_ok());

        check_results(&pool, ZOOM, vec![]).await;
    }

    #[tokio::test]
    async fn test_select() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);

        let pool = create_pool().await;
        ActivityTable::create_table(&pool).await.unwrap();
        MapTileTable::create_table(&pool, ZOOM).await.unwrap();

        ActivityTable::insert(&pool, &Activity::dummy(1, "foo")).await.unwrap();
        MapTileTable::upsert(&pool, ZOOM, &tile1, 1).await.unwrap();
        MapTileTable::upsert(&pool, ZOOM, &tile2, 1).await.unwrap();

        check_results(&pool, ZOOM, vec![
            MapTileRow { tile: tile1, activity_id: 1, activity_count: 1 },
            MapTileRow { tile: tile2, activity_id: 1, activity_count: 1 }
        ]).await;
    }

    async fn create_pool() -> DBPool {
        DBPool::connect("sqlite::memory:").await.unwrap()
    }

    async fn check_results(pool: &DBPool, zoom: MapZoom, ref_tiles: Vec<MapTileRow>) {
        let tiles = MapTileTable::select(pool, zoom).await.unwrap();
        assert_eq!(tiles.len(), ref_tiles.len());
        for (index, ref_tile) in ref_tiles.iter().enumerate() {
            let tile = tiles.get(index);
            assert_eq!(tile, Some(ref_tile));
        }
    }
}