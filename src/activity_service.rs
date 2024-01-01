// TODO: Move to service package or similar

use std::error::Error;
use axum::BoxError;
use rusqlite::Connection;
use crate::database::activity_table::ActivityTable;
use crate::database::state_table::StateTable;
use crate::domain::activity::ActivityVec;

pub struct ActivityService {
    connection: Connection
}

impl ActivityService {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let connection = Connection::open(db_path)?;
        ActivityTable::create_table(&connection)?;
        StateTable::create_table(&connection)?;
        Ok(Self{ connection })
    }

    pub fn add(&mut self, activities: &ActivityVec) -> Result<(), BoxError> {
        let tx = self.connection.transaction()?;
        for activity in activities {
            ActivityTable::upsert(&tx, activity)?;
        }
        tx.commit()?;
        Ok(())
    }
}