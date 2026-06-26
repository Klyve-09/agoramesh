//! Event-to-controller integration tests for Phase 2 TUI flows.
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test fixtures may fail fast on setup errors"
)]

use agoramesh_core::objects::{ParentKind, category, comment, post};
use agoramesh_store::Store;
use agoramesh_tui::app::{Action, AppState};
use agoramesh_tui::backend::Backend;
use agoramesh_tui::controller::handle_action;
use agoramesh_tui::events::map_event;
use agoramesh_tui::key_ux::render_key_management;
use agoramesh_tui::models::{CategorySummary, FeedFocus, FirstSeenWarning, KeyStatus, Screen};
use chrono::{Timelike, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

#[test]
fn feed_compose_unicode_preview_submit_refreshes_feed_persistence() {
    let (backend, _temp_dir) = temp_backend(true);
    backend.generate_dev_key().expect("generate dev key");
    let category = stored_category(&backend, "General");
    let mut state = state_with_category(category.clone());

    dispatch(&backend, &mut state, &press(KeyCode::Char('n')));
    for ch in "안녕 Agora".chars() {
        dispatch(&backend, &mut state, &press(KeyCode::Char(ch)));
    }
    dispatch(&backend, &mut state, &press(KeyCode::Tab));
    dispatch(&backend, &mut state, &press(KeyCode::Enter));

    assert_eq!(state.screen, Screen::Feed);
    assert_eq!(state.posts[&category.category_id][0].text, "안녕 Agora");
    let persisted = backend.load_feed(&category.category_id).expect("load feed");
    assert_eq!(persisted[0].text, "안녕 Agora");
}

#[test]
fn compose_category_selection_posts_to_selected_scope() {
    let (backend, _temp_dir) = temp_backend(true);
    backend.generate_dev_key().expect("generate dev key");
    let general = stored_category(&backend, "General");
    let random = stored_category(&backend, "Random");
    let mut state = AppState::new();
    state.screen = Screen::Compose;
    state.categories = vec![general.clone(), random.clone()];
    state.subscriptions.category_ids =
        vec![general.category_id.clone(), random.category_id.clone()];

    dispatch(&backend, &mut state, &press(KeyCode::Down));
    for ch in "scoped post".chars() {
        dispatch(&backend, &mut state, &press(KeyCode::Char(ch)));
    }
    dispatch(&backend, &mut state, &press(KeyCode::Tab));
    dispatch(&backend, &mut state, &press(KeyCode::Enter));

    assert!(
        backend
            .load_feed(&general.category_id)
            .expect("general")
            .is_empty()
    );
    assert_eq!(
        backend
            .load_feed(&random.category_id)
            .expect("random")
            .len(),
        1
    );
}

#[test]
fn subscription_toggle_loads_existing_feed_without_restart() {
    let (backend, _temp_dir) = temp_backend(true);
    backend.generate_dev_key().expect("generate dev key");
    let category = stored_category(&backend, "Existing");
    backend
        .create_post(&category.category_id, "already here", truncate(Utc::now()))
        .expect("create existing post");
    let mut state = AppState::new();
    state.screen = Screen::Subscriptions;
    state.categories = vec![category.clone()];

    dispatch(&backend, &mut state, &press(KeyCode::Char(' ')));

    assert_eq!(
        state.subscriptions.category_ids,
        vec![category.category_id.clone()]
    );
    assert_eq!(state.posts[&category.category_id][0].text, "already here");
}

#[test]
fn compose_submit_selects_submitted_category_and_new_post() {
    let (backend, _temp_dir) = temp_backend(true);
    backend.generate_dev_key().expect("generate dev key");
    let general = stored_category(&backend, "General");
    let random = stored_category(&backend, "Random");
    let existing = backend
        .create_post(&random.category_id, "older random", truncate(Utc::now()))
        .expect("create existing random post");
    let mut state = AppState::new();
    state.screen = Screen::Compose;
    state.categories = vec![general.clone(), random.clone()];
    state.subscriptions.category_ids = vec![general.category_id, random.category_id.clone()];
    state
        .posts
        .insert(random.category_id.clone(), vec![existing]);

    dispatch(&backend, &mut state, &press(KeyCode::Down));
    for ch in "new random".chars() {
        dispatch(&backend, &mut state, &press(KeyCode::Char(ch)));
    }
    dispatch(&backend, &mut state, &press(KeyCode::Tab));
    dispatch(&backend, &mut state, &press(KeyCode::Enter));

    assert_eq!(state.screen, Screen::Feed);
    assert_eq!(state.feed_focus, FeedFocus::Posts);
    assert_eq!(state.selected_category_index, 1);
    assert_eq!(state.selected_post_index, 1);
    assert_eq!(state.posts[&random.category_id][1].text, "new random");
}

#[test]
fn selected_post_enter_loads_thread() {
    let (backend, _temp_dir) = temp_backend(true);
    backend.generate_dev_key().expect("generate dev key");
    let category = stored_category(&backend, "Threaded");
    let first = backend
        .create_post(&category.category_id, "first", truncate(Utc::now()))
        .expect("create first");
    let second = backend
        .create_post(&category.category_id, "second", truncate(Utc::now()))
        .expect("create second");
    let mut state = state_with_category(category.clone());
    state
        .posts
        .insert(category.category_id, vec![first, second]);
    state.feed_focus = FeedFocus::Posts;
    state.selected_post_index = 1;

    dispatch(&backend, &mut state, &press(KeyCode::Enter));

    assert_eq!(state.screen, Screen::Thread);
    assert_eq!(
        state
            .thread
            .as_ref()
            .map(|thread| thread.post.text.as_str()),
        Some("second")
    );
}

#[test]
fn nested_comments_collapse_excludes_descendants_from_selection() {
    let (backend, _temp_dir) = temp_backend(true);
    let keypair = agoramesh_core::Keypair::generate();
    let created_at = truncate(Utc::now());
    let category = category::create(&keypair, created_at, "Nested", "Nested", "Charter")
        .expect("create category");
    let category_id = category.signed_payload().scope().to_owned();
    let root = post::create(&keypair, &category_id, "root", created_at).expect("post");
    let root_id = root.id();
    let top = comment::create(
        &keypair,
        &category_id,
        ParentKind::Post,
        root_id,
        "top",
        created_at,
    )
    .expect("top");
    let mid = comment::create(
        &keypair,
        &category_id,
        ParentKind::Comment,
        top.id(),
        "mid",
        created_at,
    )
    .expect("mid");
    let leaf = comment::create(
        &keypair,
        &category_id,
        ParentKind::Comment,
        mid.id(),
        "leaf",
        created_at,
    )
    .expect("leaf");
    let post_id = root.id().to_hex();
    let mut store = backend.store().expect("store");
    for message in [category, root, top, mid, leaf] {
        store
            .insert(message, &agoramesh_core::SystemClock)
            .expect("insert");
    }
    let mut state = AppState::new();
    state.screen = Screen::Thread;
    state.thread = Some(backend.load_thread(&post_id).expect("thread"));

    handle_action(&backend, &mut state, agoramesh_tui::app::Action::Select).expect("collapse");
    dispatch(&backend, &mut state, &press(KeyCode::Down));

    assert_eq!(state.selected_index, 0);
    assert!(state.thread.as_ref().expect("thread").comments[0].collapsed);
}

#[test]
fn subscriptions_and_warning_acknowledgement_persist_after_reopen() {
    let (backend, temp_dir) = temp_backend(true);
    let category = stored_category(&backend, "Persisted");
    let mut state = AppState::new();
    state.categories = vec![category.clone()];
    state.screen = Screen::Subscriptions;
    dispatch(&backend, &mut state, &press(KeyCode::Char(' ')));
    state.warnings = vec![FirstSeenWarning::Category {
        category_id: category.category_id.clone(),
        display_name: Some(category.display_name.clone()),
    }];
    dispatch(&backend, &mut state, &press(KeyCode::Char('a')));

    let reopened = Backend::open(Some(temp_dir.path().to_path_buf()), true).expect("reopen");
    assert_eq!(
        reopened.load_subscriptions().expect("subs").category_ids,
        vec![category.category_id.clone()]
    );
    assert_eq!(
        reopened.load_acknowledged().expect("ack").categories,
        vec![category.category_id]
    );
}

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

#[test]
fn unknown_key_does_not_change_screen_to_feed() {
    let (backend, _temp_dir) = temp_backend(true);
    let mut state = AppState::new();
    state.screen = Screen::Subscriptions;

    let event = press(KeyCode::Char('x'));
    let action = map_event(&event, state.screen);
    if let Some(action) = action {
        handle_action(&backend, &mut state, action).expect("handle action");
    }

    assert_eq!(state.screen, Screen::Subscriptions);
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

fn state_with_category(category: CategorySummary) -> AppState {
    let mut state = AppState::new();
    state.categories = vec![category.clone()];
    state.subscriptions.category_ids = vec![category.category_id];
    state
}

fn stored_category(backend: &Backend, name: &str) -> CategorySummary {
    let keypair = agoramesh_core::Keypair::generate();
    let created_at = truncate(Utc::now());
    let message = category::create(&keypair, created_at, name, name, "Charter")
        .unwrap_or_else(|err| panic!("create category: {err}"));
    let summary = CategorySummary {
        object_id: message.id().to_hex(),
        display_name: name.to_owned(),
        description: name.to_owned(),
        category_id: message.signed_payload().scope().to_owned(),
        created_at,
    };
    backend
        .store()
        .expect("store")
        .insert(message, &agoramesh_core::SystemClock)
        .expect("insert category");
    summary
}

fn truncate(value: chrono::DateTime<Utc>) -> chrono::DateTime<Utc> {
    value
        .date_naive()
        .and_hms_micro_opt(value.hour(), value.minute(), value.second(), 0)
        .unwrap_or_else(|| panic!("truncating to seconds is valid"))
        .and_local_timezone(Utc)
        .single()
        .unwrap_or_else(|| panic!("UTC timezone is valid"))
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
