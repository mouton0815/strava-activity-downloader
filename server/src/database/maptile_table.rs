use std::time::Instant;
use log::{debug, trace};
use humantime::format_duration;
use sqlx::{query, Result, Row};
use crate::database::db_executor::DbExecutor;
use crate::database::db_types::DBRow;
use crate::domain::map_zoom::MapZoom;
use crate::domain::map_tile::MapTile;
use crate::domain::map_tile_bounds::MapTileBounds;

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

const SELECT_TILES : &str =
    "SELECT x, y, activity_id, activity_count FROM $table_name ORDER BY x, y";

const SELECT_TILES_BOUNDED : &str =
    "SELECT x, y, activity_id, activity_count FROM $table_name WHERE (x BETWEEN ? AND ?) AND (y BETWEEN ? AND ?) ORDER BY x, y";

const DELETE_TILES : &str =
    "DELETE FROM $table_name";

#[derive(Debug, PartialEq)]
pub struct MapTileRow {
    tile: MapTile,
    activity_id: i64,
    activity_count: u32
}

type MapTileVec = Vec<MapTileRow>;

impl MapTileRow {
    pub fn new(tile: MapTile, activity_id: i64, activity_count: u32) -> Self {
        Self { tile, activity_id, activity_count }
    }

    pub fn get_tile(&self) -> &MapTile {
        &self.tile
    }
}

pub struct MapTileTable {
    table_name: String
}

impl MapTileTable {
    pub fn new(zoom: MapZoom) -> Self {
        let table_name = format!("maptile{}", zoom.value());
        Self { table_name }
    }

