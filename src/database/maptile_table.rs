use std::marker::PhantomData;
use log::debug;
use rusqlite::{Connection, params, Result, Transaction};
use crate::domain::map_tile::MapTile;

trait TileTableName {
    fn table_name() -> &'static str;
}

pub struct Zoom14;

impl Zoom14 { pub const ZOOM: u16 = 14; }

impl TileTableName for Zoom14 {
    fn table_name() -> &'static str { "maptile14" }
}

pub struct Zoom17;

impl Zoom17 { pub const ZOOM: u16 = 17; }

impl TileTableName for Zoom17 {
    fn table_name() -> &'static str { "maptile17" }
}

const CREATE_TILE_TABLE : &'static str =
    "CREATE TABLE IF NOT EXISTS $table_name (
        x INTEGER NOT NULL,
        y INTEGER NOT NULL,
        activity_id INTEGER NOT NULL,
        activity_count INTEGER NOT NULL,
        PRIMARY KEY (x, y)
        FOREIGN KEY(activity_id) REFERENCES activity(id)
    )";

const UPSERT_TILE: &'static str =
    "INSERT INTO $table_name (x, y, activity_id, activity_count) \
     VALUES (?, ?, ?, 1) \
     ON CONFLICT(x, y) DO \
     UPDATE SET activity_count = excluded.activity_count + 1";

// TODO: Deletion ... support it?

const SELECT_TILES : &'static str =
    "SELECT x, y, activity_id, activity_count FROM $table_name ORDER BY x, y";

#[derive(Debug, PartialEq)]
pub struct MapTileRow { // TODO: Better name
    tile: MapTile,
    activity_id: i64,
    activity_count: u32
}

impl MapTileRow {
    pub fn new(tile: MapTile, activity_id: i64, activity_count: u32) -> Self {
        Self { tile, activity_id, activity_count }
    }
}

pub struct MapTileTable<T: TileTableName> {
    _marker: PhantomData<T> // Otherwise Rust complains about unused generic type T
}

impl<T: TileTableName> MapTileTable<T> {
    pub fn create_table(conn: &Connection) -> Result<()> {
        let sql = Self::get_sql(CREATE_TILE_TABLE);
        debug!("Execute\n{}", sql);
        conn.execute(sql.as_str(), [])?;
        Ok(())
    }

    pub fn upsert(tx: &Transaction, tile: &MapTile, activity_id: u64) -> Result<()> {
        let sql = Self::get_sql(UPSERT_TILE);
        let values = params![tile.get_x(), tile.get_y(), activity_id];
        debug!("Execute\n{}\nwith {}, {}, {}", sql, tile.get_x(), tile.get_y(), activity_id);
        tx.execute(sql.as_str(), values).map(|_| ()) // Ignore returned row count
    }

    pub fn select_all(tx: &Transaction) -> Result<Vec<MapTileRow>> {
        let sql = Self::get_sql(SELECT_TILES);
        debug!("Execute\n{}", sql);
        let mut stmt = tx.prepare(sql.as_str())?;
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

    fn get_sql(sql: &str) -> String {
        sql.replace("$table_name", T::table_name())
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::database::activity_table::ActivityTable;
    use crate::database::maptile_table::{MapTileRow, MapTileTable, Zoom14};
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;

    #[test]
    fn test_upsert() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);
        let tile3 = MapTile::new(1, 1); // Identical to tile1

        let mut conn = create_connection();
        assert!(ActivityTable::create_table(&conn).is_ok());
        assert!(MapTileTable::<Zoom14>::create_table(&conn).is_ok());

        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).is_ok());
        assert!(ActivityTable::insert(&tx, &Activity::dummy(2, "bar")).is_ok());

        assert!(MapTileTable::<Zoom14>::upsert(&tx, &tile1, 1).is_ok());
        assert!(MapTileTable::<Zoom14>::upsert(&tx, &tile2, 2).is_ok());
        assert!(MapTileTable::<Zoom14>::upsert(&tx, &tile3, 1).is_ok()); // tile3 overwrites tile1
        assert!(tx.commit().is_ok());

        let ref_tile_rows = [
            &MapTileRow { tile: tile1, activity_id: 1, activity_count: 2 },
            &MapTileRow { tile: tile2, activity_id: 2, activity_count: 1 }
        ];
        check_results(&mut conn, &ref_tile_rows);
    }

    fn create_connection() -> Connection {
        let conn = Connection::open(":memory:");
        assert!(conn.is_ok());
        conn.unwrap()
    }

    fn check_results(conn: &mut Connection, ref_tile_rows: &[&MapTileRow]) {
        let tx = conn.transaction().unwrap();

        let tile_rows = MapTileTable::<Zoom14>::select_all(&tx);
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