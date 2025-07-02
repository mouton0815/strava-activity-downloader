use log::{debug, trace};
use rusqlite::{Connection, params, Result, Transaction};
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

const SELECT_TILES : &str =
    "SELECT x, y, activity_id, activity_count FROM $table_name ORDER BY x, y";

const DELETE_TILES : &str =
    "DELETE FROM $table_name";

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

    pub fn select_all(&self, tx: &Transaction) -> Result<Vec<MapTileRow>> {
        let sql = self.get_sql(SELECT_TILES);
        debug!("Execute\n{sql}");
        let mut stmt = tx.prepare(&sql)?;
        let tile_iter = stmt.query_map([], |row| {
            Ok(MapTileRow::new(
                MapTile::new(row.get(0)?, row.get(1)?),
                row.get(2)?,
                row.get(3)?
            ))
        })?;
        let mut tile_vec: Vec<MapTileRow> = Vec::new();
        for tile in tile_iter {
            tile_vec.push(tile?)
        }
        Ok(tile_vec)
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
    use crate::database::maptile_table::{MapTileRow, MapTileTable};
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;
    use crate::domain::map_zoom::MapZoom;

    #[test]
    fn test_upsert() {
        let mut conn = create_connection();
        do_upsert(&mut conn);
    }

    #[test]
    fn test_delete() {
        let mut conn = create_connection();
        do_upsert(&mut conn);

        // Now delete everything again:
        let tile_table = MapTileTable::new(MapZoom::Level14);
        let tx = conn.transaction().unwrap();
        assert!(tile_table.delete_all(&tx).is_ok());
        assert!(tx.commit().is_ok());

        check_results(tile_table, &mut conn, &[]);
    }

    fn do_upsert(conn: &mut Connection) {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);
        let tile3 = MapTile::new(1, 1); // Identical to tile1
        let tile4 = MapTile::new(1, 1); // Ditto

        let tile_table = MapTileTable::new(MapZoom::Level14);
        assert!(ActivityTable::create_table(&conn).is_ok());
        assert!(tile_table.create_table(&conn).is_ok());

        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).is_ok());
        assert!(ActivityTable::insert(&tx, &Activity::dummy(2, "bar")).is_ok());

        assert!(tile_table.upsert(&tx, &tile1, 1).is_ok());
        assert!(tile_table.upsert(&tx, &tile2, 2).is_ok());
        assert!(tile_table.upsert(&tx, &tile3, 1).is_ok()); // tile3 is same as tile1
        assert!(tile_table.upsert(&tx, &tile4, 1).is_ok()); // Ditto
        assert!(tx.commit().is_ok());

        let ref_tile_rows = [
            &MapTileRow { tile: tile1, activity_id: 1, activity_count: 3 },
            &MapTileRow { tile: tile2, activity_id: 2, activity_count: 1 }
        ];
        check_results(tile_table, conn, &ref_tile_rows);
    }

    fn create_connection() -> Connection {
        let conn = Connection::open(":memory:");
        assert!(conn.is_ok());
        conn.unwrap()
    }

    fn check_results(tile_table: MapTileTable, conn: &mut Connection, ref_tile_rows: &[&MapTileRow]) {
        let tx = conn.transaction().unwrap();

        let tile_rows = tile_table.select_all(&tx);
        assert!(tile_rows.is_ok());
        assert!(tx.commit().is_ok());

        let tile_rows = tile_rows.unwrap();
        assert_eq!(tile_rows.len(), ref_tile_rows.len());

        for (index, &ref_tile) in ref_tile_rows.iter().enumerate() {
            let tile = tile_rows.get(index);
            assert_eq!(tile, Some(ref_tile));
        }
    }
}