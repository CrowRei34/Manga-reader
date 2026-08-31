// Reducer tests for the global app::update function (Task 11).
use bakeneko::app::{Message, update, AppState};
use bakeneko::core::models::{PingReply, Source};

#[test]
fn daemon_started_sets_ready_and_error_none() {
    let mut s = AppState::default();
    s.daemon_ready = false;
    let _task = update(
        &mut s,
        Message::DaemonStarted(Ok(PingReply { version: "1.0.0".into(), java: "21".into() })),
    );
    assert!(s.daemon_ready);
    assert!(s.error.is_none());
}

#[test]
fn daemon_started_error_sets_error() {
    let mut s = AppState::default();
    let _task = update(
        &mut s,
        Message::DaemonStarted(Err(bakeneko::core::error::DaemonError::Spawn("boom".into()))),
    );
    assert!(!s.daemon_ready);
    assert!(s.error.is_some());
}

#[test]
fn sources_listed_populates() {
    let mut s = AppState::default();
    let _task = update(
        &mut s,
        Message::SourcesListed(Ok(vec![Source { id: "MANGADEX".into(), name: "MangaDex".into(), language: Some("en".into()) }])),
    );
    assert_eq!(s.sources.len(), 1);
}

#[test]
fn navigate_to_changes_screen() {
    let mut s = AppState::default();
    let _task = update(&mut s, Message::NavigateTo(bakeneko::features::Screen::Browse));
    assert!(matches!(s.screen, bakeneko::features::Screen::Browse));
}