    pub async fn create_table<'e, E>(&self, executor: E) -> Result<()>
        where E: DbExecutor<'e> {
        let sql = self.get_sql(CREATE_TILE_TABLE);
        debug!("Execute\n{sql}");
        query(sql.as_str()).execute(executor).await?;
        Ok(())
    }

    pub async fn upsert<'e, E>(&self, executor: E, tile: &MapTile, activity_id: u64) -> Result<()>
        where E: DbExecutor<'e> {
        let sql = self.get_sql(UPSERT_TILE);
        trace!("Execute\n{}\nwith {}, {}, {}", sql, tile.get_x(), tile.get_y(), activity_id);
        query(sql.as_str())
            .bind(tile.get_x() as i64) // sqlx::sqlite cannot encode u64
            .bind(tile.get_y() as i64) // see https://docs.rs/sqlx/latest/sqlx/sqlite/types
            .bind(activity_id as i64)
            .execute(executor)
            .await
            .map(|_| ()) // Ignore returned row count
    }

    pub async fn select<'e, E>(&self, executor: E, bounds: Option<MapTileBounds>) -> Result<MapTileVec>
        where E: DbExecutor<'e> {
        let sql = match bounds {
            Some(_) => self.get_sql(SELECT_TILES_BOUNDED),
            None => self.get_sql(SELECT_TILES)
        };
        debug!("Execute\n{sql}");
        let query = match bounds {
            None => query(sql.as_str()),
            Some(bounds) => query(sql.as_str())
                .bind(bounds.x1 as i64)
                .bind(bounds.x2 as i64)
                .bind(bounds.y1 as i64)
                .bind(bounds.y2 as i64)
        };
        let timer = Instant::now();
        let tiles: MapTileVec = query
            .map(|row: DBRow| {
                MapTileRow::new(
                    MapTile::new(row.get(0), row.get(1)),
                    row.get(2),
                    row.get(3))
            })
            .fetch_all(executor)
            .await?;
        debug!("Select from {} took {}", self.table_name, format_duration(timer.elapsed()));
        Ok(tiles)
    }

    pub async fn delete_all<'e, E>(&self, executor: E) -> Result<usize>
        where E: DbExecutor<'e> {
        let sql = self.get_sql(DELETE_TILES);
        debug!("Execute\n{sql}");
        let result = query(sql.as_str()).execute(executor).await?;
        Ok(result.rows_affected() as usize)
    }

    fn get_sql(&self, sql: &str) -> String {
        sql.replace("$table_name", &self.table_name)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::activity_table::ActivityTable;
    use crate::database::db_types::DBPool;
    use crate::database::maptile_table::{MapTileRow, MapTileTable, MapTileVec};
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_tile_bounds::MapTileBounds;
    use crate::domain::map_zoom::MapZoom;

    #[tokio::test]
    async fn test_upsert() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);
        let tile3 = MapTile::new(1, 1); // Identical to tile1
        let tile4 = MapTile::new(1, 1); // Ditto

        let pool = create_pool().await;
        ActivityTable::create_table(&pool).await.unwrap();
        let tile_table = create_tile_table(&pool).await;

        ActivityTable::insert(&pool, &Activity::dummy(1, "foo")).await.unwrap();
        ActivityTable::insert(&pool, &Activity::dummy(2, "bar")).await.unwrap();

        assert!(tile_table.upsert(&pool, &tile1, 1).await.is_ok());
        assert!(tile_table.upsert(&pool, &tile2, 2).await.is_ok());
        assert!(tile_table.upsert(&pool, &tile3, 1).await.is_ok()); // tile3 is same as tile1
        assert!(tile_table.upsert(&pool, &tile4, 1).await.is_ok()); // Ditto

        let ref_tile_rows = vec![
            MapTileRow { tile: tile1, activity_id: 1, activity_count: 3 },
            MapTileRow { tile: tile2, activity_id: 2, activity_count: 1 }
        ];
        check_results(tile_table, &pool, ref_tile_rows).await;
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = create_pool().await;
        ActivityTable::create_table(&pool).await.unwrap();
        let tile_table = create_tile_table(&pool).await;

        ActivityTable::insert(&pool, &Activity::dummy(1, "foo")).await.unwrap();
        ActivityTable::insert(&pool, &Activity::dummy(2, "bar")).await.unwrap();
        tile_table.upsert(&pool, &MapTile::new(1, 1), 1).await.unwrap();
        tile_table.upsert(&pool, &MapTile::new(2, 2), 2).await.unwrap();

        assert!(tile_table.delete_all(&pool).await.is_ok());

        check_results(tile_table, &pool, vec![]).await;
    }

    #[tokio::test]
    async fn test_select_with_bounds() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);

        let pool = create_pool().await;
        ActivityTable::create_table(&pool).await.unwrap();
        let tile_table = create_tile_table(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        ActivityTable::insert(&mut *tx, &Activity::dummy(1, "foo")).await.unwrap();
        tile_table.upsert(&mut *tx, &tile1, 1).await.unwrap();
        tile_table.upsert(&mut *tx, &tile2, 1).await.unwrap();

        // 1) Select no tile
        let bounds = MapTileBounds::new(3, 3, 3, 3);
        let result = tile_table.select(&mut *tx, Some(bounds)).await;
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![]);

        // 2) Select upper-left tile only
        let bounds = MapTileBounds::new(0, 0, 1, 1);
        let result = tile_table.select(&mut *tx, Some(bounds)).await;
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![
            MapTileRow { tile: tile1.clone(), activity_id: 1, activity_count: 1 }
        ]);

        // 3) Select lower-right tile only
        let bounds = MapTileBounds::new(2, 2, 5, 5);
        let result = tile_table.select(&mut *tx, Some(bounds)).await;
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![
            MapTileRow { tile: tile2.clone(), activity_id: 1, activity_count: 1 }
        ]);

        // 4) Select both tiles
        let bounds = MapTileBounds::new(1, 1, 2, 2);
        let result = tile_table.select(&mut *tx, Some(bounds)).await;
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![
            MapTileRow { tile: tile1, activity_id: 1, activity_count: 1 },
            MapTileRow { tile: tile2, activity_id: 1, activity_count: 1 }
        ]);

        tx.commit().await.unwrap();
    }

    async fn create_pool() -> DBPool {
        DBPool::connect("sqlite::memory:").await.unwrap()
    }

    async fn create_tile_table(pool: &DBPool) -> MapTileTable {
        let tile_table = MapTileTable::new(MapZoom::Level14);
        tile_table.create_table(pool).await.unwrap();
        tile_table
    }

    async fn check_results(tile_table: MapTileTable, pool: &DBPool, ref_tile_rows: MapTileVec) {
        let tile_rows = tile_table.select(pool, None).await.unwrap();
        compare_results(tile_rows, ref_tile_rows);
    }

    fn compare_results(tile_rows: MapTileVec, ref_tile_rows: MapTileVec) {
        assert_eq!(tile_rows.len(), ref_tile_rows.len());
        for (index, ref_tile) in ref_tile_rows.iter().enumerate() {
            let tile = tile_rows.get(index);
            assert_eq!(tile, Some(ref_tile));
        }
    }
}