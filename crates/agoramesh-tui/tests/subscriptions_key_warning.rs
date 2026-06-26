//! Integration tests for subscriptions, key UX, and first-seen warnings.
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test fixtures may fail fast on setup errors"
)]

use agoramesh_tui::app::{Action, AppState};
use agoramesh_tui::backend::Backend;
use agoramesh_tui::controller::handle_action;
use agoramesh_tui::events::map_event;
use agoramesh_tui::key_ux::generate_dev_key;
use agoramesh_tui::models::{
    AcknowledgedFirstSeen, CategorySummary, PeerStatus, Screen, Subscriptions,
};
use agoramesh_tui::subscriptions;
use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[track_caller]
fn temp_backend() -> (Backend, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|err| panic!("create tempdir: {err}"));
    let backend = Backend::open(Some(temp_dir.path().to_path_buf()), true)
        .unwrap_or_else(|err| panic!("open backend: {err}"));
    (backend, temp_dir)
}

#[test]
fn subscriptions_persist_and_reload() {
    let (backend, _temp_dir) = temp_backend();
    let path = backend.data_dir().join("subscriptions.json");
    let mut subscriptions = Subscriptions::default();
    subscriptions.category_ids.push("cat-a".to_owned());
    subscriptions.category_ids.push("cat-b".to_owned());
    subscriptions::save(&path, &subscriptions).expect("save");
    let loaded = subscriptions::load(&path).expect("load");
    assert_eq!(
        loaded.category_ids,
        vec!["cat-a".to_owned(), "cat-b".to_owned()]
    );
}

#[test]
fn key_panel_handles_missing_and_generated_keys() {
    let (backend, _temp_dir) = temp_backend();
    let status = backend.key_status(false).expect("missing key status");
    assert!(matches!(status, agoramesh_tui::models::KeyStatus::Missing));

    let status = generate_dev_key(&backend).expect("generate dev key");
    assert!(
        matches!(status, agoramesh_tui::models::KeyStatus::Present { .. }),
        "key should be present after generation"
    );
}

#[test]
fn first_seen_warnings_update_state_on_acknowledge() {
    let mut state = AppState::new();
    state.categories = vec![CategorySummary {
        object_id: "oid".to_owned(),
        display_name: "General".to_owned(),
        description: String::new(),
        category_id: "cat-general".to_owned(),
        created_at: Utc::now(),
    }];
    state.peers = vec![PeerStatus {
        name: None,
        address: "http://127.0.0.1:8080".to_owned(),
        last_sync_ok: None,
    }];
    state.warnings = agoramesh_tui::first_seen::compute_warnings(
        &state.categories,
        &state.peers,
        &state.acknowledged,
    );
    assert_eq!(state.warnings.len(), 2);

    let warning = state.warnings[0].clone();
    state = state.apply(Action::AcknowledgeWarning(warning));
    assert_eq!(state.warnings.len(), 1);
    assert_eq!(state.acknowledged.categories.len(), 1);
}

#[test]
fn first_seen_acknowledged_persists_and_reloads() {
    let (backend, _temp_dir) = temp_backend();
    let mut acknowledged = AcknowledgedFirstSeen::default();
    acknowledged.categories.push("cat-a".to_owned());
    acknowledged.peers.push("http://127.0.0.1:8080".to_owned());
    backend.save_acknowledged(&acknowledged).expect("save");
    let loaded = backend.load_acknowledged().expect("load");
    assert_eq!(loaded.categories, vec!["cat-a".to_owned()]);
    assert_eq!(loaded.peers, vec!["http://127.0.0.1:8080".to_owned()]);
}

#[test]
fn unknown_key_does_not_change_screen_to_feed() {
    let (backend, _temp_dir) = temp_backend();
    let mut state = AppState::new();
    state.screen = Screen::Subscriptions;

    let event = press(KeyCode::Char('x'));
    let action = map_event(&event, state.screen);
    if let Some(action) = action {
        handle_action(&backend, &mut state, action).expect("handle action");
    }

    assert_eq!(state.screen, Screen::Subscriptions);
}

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
