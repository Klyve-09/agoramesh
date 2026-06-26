//! Key Management controller integration tests.
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixtures may fail fast on setup errors"
)]

use agoramesh_tui::app::{Action, AppState};
use agoramesh_tui::backend::Backend;
use agoramesh_tui::controller::handle_action;
use agoramesh_tui::events::map_event;
use agoramesh_tui::key_ux::render_key_management;
use agoramesh_tui::models::{KeyStatus, Screen};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[test]
fn encrypted_key_generate_reopen_unlock_backup_restore() {
    let (backend, temp_dir) = temp_backend(false);
    let mut state = AppState::new();
    state.screen = Screen::KeyManagement;
    for ch in "correct horse".chars() {
        dispatch(&backend, &mut state, &press(KeyCode::Char(ch)));
    }
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('g')));
    assert!(matches!(state.key_status, KeyStatus::Present { .. }));
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('b')));

    let reopened = Backend::open(Some(temp_dir.path().to_path_buf()), false).expect("reopen");
    assert!(matches!(
        reopened.key_status(false).expect("locked"),
        KeyStatus::Locked { .. }
    ));
    let mut reopened_state = AppState::new();
    reopened_state.screen = Screen::KeyManagement;
    for ch in "correct horse".chars() {
        dispatch(&reopened, &mut reopened_state, &press(KeyCode::Char(ch)));
    }
    dispatch(&reopened, &mut reopened_state, &press(KeyCode::Enter));
    assert!(matches!(
        reopened_state.key_status,
        KeyStatus::Present { .. }
    ));
    std::fs::remove_file(temp_dir.path().join("identity.key")).expect("remove key");
    dispatch(&reopened, &mut reopened_state, &ctrl(KeyCode::Char('r')));
    assert!(temp_dir.path().join("identity.key").exists());
}

#[test]
fn encrypted_key_generate_does_not_overwrite_existing_key() {
    let (backend, _temp_dir) = temp_backend(false);
    let mut state = AppState::new();
    state.screen = Screen::KeyManagement;
    for ch in "first passphrase".chars() {
        dispatch(&backend, &mut state, &press(KeyCode::Char(ch)));
    }
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('g')));
    let first_public_key = public_key_hex(&state.key_status);

    for ch in "second passphrase".chars() {
        dispatch(&backend, &mut state, &press(KeyCode::Char(ch)));
    }
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('g')));

    assert_eq!(public_key_hex(&state.key_status), first_public_key);
    assert_eq!(
        state.status_message.as_deref(),
        Some("Key overwrite disabled; use backup/restore instead")
    );
}

#[test]
fn dev_plaintext_key_generate_does_not_overwrite_existing_key() {
    let (backend, _temp_dir) = temp_backend(true);
    let mut state = AppState::new();
    state.screen = Screen::KeyManagement;

    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('d')));
    let first_public_key = public_key_hex(&state.key_status);
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('d')));

    assert_eq!(public_key_hex(&state.key_status), first_public_key);
    assert_eq!(
        state.status_message.as_deref(),
        Some("Key overwrite disabled; use backup/restore instead")
    );
}

#[test]
fn backup_without_key_sets_status_and_does_not_exit() {
    let (backend, temp_dir) = temp_backend(true);
    let mut state = AppState::new();
    state.screen = Screen::KeyManagement;

    let result = handle_action(&backend, &mut state, Action::BackupKey);

    assert!(result.is_ok());
    assert_eq!(state.screen, Screen::KeyManagement);
    assert_status_contains(&state, "Backup failed");
    assert!(!temp_dir.path().join("identity.key").exists());
}

#[test]
fn restore_without_backup_sets_status_and_does_not_exit() {
    let (backend, _temp_dir) = temp_backend(true);
    let mut state = AppState::new();
    state.screen = Screen::KeyManagement;
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('d')));
    let existing_public_key = public_key_hex(&state.key_status);

    let action = map_event(&ctrl(KeyCode::Char('r')), state.screen).expect("restore maps");
    let result = handle_action(&backend, &mut state, action);

    assert!(result.is_ok());
    assert_eq!(state.screen, Screen::KeyManagement);
    assert_status_contains(&state, "Restore failed");
    assert_eq!(public_key_hex(&state.key_status), existing_public_key);
    assert_eq!(
        public_key_hex(&backend.key_status(false).expect("key status")),
        existing_public_key
    );
}

