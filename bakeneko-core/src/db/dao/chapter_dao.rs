//! DAO de capítulos. `list_for_manga` lo usa details/downloads; el resto
//! (`replace_for_manga`, `mark_read`, helpers) lo ejercita `tests/dao_test`.
#![allow(dead_code)]
use crate::error::DbError;
use crate::models::Chapter;
use rusqlite::{params, Connection, Row};

fn chapter_blob_json(c: &Chapter) -> String {
    if c.blob.is_empty() {
        serde_json::json!({}).to_string()
    } else {
        serde_json::Value::Object(c.blob.clone()).to_string()
    }
}

fn row_to_chapter(row: &Row) -> rusqlite::Result<Chapter> {
    let blob_json: String = row.get(4)?;
    let blob: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&blob_json).unwrap_or_default();
    let read_i: i64 = row.get(5)?;
    Ok(Chapter {
        source: String::new(),
        url: row.get(1)?,
        title: row.get(2)?,
        number: row.get(3)?,
        volume: 0,
        language: None,
        scanlator: None,
        upload_date: 0,
        branch: None,
        blob,
        read: read_i != 0,
    })
}

pub fn replace_for_manga(
    conn: &Connection,
    manga_id: i64,
    chapters: &[Chapter],
) -> Result<(), DbError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM chapter WHERE manga_id=?1", params![manga_id])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO chapter (manga_id, url, name, number, blob_json, read)
             VALUES (?1,?2,?3,?4,?5,0)",
        )?;
        for c in chapters {
            stmt.execute(params![manga_id, c.url, c.title, c.number, chapter_blob_json(c)])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn list_for_manga(conn: &Connection, manga_id: i64) -> Result<Vec<Chapter>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT manga_id, url, name, number, blob_json, read FROM chapter WHERE manga_id=?1 ORDER BY number",
    )?;
    let rows = stmt.query_map(params![manga_id], row_to_chapter)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn mark_read(
    conn: &Connection,
    manga_id: i64,
    url: &str,
    read: bool,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE chapter SET read=?1 WHERE manga_id=?2 AND url=?3",
        params![if read { 1 } else { 0 }, manga_id, url],
    )?;
    Ok(())
}