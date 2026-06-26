use chrono::Utc;

use super::*;
use crate::models::{CategorySummary, FirstSeenWarning, Screen, Subscriptions};

#[test]
fn first_seen_compute_warnings_lists_unacknowledged_categories_and_peers() {
    let category = crate::models::CategorySummary {
        object_id: "oid".to_owned(),
        display_name: "General".to_owned(),
        description: "General chat".to_owned(),
        category_id: "cat-general".to_owned(),
        created_at: chrono::Utc::now(),
    };
    let peer = crate::models::PeerStatus {
        name: None,
        address: "http://127.0.0.1:8080".to_owned(),
        last_sync_ok: None,
    };
    let acknowledged = crate::models::AcknowledgedFirstSeen::default();
    let warnings = crate::first_seen::compute_warnings(&[category], &[peer], &acknowledged);
    assert_eq!(warnings.len(), 2);
}

#[test]
fn handle_action_acknowledges_first_seen_warning_and_saves_it() {
    let (backend, _temp_dir) = backend_fixture(true);
    let warning = FirstSeenWarning::Category {
        category_id: "cat-1".to_owned(),
        display_name: None,
    };
    let mut state = AppState::new().apply(Action::SetWarnings(vec![warning]));

    let result = handle_action(&backend, &mut state, Action::AcknowledgeCurrentWarning);

    assert!(result.expect("select action succeeds").is_none());
    assert!(state.warnings.is_empty());
    let saved = backend.load_acknowledged().expect("load saved ack");
    assert_eq!(saved.categories, vec!["cat-1".to_owned()]);
}

#[test]
fn handle_action_submits_compose_and_returns_to_feed() {
    let (backend, _temp_dir) = backend_fixture(true);
    backend.generate_dev_key().expect("generate dev key");
    let category = sample_category("cat-1");
    let mut state = AppState::new();
    state.categories = vec![category.clone()];
    state.subscriptions = Subscriptions {
        category_ids: vec![category.category_id.clone()],
    };
    state.screen = Screen::Compose;
    state.compose.category_index = 0;
    state.compose.text = "Hello compose".to_owned();
    state.compose.preview = true;

    let result = handle_action(&backend, &mut state, Action::ComposeSubmit);

    assert!(result.expect("compose submit succeeds").is_none());
    assert_eq!(state.screen, Screen::Feed);
    assert_eq!(state.compose.text, "");
    assert_eq!(state.compose.status.as_deref(), Some("Post submitted"));
    assert_eq!(state.status_message.as_deref(), Some("Post submitted"));
    assert_eq!(
        state.posts.get(&category.category_id).map(Vec::len),
        Some(1)
    );
}

#[test]
fn handle_action_generates_dev_key_and_updates_status() {
    let (backend, _temp_dir) = backend_fixture(true);
    let mut state = AppState::new();

    let result = handle_action(&backend, &mut state, Action::GenerateDevKey);

    assert!(result.expect("generate dev key succeeds").is_none());
    assert!(matches!(state.key_status, KeyStatus::Present { .. }));
    assert_eq!(
        state.status_message.as_deref(),
        Some("Development key generated")
    );
}

#[test]
fn handle_action_toggles_selected_subscription_and_saves_it() {
    let (backend, _temp_dir) = backend_fixture(true);
    let category = sample_category("cat-1");
    let category_id = category.category_id.clone();
    let mut state = AppState::new();
    state.screen = Screen::Subscriptions;
    state.categories = vec![category];

    let result = handle_action(&backend, &mut state, Action::ToggleSelectedSubscription);

    assert!(result.expect("toggle subscription succeeds").is_none());
    assert_eq!(state.subscriptions.category_ids, vec![category_id.clone()]);
    let saved = backend
        .load_subscriptions()
        .expect("load saved subscriptions");
    assert_eq!(saved.category_ids, vec![category_id]);
}

#[test]
fn handle_action_selects_the_newest_post_thread_from_feed() {
    let (backend, _temp_dir) = backend_fixture(true);
    backend.generate_dev_key().expect("generate dev key");
    let category = sample_category("cat-1");
    let mut state = AppState::new();
    state.categories = vec![category.clone()];
    state.subscriptions = Subscriptions {
        category_ids: vec![category.category_id.clone()],
    };
    state.posts.insert(
        category.category_id.clone(),
        vec![
            backend
                .create_post(&category.category_id, "Root post", Utc::now())
                .expect("create post"),
        ],
    );

    let result = handle_action(&backend, &mut state, Action::Select);

    assert!(result.expect("select succeeds").is_none());
    assert_eq!(state.screen, Screen::Thread);
    assert_eq!(
        state
            .thread
            .as_ref()
            .map(|thread| thread.post.text.as_str()),
        Some("Root post")
    );
}

#[test]
fn handle_action_select_on_feed_without_posts_sets_status_message() {
    let (backend, _temp_dir) = backend_fixture(true);
    let category = sample_category("cat-1");
    let mut state = AppState::new();
    state.categories = vec![category];

    let result = handle_action(&backend, &mut state, Action::Select);

    assert!(result.expect("select succeeds").is_none());
    assert_eq!(state.screen, Screen::Feed);
    assert_eq!(
        state.status_message.as_deref(),
        Some("No post selected in the current feed category")
    );
}

#[test]
fn terminal_setup_cleanup_attempts_all_completed_steps_after_failure() {
    let mut cleanup = RecordingSetupCleanup::default();
    let progress = TerminalSetupProgress {
        raw_mode: true,
        alternate_screen: true,
        mouse_capture: true,
    };

    cleanup_terminal_setup(&progress, &mut cleanup).expect("cleanup succeeds");

    assert_eq!(
        cleanup.calls,
        vec![
            CleanupCall::DisableMouseCapture,
            CleanupCall::LeaveAlternateScreen,
            CleanupCall::DisableRawMode,
        ]
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CleanupCall {
    DisableMouseCapture,
    LeaveAlternateScreen,
    DisableRawMode,
}

#[derive(Default)]
struct RecordingSetupCleanup {
    calls: Vec<CleanupCall>,
}

impl TerminalSetupCleanup for RecordingSetupCleanup {
    fn disable_mouse_capture(&mut self) -> std::io::Result<()> {
        self.calls.push(CleanupCall::DisableMouseCapture);
        Ok(())
    }

    fn leave_alternate_screen(&mut self) -> std::io::Result<()> {
        self.calls.push(CleanupCall::LeaveAlternateScreen);
        Ok(())
    }

    fn disable_raw_mode(&mut self) -> std::io::Result<()> {
        self.calls.push(CleanupCall::DisableRawMode);
        Ok(())
    }
}

fn backend_fixture(plaintext: bool) -> (Backend, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("create tempdir");
    let backend =
        Backend::open(Some(temp_dir.path().to_path_buf()), plaintext).expect("open backend");
    (backend, temp_dir)
}

fn sample_category(category_id: &str) -> CategorySummary {
    CategorySummary {
        object_id: format!("{category_id}-object"),
        display_name: "General".to_owned(),
        description: String::new(),
        category_id: category_id.to_owned(),
        created_at: Utc::now(),
    }
}
