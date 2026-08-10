pub mod dao;
pub mod schema;

use crate::core::error::DbError;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
pub use schema::SCHEMA_SQL;

pub struct Database { conn: Arc<Mutex<Connection>> }

impl Database {
    pub fn open(path: Option<&Path>) -> Result<Self, DbError> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn migrate(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if v < 1 {
            conn.execute_batch(SCHEMA_SQL)?;
            conn.pragma_update(None, "user_version", 1)?;
        }
        Ok(())
    }

    pub fn user_version(&self) -> Result<i32, DbError> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> { self.conn.clone() }
}

pub async fn db_blocking<F, T>(db: Arc<Mutex<Connection>>, f: F) -> Result<T, DbError>
where F: FnOnce(&mut Connection) -> Result<T, DbError> + Send + 'static,
      T: Send + 'static {
    tokio::task::spawn_blocking(move || {
        let mut conn = db.lock().unwrap();
        f(&mut conn)
    })
    .await
    .map_err(|e| DbError::Join(e.to_string()))?
}
