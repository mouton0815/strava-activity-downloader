use log::debug;
use rusqlite::{Connection, params, Result, Row, Transaction};
use crate::domain::tile::Tile;

const CREATE_TILE_TABLE : &'static str =
    "CREATE TABLE IF NOT EXISTS tile (
        x INTEGER NOT NULL,
        y INTEGER NOT NULL,
        activity_id INTEGER NOT NULL,
        activity_count INTEGER NOT NULL,
        PRIMARY KEY (x, y)
        FOREIGN KEY(activity_id) REFERENCES activity(id)
    )";

const UPSERT_TILE: &'static str =
    "INSERT INTO tile (x, y, activity_id, activity_count) \
     VALUES (?, ?, ?, 1) \
     ON CONFLICT(x, y) DO \
     UPDATE SET activity_count = excluded.activity_count + 1";

// TODO: Deletion ... support it?

const SELECT_TILES : &'static str =
    "SELECT x, y, activity_id, activity_count FROM tile";

#[derive(Debug, PartialEq)]
pub struct TileRow { // TODO: Better name
    tile: Tile,
    activity_id: i64,
    activity_count: u32
}

pub struct TileTable;

impl TileTable {
    pub fn create_table(conn: &Connection) -> Result<()> {
        debug!("Execute\n{}", CREATE_TILE_TABLE);
        conn.execute(CREATE_TILE_TABLE, [])?;
        Ok(())
    }

    pub fn upsert(tx: &Transaction, tile: &Tile, activity_id: i64) -> Result<()> {
        let values = params![tile.x, tile.y, activity_id];
        tx.execute(UPSERT_TILE, values).map(|_| ()) // Ignore returned row count
    }

    pub fn select_all(tx: &Transaction) -> Result<Vec<TileRow>> {
        debug!("Execute\n{}", SELECT_TILES);
        let mut stmt = tx.prepare(SELECT_TILES)?;
        let tile_iter = stmt.query_map([], |row| {
            Self::row_to_tile_row(row)
        })?;
        let mut tile_vec: Vec<TileRow> = Vec::new();
        for tile in tile_iter {
            tile_vec.push(tile?)
        }
        Ok(tile_vec)
    }

    fn row_to_tile_row(row: &Row) -> Result<TileRow> {
        // Reverse the conversion of floats to integers done in function upsert:
        Ok(TileRow {
            tile: Tile {
                x: row.get(0)?,
                y: row.get(1)?
            },
            activity_id: row.get(2)?,
            activity_count: row.get(3)?
        })
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::database::activity_table::ActivityTable;
    use crate::database::tile_table::{TileRow, TileTable};
    use crate::domain::activity::Activity;
    use crate::domain::tile::Tile;

    #[test]
    fn test_upsert() {
        let tile1 = Tile{ x: 1, y: 1 };
        let tile2 = Tile{ x: 2, y: 2 };
        let tile3 = Tile{ x: 1, y: 1 }; // Identical to tile1

        let mut conn = create_connection();
        assert!(ActivityTable::create_table(&conn).is_ok());
        assert!(TileTable::create_table(&conn).is_ok());

        let tx = conn.transaction().unwrap();
        assert!(ActivityTable::insert(&tx, &Activity::dummy(1, "foo")).is_ok());
        assert!(ActivityTable::insert(&tx, &Activity::dummy(2, "bar")).is_ok());

        assert!(TileTable::upsert(&tx, &tile1, 1).is_ok());
        assert!(TileTable::upsert(&tx, &tile2, 2).is_ok());
        assert!(TileTable::upsert(&tx, &tile3, 1).is_ok()); // tile3 overwrites tile1
        assert!(tx.commit().is_ok());

        let ref_tile_rows = [
            &TileRow{ tile: tile1, activity_id: 1, activity_count: 2 },
            &TileRow{ tile: tile2, activity_id: 2, activity_count: 1 }
        ];
        check_results(&mut conn, &ref_tile_rows);
    }

    fn create_connection() -> Connection {
        let conn = Connection::open(":memory:");
        assert!(conn.is_ok());
        conn.unwrap()
    }

    fn check_results(conn: &mut Connection, ref_tile_rows: &[&TileRow]) {
        let tx = conn.transaction().unwrap();

        let tile_rows = TileTable::select_all(&tx);
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