#[test]
fn restore_corrupt_backup_sets_status_and_preserves_existing_key() {
    let (backend, temp_dir) = temp_backend(false);
    let mut state = AppState::new();
    state.screen = Screen::KeyManagement;
    for ch in "correct horse".chars() {
        dispatch(&backend, &mut state, &press(KeyCode::Char(ch)));
    }
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('g')));
    let existing_public_key = public_key_hex(&state.key_status);
    std::fs::write(
        temp_dir.path().join("identity.key.backup"),
        b"not a key file",
    )
    .expect("write corrupt backup");

    let result = handle_action(&backend, &mut state, Action::RestoreKey);

    assert!(result.is_ok());
    assert_eq!(state.screen, Screen::KeyManagement);
    assert_status_contains(&state, "Restore failed");
    assert_eq!(public_key_hex(&state.key_status), existing_public_key);
    assert_eq!(
        public_key_hex(&backend.key_status(false).expect("key status")),
        existing_public_key
    );
}

#[test]
fn backup_write_failure_sets_status_and_does_not_exit() {
    let (backend, temp_dir) = temp_backend(true);
    let mut state = AppState::new();
    state.screen = Screen::KeyManagement;
    dispatch(&backend, &mut state, &ctrl(KeyCode::Char('d')));
    let backup_path = temp_dir.path().join("identity.key.backup");
    std::fs::create_dir(&backup_path).expect("create backup path collision");

    let result = handle_action(&backend, &mut state, Action::BackupKey);

    assert!(result.is_ok());
    assert_eq!(state.screen, Screen::KeyManagement);
    assert_status_contains(&state, "Backup failed");
    assert!(temp_dir.path().join("identity.key").exists());
}

#[test]
fn key_management_help_matches_event_bindings() {
    assert_eq!(
        map_event(&ctrl(KeyCode::Char('g')), Screen::KeyManagement),
        Some(Action::GenerateEncryptedKey)
    );
    assert_eq!(
        map_event(&ctrl(KeyCode::Char('d')), Screen::KeyManagement),
        Some(Action::GenerateDevKey)
    );
    assert_eq!(
        map_event(&ctrl(KeyCode::Char('b')), Screen::KeyManagement),
        Some(Action::BackupKey)
    );
    assert_eq!(
        map_event(&ctrl(KeyCode::Char('r')), Screen::KeyManagement),
        Some(Action::RestoreKey)
    );

    let mut text = render_key_management_text(KeyStatus::Missing);
    text.push_str(&render_key_management_text(KeyStatus::Locked {
        public_key_hex: Some("abc".to_owned()),
    }));
    text.push_str(&render_key_management_text(KeyStatus::Present {
        public_key_hex: "abc".to_owned(),
    }));

    assert!(text.contains("Ctrl+g"));
    assert!(text.contains("Ctrl+d"));
    assert!(text.contains("Ctrl+b"));
    assert!(text.contains("Ctrl+r"));
    assert!(!text.contains("press g"));
    assert!(!text.contains("Use d"));
    assert!(!text.contains("press u"));
}

fn dispatch(backend: &Backend, state: &mut AppState, event: &Event) {
    let action = map_event(event, state.screen).expect("event maps to action");
    handle_action(backend, state, action).expect("handle action");
}

fn temp_backend(plaintext: bool) -> (Backend, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|err| panic!("create tempdir: {err}"));
    let backend = Backend::open(Some(temp_dir.path().to_path_buf()), plaintext)
        .unwrap_or_else(|err| panic!("open backend: {err}"));
    (backend, temp_dir)
}

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn ctrl(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL))
}

fn public_key_hex(status: &KeyStatus) -> String {
    match status {
        KeyStatus::Present { public_key_hex }
        | KeyStatus::Locked {
            public_key_hex: Some(public_key_hex),
        } => public_key_hex.clone(),
        KeyStatus::Missing
        | KeyStatus::Locked {
            public_key_hex: None,
        } => {
            panic!("key status has no public key: {status:?}")
        }
    }
}

fn assert_status_contains(state: &AppState, needle: &str) {
    assert!(
        state
            .status_message
            .as_deref()
            .is_some_and(|message| message.contains(needle)),
        "status {:?} did not contain {needle:?}",
        state.status_message
    );
    assert_eq!(state.key_input.status, state.status_message);
}

fn render_key_management_text(key_status: KeyStatus) -> String {
    let mut state = AppState::new();
    state.key_status = key_status;
    let mut buffer = Buffer::empty(Rect::new(0, 0, 96, 24));
    render_key_management(&state, buffer.area, &mut buffer);
    buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>()
}
