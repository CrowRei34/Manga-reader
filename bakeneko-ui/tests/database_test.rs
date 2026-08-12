// tests/database_test.rs
use bakeneko::core::db::{Database, SCHEMA_SQL};
use rusqlite::Connection;

#[test]
fn schema_matches_dart_tables() {
    for table in ["manga", "chapter", "category", "manga_category", "history", "download"] {
        assert!(SCHEMA_SQL.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")),
            "falta tabla {table}");
    }
}

#[test]
fn open_memory_and_migrate_is_idempotent() {
    let db = Database::open(None).unwrap();
    db.migrate().unwrap();
    db.migrate().unwrap(); // idempotente
    assert_eq!(db.user_version().unwrap(), 1);
}

#[test]
fn open_file_creates_schema() {
    let dir = std::env::temp_dir().join("bakeneko-db-test");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("test.sqlite");
    let _ = std::fs::remove_file(&p);
    {
        let db = Database::open(Some(&p)).unwrap();
        db.migrate().unwrap();
    }
    let conn = Connection::open(&p).unwrap();
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table'", [], |r| r.get(0)).unwrap();
    assert!(n >= 6);
}
