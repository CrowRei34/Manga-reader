// tests/dao_test.rs
use bakeneko::core::db::{Database, SCHEMA_SQL};
use bakeneko::core::db::dao::{manga_dao, chapter_dao, history_dao, category_dao, download_dao};
use bakeneko::core::models::{Chapter, DownloadState, Manga};
use rusqlite::Connection;

fn setup() -> Connection {
    // Sanity check: Database still builds/migrates, mirroring the original brief,
    // before dropping a fresh independent in-memory Connection for DAO tests.
    {
        let _db = Database::open(None).unwrap();
        // migrated below
    }
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(SCHEMA_SQL).unwrap();
    let _ = conn.pragma_update(None, "user_version", 1);
    conn
}

fn sample_manga(url: &str) -> Manga {
    Manga { source: "MANGADEX".into(), url: url.into(), title: "T".into(), public_url: None,
        rating: 0.0, is_nsfw: false, cover_url: None, large_cover_url: None,
        description: None, authors: vec![], state: None, chapters: vec![], blob: Default::default() }
}

#[test]
fn manga_upsert_and_get_by_key() {
    let conn = setup();
    let m = sample_manga("/u1");
    let id = manga_dao::upsert(&conn, &m, 1700000000).unwrap();
    let got = manga_dao::get_by_key(&conn, "MANGADEX", "/u1").unwrap().unwrap();
    assert_eq!(got.title, "T");
    let id2 = manga_dao::upsert(&conn, &m, 1700000000).unwrap();
    assert_eq!(id, id2); // idempotente por UNIQUE(source,url)
}

#[test]
fn library_flag_roundtrip() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/lib"), 0).unwrap();
    manga_dao::set_library_flag(&conn, id, true).unwrap();
    let lib = manga_dao::list_library(&conn).unwrap();
    assert_eq!(lib.len(), 1);
}

#[test]
fn chapters_replace_and_mark_read() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/ch"), 0).unwrap();
    let ch = Chapter { source: "MANGADEX".into(), url: "/c1".into(), title: "Cap 1".into(), number: 1.0,
        volume: 1, scanlator: None, upload_date: 0, branch: None, blob: Default::default(), read: false };
    chapter_dao::replace_for_manga(&conn, id, &[ch]).unwrap();
    let list = chapter_dao::list_for_manga(&conn, id).unwrap();
    assert_eq!(list.len(), 1);
    chapter_dao::mark_read(&conn, id, "/c1", true).unwrap();
    assert!(chapter_dao::list_for_manga(&conn, id).unwrap()[0].read);
}

#[test]
fn history_upsert_recent() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/h"), 0).unwrap();
    history_dao::upsert(&conn, id, 2, 5, 200).unwrap();
    history_dao::upsert(&conn, id, 3, 0, 300).unwrap(); // misma manga, último gana
    let rec = history_dao::recent(&conn, 10).unwrap();
    assert_eq!(rec.len(), 1);
    assert_eq!(rec[0], (id, 3, 0, 300));
}

#[test]
fn categories_and_assignment() {
    let conn = setup();
    let cid = category_dao::add(&conn, "Favoritos", "#ff0000").unwrap();
    let id = manga_dao::upsert(&conn, &sample_manga("/cat"), 0).unwrap();
    category_dao::assign(&conn, id, cid).unwrap();
    let cats = category_dao::for_manga(&conn, id).unwrap();
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0].name, "Favoritos");
}

#[test]
fn download_states_transition() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/dl"), 0).unwrap();
    download_dao::upsert(&conn, id, "/c1", DownloadState::Queued).unwrap();
    download_dao::update_progress(&conn, id, "/c1", 2, 10).unwrap();
    let e = download_dao::list(&conn).unwrap();
    assert_eq!(e[0].done_pages, 2);
    assert_eq!(e[0].total_pages, 10);
    download_dao::set_state(&conn, id, "/c1", DownloadState::Done).unwrap();
    assert_eq!(download_dao::list_by_state(&conn, DownloadState::Done).unwrap().len(), 1);
}