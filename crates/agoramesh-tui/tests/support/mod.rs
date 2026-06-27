#![allow(
    clippy::redundant_pub_crate,
    reason = "integration-test support helpers are pub(crate) to satisfy unreachable_pub"
)]

use agoramesh_core::objects::category;
use agoramesh_store::Store;
use agoramesh_tui::app::AppState;
use agoramesh_tui::backend::Backend;
use agoramesh_tui::controller::handle_action;
use agoramesh_tui::events::map_event;
use agoramesh_tui::models::CategorySummary;
use chrono::{Timelike, Utc};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[allow(dead_code, reason = "only used by controller_flows integration test")]
pub(super) fn dispatch(backend: &Backend, state: &mut AppState, event: &Event) {
    let action = map_event(event, state.screen).expect("event maps to action");
    handle_action(backend, state, action).expect("handle action");
}

pub(super) fn temp_backend(plaintext: bool) -> (Backend, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|err| panic!("create tempdir: {err}"));
    let backend = Backend::open(Some(temp_dir.path().to_path_buf()), plaintext)
        .unwrap_or_else(|err| panic!("open backend: {err}"));
    (backend, temp_dir)
}

#[allow(dead_code, reason = "only used by controller_flows integration test")]
pub(super) fn state_with_category(category: CategorySummary) -> AppState {
    let mut state = AppState::new();
    state.categories = vec![category.clone()];
    state.subscriptions.category_ids = vec![category.category_id];
    state
}

#[allow(dead_code, reason = "only used by controller_flows integration test")]
pub(super) fn stored_category(backend: &Backend, name: &str) -> CategorySummary {
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

pub(super) fn truncate(value: chrono::DateTime<Utc>) -> chrono::DateTime<Utc> {
    value
        .with_nanosecond(0)
        .unwrap_or_else(|| panic!("truncating to seconds is valid"))
}

#[allow(dead_code, reason = "only used by controller_flows integration test")]
pub(super) fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
