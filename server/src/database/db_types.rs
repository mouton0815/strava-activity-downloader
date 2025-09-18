use sqlx::{Sqlite, SqlitePool};
use sqlx::sqlite::SqliteRow;

// When migrating to Postgres, hopefully only this file has to be adapted.
pub type DbType = Sqlite;
pub type DBPool = SqlitePool;
pub type DBRow = SqliteRow;

