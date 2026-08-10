use crate::core::error::DbError;
use rusqlite::{params, Connection};

pub fn upsert(
    conn: &Connection,
    manga_id: i64,
    chapter_index: i32,
    page_index: i32,
    updated_at: i64,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO history (manga_id, chapter_index, page_index, updated_at)
         VALUES (?1,?2,?3,?4)
         ON CONFLICT(manga_id) DO UPDATE SET
            chapter_index=excluded.chapter_index,
            page_index=excluded.page_index,
            updated_at=excluded.updated_at",
        params![manga_id, chapter_index, page_index, updated_at],
    )?;
    Ok(())
}

pub fn recent(conn: &Connection, limit: i64) -> Result<Vec<(i64, i32, i32, i64)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT manga_id, chapter_index, page_index, updated_at
         FROM history ORDER BY updated_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Borra el historial de un manga (UI pendiente).
#[allow(dead_code)]
pub fn delete(conn: &Connection, manga_id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM history WHERE manga_id=?1", params![manga_id])?;
    Ok(())
}