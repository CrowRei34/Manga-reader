// tests/models_test.rs
use bakeneko::core::models::*;
use serde_json::json;

#[test]
fn manga_deserializes_with_camelcase() {
    let j = json!({
        "source": "MANGADEX", "url": "/title/abc", "title": "One Piece",
        "publicUrl": "https://mangadex.org/title/abc",
        "rating": 4.5, "isNsfw": false, "coverUrl": "http://c/1.jpg",
        "largeCoverUrl": "http://c/1l.jpg", "description": "d", "authors": ["Oda"],
        "state": "ONGOING", "chapters": []
    });
    let m: Manga = serde_json::from_value(j).unwrap();
    assert_eq!(m.source, "MANGADEX");
    assert_eq!(m.public_url.as_deref(), Some("https://mangadex.org/title/abc"));
    assert_eq!(m.rating, 4.5);
    assert_eq!(m.authors, vec!["Oda".to_string()]);
    assert_eq!(m.key(), "MANGADEX|/title/abc");
}

#[test]
fn manga_defaults_for_missing_optional() {
    let j = json!({"source": "MANGASEE", "url": "/u", "title": "T"});
    let m: Manga = serde_json::from_value(j).unwrap();
    assert_eq!(m.rating, 0.0);
    assert!(!m.is_nsfw);
    assert!(m.authors.is_empty());
    assert!(m.chapters.is_empty());
}

#[test]
fn chapter_upload_date_roundtrip() {
    let j = json!({"source":"MANGADEX","url":"/c1","title":"Cap 1","number":1.5,"volume":2,"uploadDate":1700000000000i64});
    let c: Chapter = serde_json::from_value(j).unwrap();
    assert_eq!(c.number, 1.5);
    assert_eq!(c.upload_date, 1700000000000);
}

#[test]
fn page_roundtrip() {
    let p = Page { source: "S".into(), url: "http://img/1.jpg".into(), preview: None };
    let v = serde_json::to_value(&p).unwrap();
    let back: Page = serde_json::from_value(v).unwrap();
    assert_eq!(back.url, "http://img/1.jpg");
}

#[test]
fn download_state_serde_lowercase() {
    let v = serde_json::json!("downloading");
    let s: DownloadState = serde_json::from_value(v).unwrap();
    assert_eq!(s, DownloadState::Downloading);
    assert_eq!(serde_json::to_value(&s).unwrap(), serde_json::json!("downloading"));
}
