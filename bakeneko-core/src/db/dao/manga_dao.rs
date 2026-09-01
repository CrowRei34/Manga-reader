use crate::error::DbError;
use crate::models::Manga;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

const SELECT_COLS: &str =
    "id, source, url, title, cover_url, description, blob_json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredMetadata {
    #[serde(default)] public_url: Option<String>,
    #[serde(default)] rating: f32,
    #[serde(default)] is_nsfw: bool,
    #[serde(default)] large_cover_url: Option<String>,
    #[serde(default)] authors: Vec<String>,
    #[serde(default)] state: Option<String>,
    #[serde(default)] blob: serde_json::Map<String, serde_json::Value>,
}

fn row_to_manga(row: &Row) -> rusqlite::Result<Manga> {
    let blob_json: String = row.get(6)?;
    let metadata: StoredMetadata = serde_json::from_str(&blob_json).unwrap_or_else(|_| {
        StoredMetadata {
            blob: serde_json::from_str(&blob_json).unwrap_or_default(),
            ..StoredMetadata::default()
        }
    });
    Ok(Manga {
        source: row.get(1)?,
        url: row.get(2)?,
        title: row.get(3)?,
        public_url: metadata.public_url,
        rating: metadata.rating,
        is_nsfw: metadata.is_nsfw,
        cover_url: row.get(4)?,
        large_cover_url: metadata.large_cover_url,
        description: row.get(5)?,
        authors: metadata.authors,
        state: metadata.state,
        chapters: vec![],
        blob: metadata.blob,
    })
}

pub fn upsert(conn: &Connection, m: &Manga, added_at: i64) -> Result<i64, DbError> {
    let metadata = serde_json::to_string(&StoredMetadata {
        public_url: m.public_url.clone(),
        rating: m.rating,
        is_nsfw: m.is_nsfw,
        large_cover_url: m.large_cover_url.clone(),
        authors: m.authors.clone(),
        state: m.state.clone(),
        blob: m.blob.clone(),
    }).map_err(|error| DbError::Join(format!("metadata JSON: {error}")))?;
    conn.execute(
        "INSERT INTO manga (source, url, title, cover_url, description, blob_json, added_at, library)
         VALUES (?1,?2,?3,?4,?5,?6,?7,0)
         ON CONFLICT(source, url) DO UPDATE SET
            title=excluded.title,
            cover_url=excluded.cover_url,
            description=excluded.description,
            blob_json=excluded.blob_json",
        params![m.source, m.url, m.title, m.cover_url, m.description, metadata, added_at],
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

pub fn list_library_by_category(conn: &Connection, category_id: i64) -> Result<Vec<Manga>, DbError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM manga m
         INNER JOIN manga_category mc ON mc.manga_id=m.id
         WHERE m.library=1 AND mc.category_id=?1
         ORDER BY m.title COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map(params![category_id], row_to_manga)?;
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
