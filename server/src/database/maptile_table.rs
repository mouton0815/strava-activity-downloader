use std::time::Instant;
use log::{debug, trace};
use humantime::format_duration;
use rusqlite::{Connection, params, Result, ToSql, Transaction};
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

    pub fn create_table(&self, conn: &Connection) -> Result<()> {
        let sql = self.get_sql(CREATE_TILE_TABLE);
        debug!("Execute\n{sql}");
        conn.execute(&sql, [])?;
        Ok(())
    }

    pub fn upsert(&self, tx: &Transaction, tile: &MapTile, activity_id: u64) -> Result<()> {
        let sql = self.get_sql(UPSERT_TILE);
        let values = params![tile.get_x(), tile.get_y(), activity_id];
        trace!("Execute\n{}\nwith {}, {}, {}", sql, tile.get_x(), tile.get_y(), activity_id);
        tx.execute(&sql, values).map(|_| ()) // Ignore returned row count
    }

    pub fn select(&self, tx: &Transaction, bounds: Option<MapTileBounds>) -> Result<MapTileVec> {
        let sql = match bounds {
            Some(_) => self.get_sql(SELECT_TILES_BOUNDED),
            None => self.get_sql(SELECT_TILES)
        };
        let params: &[&dyn ToSql] = match bounds {
            Some(ref bounds) => &[&bounds.x1, &bounds.x2, &bounds.y1, &bounds.y2],
            None => &[]
        };
        let timer = Instant::now();
        let mut stmt = tx.prepare(&sql)?;
        let tile_iter = stmt.query_map(params, |row| {
            Ok(MapTileRow::new(
                MapTile::new(row.get(0)?, row.get(1)?),
                row.get(2)?,
                row.get(3)?
            ))
        })?;
        debug!("Execution took {}:\n{}", format_duration(timer.elapsed()), &sql);
        tile_iter.collect::<Result<MapTileVec, _>>()
    }

    pub fn delete_all(&self, tx: &Transaction) -> Result<usize> {
        let sql = self.get_sql(DELETE_TILES);
        debug!("Execute\n{sql}");
        tx.execute(&sql, params![])
    }

    fn get_sql(&self, sql: &str) -> String {
        sql.replace("$table_name", &self.table_name)
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::database::activity_table::ActivityTable;
    use crate::database::maptile_table::{MapTileRow, MapTileTable, MapTileVec};
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_tile_bounds::MapTileBounds;
    use crate::domain::map_zoom::MapZoom;

    #[test]
    fn test_upsert() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);
        let tile3 = MapTile::new(1, 1); // Identical to tile1
        let tile4 = MapTile::new(1, 1); // Ditto

        let mut conn = create_connection();
        ActivityTable::create_table(&conn).unwrap();
        let tile_table = create_tile_table(&conn);

        let tx = conn.transaction().unwrap();
        ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).unwrap();
        ActivityTable::insert(&tx, &Activity::dummy(2, "bar")).unwrap();

        assert!(tile_table.upsert(&tx, &tile1, 1).is_ok());
        assert!(tile_table.upsert(&tx, &tile2, 2).is_ok());
        assert!(tile_table.upsert(&tx, &tile3, 1).is_ok()); // tile3 is same as tile1
        assert!(tile_table.upsert(&tx, &tile4, 1).is_ok()); // Ditto
        tx.commit().unwrap();

        /*
        let ref_tile_rows = [
            &MapTileRow { tile: tile1, activity_id: 1, activity_count: 3 },
            &MapTileRow { tile: tile2, activity_id: 2, activity_count: 1 }
        ];
         */

        let ref_tile_rows = vec![
            MapTileRow { tile: tile1, activity_id: 1, activity_count: 3 },
            MapTileRow { tile: tile2, activity_id: 2, activity_count: 1 }
        ];
        check_results(tile_table, &mut conn, ref_tile_rows);
    }

    #[test]
    fn test_delete() {
        let mut conn = create_connection();
        ActivityTable::create_table(&conn).unwrap();
        let tile_table = create_tile_table(&conn);

        let tx = conn.transaction().unwrap();
        ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).unwrap();
        ActivityTable::insert(&tx, &Activity::dummy(2, "bar")).unwrap();
        tile_table.upsert(&tx, &MapTile::new(1, 1), 1).unwrap();
        tile_table.upsert(&tx, &MapTile::new(2, 2), 2).unwrap();

        assert!(tile_table.delete_all(&tx).is_ok());
        tx.commit().unwrap();

        check_results(tile_table, &mut conn, vec![]);
    }

    #[test]
    fn test_select_with_bounds() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);

        let mut conn = create_connection();
        ActivityTable::create_table(&conn).unwrap();
        let tile_table = create_tile_table(&conn);

        let tx = conn.transaction().unwrap();
        ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).unwrap();
        tile_table.upsert(&tx, &tile1, 1).unwrap();
        tile_table.upsert(&tx, &tile2, 1).unwrap();

        // 1) Select no tile
        let bounds = MapTileBounds::new(3, 3, 3, 3);
        let result = tile_table.select(&tx, Some(bounds));
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![]);

        // 2) Select upper-left tile only
        let bounds = MapTileBounds::new(0, 0, 1, 1);
        let result = tile_table.select(&tx, Some(bounds));
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![
            MapTileRow { tile: tile1.clone(), activity_id: 1, activity_count: 1 }
        ]);

        // 3) Select lower-right tile only
        let bounds = MapTileBounds::new(2, 2, 5, 5);
        let result = tile_table.select(&tx, Some(bounds));
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![
            MapTileRow { tile: tile2.clone(), activity_id: 1, activity_count: 1 }
        ]);

        // 4) Select both tiles
        let bounds = MapTileBounds::new(1, 1, 2, 2);
        let result = tile_table.select(&tx, Some(bounds));
        assert!(result.is_ok());
        compare_results(result.unwrap(), vec![
            MapTileRow { tile: tile1, activity_id: 1, activity_count: 1 },
            MapTileRow { tile: tile2, activity_id: 1, activity_count: 1 }
        ]);

        tx.commit().unwrap();
    }

    fn create_connection() -> Connection {
        Connection::open(":memory:").unwrap()
    }

    fn create_tile_table(conn: &Connection) -> MapTileTable {
        let tile_table = MapTileTable::new(MapZoom::Level14);
        tile_table.create_table(&conn).unwrap();
        tile_table
    }

    fn check_results(tile_table: MapTileTable, conn: &mut Connection, ref_tile_rows: MapTileVec) {
        let tx = conn.transaction().unwrap();
        let tile_rows = tile_table.select(&tx, None).unwrap();
        tx.commit().unwrap();
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