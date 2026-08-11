//! DAO de descargas. `list` lo usa la pantalla de Descargas; el resto
//! (`upsert`, `list_by_state`, `update_progress`, `set_state`) lo invoca el
//! DownloadManager (ejercitado por `tests/download_manager_test`).
#![allow(dead_code)]
use crate::error::DbError;
use crate::models::{DownloadEntry, DownloadState};
use rusqlite::{params, Connection, Row};

fn ds_to_str(s: DownloadState) -> &'static str {
    match s {
        DownloadState::Idle => "idle",
        DownloadState::Queued => "queued",
        DownloadState::Downloading => "downloading",
        DownloadState::Done => "done",
        DownloadState::Error => "error",
    }
}

fn str_to_ds(s: &str) -> DownloadState {
    match s {
        "idle" => DownloadState::Idle,
        "queued" => DownloadState::Queued,
        "downloading" => DownloadState::Downloading,
        "done" => DownloadState::Done,
        "error" => DownloadState::Error,
        _ => DownloadState::Idle,
    }
}

fn row_to_entry(row: &Row) -> rusqlite::Result<DownloadEntry> {
    let state_txt: String = row.get(2)?;
    Ok(DownloadEntry {
        manga_id: row.get(0)?,
        chapter_url: row.get(1)?,
        state: str_to_ds(&state_txt),
        total_pages: row.get(3)?,
        done_pages: row.get(4)?,
    })
}

const SELECT_COLS: &str = "manga_id, chapter_url, state, total_pages, done_pages";

pub fn upsert(
    conn: &Connection,
    manga_id: i64,
    chapter_url: &str,
    state: DownloadState,
) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO download (manga_id, chapter_url, state, total_pages, done_pages)
         VALUES (?1,?2,?3,0,0)
         ON CONFLICT(manga_id, chapter_url) DO UPDATE SET state=excluded.state",
        params![manga_id, chapter_url, ds_to_str(state)],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<DownloadEntry>, DbError> {
    let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM download"))?;
    let rows = stmt.query_map([], row_to_entry)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_by_state(
    conn: &Connection,
    state: DownloadState,
) -> Result<Vec<DownloadEntry>, DbError> {
    let mut stmt =
        conn.prepare(&format!("SELECT {SELECT_COLS} FROM download WHERE state=?1"))?;
    let rows = stmt.query_map(params![ds_to_str(state)], row_to_entry)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn update_progress(
    conn: &Connection,
    manga_id: i64,
    chapter_url: &str,
    done: i32,
    total: i32,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE download SET done_pages=?1, total_pages=?2 WHERE manga_id=?3 AND chapter_url=?4",
        params![done, total, manga_id, chapter_url],
    )?;
    Ok(())
}

pub fn set_state(
    conn: &Connection,
    manga_id: i64,
    chapter_url: &str,
    state: DownloadState,
) -> Result<(), DbError> {
    conn.execute(
        "UPDATE download SET state=?1 WHERE manga_id=?2 AND chapter_url=?3",
        params![ds_to_str(state), manga_id, chapter_url],
    )?;
    Ok(())
}