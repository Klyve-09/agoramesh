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
use agoramesh_tui::app::AppState;
use agoramesh_tui::backend::Backend;
use agoramesh_tui::controller::handle_action;
use agoramesh_tui::events::map_event;
use agoramesh_tui::models::{CategorySummary, FeedFocus, FirstSeenWarning, Screen};
use chrono::{Timelike, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

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
        .with_nanosecond(0)
        .unwrap_or_else(|| panic!("truncating to seconds is valid"))
}

fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
