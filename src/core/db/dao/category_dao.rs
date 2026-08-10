//! DAO de categorías. Superficie completa de la tabla `category` +
//! `manga_category`, ejercitada por `tests/dao_test`; el UI de categorías
//! aún no está conectado, así que el binario no referencia estas funciones.
#![allow(dead_code)]
use crate::core::error::DbError;
use crate::core::models::Category;
use rusqlite::{params, Connection, Row};

fn row_to_category(row: &Row) -> rusqlite::Result<Category> {
    let auto_download: i64 = row.get(4)?;
    Ok(Category {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        color: row.get(2)?,
        auto_download: auto_download != 0,
        created_at: row.get(3)?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Category>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, created_at, auto_download FROM category ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], row_to_category)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn add(conn: &Connection, name: &str, color: &str) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO category (name, color, auto_download, created_at)
         VALUES (?1,?2,0, strftime('%s','now'))",
        params![name, color],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, name: &str) -> Result<(), DbError> {
    conn.execute("UPDATE category SET name=?1 WHERE id=?2", params![name, id])?;
    Ok(())
}

pub fn set_color(conn: &Connection, id: i64, color: &str) -> Result<(), DbError> {
    conn.execute("UPDATE category SET color=?1 WHERE id=?2", params![color, id])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM category WHERE id=?1", params![id])?;
    Ok(())
}

pub fn assign(conn: &Connection, manga_id: i64, category_id: i64) -> Result<(), DbError> {
    conn.execute(
        "INSERT OR IGNORE INTO manga_category (manga_id, category_id) VALUES (?1,?2)",
        params![manga_id, category_id],
    )?;
    Ok(())
}

pub fn unassign(conn: &Connection, manga_id: i64, category_id: i64) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM manga_category WHERE manga_id=?1 AND category_id=?2",
        params![manga_id, category_id],
    )?;
    Ok(())
}

pub fn for_manga(conn: &Connection, manga_id: i64) -> Result<Vec<Category>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.color, c.created_at, c.auto_download
         FROM category c
         INNER JOIN manga_category mc ON mc.category_id = c.id
         WHERE mc.manga_id=?1
         ORDER BY c.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![manga_id], row_to_category)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}