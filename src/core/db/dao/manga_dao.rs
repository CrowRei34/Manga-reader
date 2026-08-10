use crate::core::error::DbError;
use crate::core::models::Manga;
use rusqlite::{params, Connection, Row};

const SELECT_COLS: &str =
    "id, source, url, title, cover_url, description, blob_json";

fn row_to_manga(row: &Row) -> rusqlite::Result<Manga> {
    let blob_json: String = row.get(6)?;
    let blob: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&blob_json).unwrap_or_default();
    Ok(Manga {
        source: row.get(1)?,
        url: row.get(2)?,
        title: row.get(3)?,
        public_url: None,
        rating: 0.0,
        is_nsfw: false,
        cover_url: row.get(4)?,
        large_cover_url: None,
        description: row.get(5)?,
        authors: vec![],
        state: None,
        chapters: vec![],
        blob,
    })
}

pub fn upsert(conn: &Connection, m: &Manga, added_at: i64) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO manga (source, url, title, cover_url, description, blob_json, added_at, library)
         VALUES (?1,?2,?3,?4,?5,?6,?7,0)
         ON CONFLICT(source, url) DO UPDATE SET
            title=excluded.title,
            cover_url=excluded.cover_url,
            description=excluded.description,
            blob_json=excluded.blob_json",
        params![m.source, m.url, m.title, m.cover_url, m.description, m.blob_json(), added_at],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM manga WHERE source=?1 AND url=?2",
        params![m.source, m.url],
        |r| r.get(0),
    )?;
    Ok(id)
}

/// Búsqueda por (source, url); usada por `dao_test` y flujos futuros.
#[allow(dead_code)]
pub fn get_by_key(conn: &Connection, source: &str, url: &str) -> Result<Option<Manga>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM manga WHERE source=?1 AND url=?2"
    ))?;
    let mut rows = stmt.query(params![source, url])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_manga(row)?)),
        None => Ok(None),
    }
}

pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Manga>, DbError> {
    let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM manga WHERE id=?1"))?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_manga(row)?)),
        None => Ok(None),
    }
}

pub fn get_id_by_key(conn: &Connection, source: &str, url: &str) -> Result<i64, DbError> {
    let id: i64 = conn.query_row(
        "SELECT id FROM manga WHERE source=?1 AND url=?2",
        params![source, url],
        |r| r.get(0),
    )?;
    Ok(id)
}

pub fn list_library(conn: &Connection) -> Result<Vec<Manga>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM manga WHERE library=1 ORDER BY title COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map([], row_to_manga)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn set_library_flag(conn: &Connection, id: i64, in_library: bool) -> Result<(), DbError> {
    conn.execute(
        "UPDATE manga SET library=?1 WHERE id=?2",
        params![if in_library { 1 } else { 0 }, id],
    )?;
    Ok(())
}

/// Quita un manga de la librería/BD (UI pendiente).
#[allow(dead_code)]
pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM manga WHERE id=?1", params![id])?;
    Ok(())
}