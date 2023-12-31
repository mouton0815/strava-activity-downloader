use log::debug;
use rusqlite::{Connection, OptionalExtension, params, Result, Transaction};

// The rowId is a helper column to do upserts using "ON CONFLICT". Its value is always 1.
const CREATE_STATE_TABLE: &'static str =
    "CREATE TABLE IF NOT EXISTS state (
        rowId INTEGER NOT NULL PRIMARY KEY,
        timestamp TEXT NOT NULL
    )";

const UPSERT_STATE: &'static str =
    "INSERT INTO state (rowId, timestamp) VALUES (1, ?)
        ON CONFLICT(rowId) DO
        UPDATE SET timestamp = excluded.timestamp";

const SELECT_STATE : &'static str =
    "SELECT timestamp FROM state WHERE rowId = 1";

// This is just a namespace to keep method names short
pub struct StateTable;

impl StateTable {
    pub fn create_table(conn: &Connection) -> Result<()> {
        debug!("Execute\n{}", CREATE_STATE_TABLE);
        conn.execute(CREATE_STATE_TABLE, [])?;
        Ok(())
    }

    pub fn upsert(tx: &Transaction, timestamp: &str) -> Result<()> {
        debug!("Execute\n{} with: {}", UPSERT_STATE, timestamp);
        tx.execute(UPSERT_STATE, params![timestamp])?;
        Ok(())
    }

    pub fn select(tx: &Transaction) -> Result<Option<String>> {
        let mut stmt = tx.prepare(SELECT_STATE)?;
        stmt.query_row([], |row | {
            Ok(row.get(0)?)
        }).optional()
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::database::state_table::StateTable;

    #[test]
    fn test_upsert_initial() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        assert!(StateTable::upsert(&tx, "foo").is_ok());
        assert!(tx.commit().is_ok());

        check_result(&mut conn, "foo");
    }

    #[test]
    fn test_upsert_conflict() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        assert!(StateTable::upsert(&tx, "foo").is_ok());
        assert!(StateTable::upsert(&tx, "bar").is_ok());
        assert!(tx.commit().is_ok());

        check_result(&mut conn, "bar");
    }

    #[test]
    fn test_select_empty() {
        let mut conn = create_connection_and_table();
        let tx = conn.transaction().unwrap();
        let state = StateTable::select(&tx);
        assert!(tx.commit().is_ok());
        assert!(state.is_ok());
        assert!(state.unwrap().is_none());
    }

    fn create_connection_and_table() -> Connection {
        let conn = Connection::open(":memory:");
        assert!(conn.is_ok());
        let conn = conn.unwrap();
        assert!(StateTable::create_table(&conn).is_ok());
        conn
    }

    fn check_result(conn: &mut Connection, reference: &str) {
        let tx = conn.transaction().unwrap();
        let timestamp = StateTable::select(&tx);
        assert!(tx.commit().is_ok());
        assert!(timestamp.is_ok());
        let timestamp = timestamp.unwrap();
        assert!(timestamp.is_some());
        assert_eq!(timestamp.unwrap(), reference);
    }
}
