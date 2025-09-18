use sqlx::Executor;
use crate::database::db_types::DbType;

// An sqlx Executor trait for the usage in function signatures. Such functions accept either
// an sqlx Pool, a Connection, or a Transaction.

// Because Rust's trait aliases are not yet stable, this is the closest we can get:
pub trait DbExecutor<'e>: Executor<'e, Database = DbType> {}
// Provide a blanket implementation that implements DbExecutor for any type that satisfies the bound:
impl<'e, T> DbExecutor<'e> for T where T: Executor<'e, Database = DbType> {}