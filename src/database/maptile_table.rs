use log::debug;
use rusqlite::{Connection, params, Result, Transaction};
use crate::domain::map_tile::MapTile;

const CREATE_TILE_TABLE : &'static str =
    "CREATE TABLE IF NOT EXISTS maptile (
        x INTEGER NOT NULL,
        y INTEGER NOT NULL,
        activity_id INTEGER NOT NULL,
        activity_count INTEGER NOT NULL,
        PRIMARY KEY (x, y)
        FOREIGN KEY(activity_id) REFERENCES activity(id)
    )";

const UPSERT_TILE: &'static str =
    "INSERT INTO maptile (x, y, activity_id, activity_count) \
     VALUES (?, ?, ?, 1) \
     ON CONFLICT(x, y) DO \
     UPDATE SET activity_count = excluded.activity_count + 1";

// TODO: Deletion ... support it?

const SELECT_TILES : &'static str =
    "SELECT x, y, activity_id, activity_count FROM maptile ORDER BY x, y";

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

pub struct MapTileTable;

impl MapTileTable {
    pub fn create_table(conn: &Connection) -> Result<()> {
        // let sql = CREATE_TILE_TABLE.replace("maptile", "maptile14");
        debug!("Execute\n{}", CREATE_TILE_TABLE);
        conn.execute(CREATE_TILE_TABLE, [])?;
        Ok(())
    }

    pub fn upsert(tx: &Transaction, tile: &MapTile, activity_id: u64) -> Result<()> {
        let values = params![tile.get_x(), tile.get_y(), activity_id];
        tx.execute(UPSERT_TILE, values).map(|_| ()) // Ignore returned row count
    }

    pub fn select_all(tx: &Transaction) -> Result<Vec<MapTileRow>> {
        debug!("Execute\n{}", SELECT_TILES);
        let mut stmt = tx.prepare(SELECT_TILES)?;
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
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::database::activity_table::ActivityTable;
    use crate::database::maptile_table::{MapTileRow, MapTileTable};
    use crate::domain::activity::Activity;
    use crate::domain::map_tile::MapTile;

    #[test]
    fn test_upsert() {
        let tile1 = MapTile::new(1, 1);
        let tile2 = MapTile::new(2, 2);
        let tile3 = MapTile::new(1, 1); // Identical to tile1

        let mut conn = create_connection();
        assert!(ActivityTable::create_table(&conn).is_ok());
        assert!(MapTileTable::create_table(&conn).is_ok());

        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).is_ok());
        assert!(ActivityTable::insert(&tx, &Activity::dummy(2, "bar")).is_ok());

        assert!(MapTileTable::upsert(&tx, &tile1, 1).is_ok());
        assert!(MapTileTable::upsert(&tx, &tile2, 2).is_ok());
        assert!(MapTileTable::upsert(&tx, &tile3, 1).is_ok()); // tile3 overwrites tile1
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

        let tile_rows = MapTileTable::select_all(&tx);
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