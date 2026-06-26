//! Integration tests for the TUI feed, compose, and thread flows.
#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "test fixtures may fail fast on setup errors"
)]

mod support;

use agoramesh_core::objects::{category, post};
use agoramesh_store::Store;
use agoramesh_tui::app::AppState;
use agoramesh_tui::compose::{ComposeState, render_compose, submit_compose};
use agoramesh_tui::feed::render_feed;
use agoramesh_tui::models::{CategorySummary, Subscriptions};
use agoramesh_tui::thread::render_thread;
use chrono::Utc;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use support::{temp_backend, truncate};

const PLAINTEXT: bool = true;

#[test]
fn feed_and_thread_integration() {
    let (backend, _temp_dir) = temp_backend(PLAINTEXT);
    backend.generate_dev_key().expect("generate dev key");

    let keypair = agoramesh_cli::keyring::Keyring::new(&backend.data_dir().join("identity.key"))
        .dev_plaintext_load()
        .expect("load key");
    let created_at = truncate(Utc::now());
    let category_message =
        category::create(&keypair, created_at, "General", "General chat", "Charter")
            .expect("create category");
    let category_id = category_message.signed_payload().scope().to_owned();
    let post_message =
        post::create(&keypair, &category_id, "Integration post", created_at).expect("create post");
    let post_id = post_message.id().to_hex();
    let mut store = backend.store().expect("open store");
    store
        .insert(category_message, &agoramesh_core::SystemClock)
        .expect("insert category");
    store
        .insert(post_message, &agoramesh_core::SystemClock)
        .expect("insert post");

    let posts = backend.load_feed(&category_id).expect("load feed");
    assert_eq!(posts.len(), 1);
    assert_eq!(
        posts.first().map_or("", |post| post.text.as_str()),
        "Integration post"
    );

    let thread = backend.load_thread(&post_id).expect("load thread");
    assert_eq!(thread.post.text, "Integration post");

    let mut state = AppState::new();
    state.categories = vec![CategorySummary {
        object_id: "cat-oid".to_owned(),
        display_name: "General".to_owned(),
        description: String::new(),
        category_id,
        created_at,
    }];
    state.subscriptions = Subscriptions {
        category_ids: vec![
            state
                .categories
                .first()
                .map_or(String::new(), |category| category.category_id.clone()),
        ],
    };
    state.thread = Some(thread);

    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
    render_feed(&state, buffer.area, &mut buffer);
    let text = buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(text.contains("General"));

    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
    render_thread(&state, buffer.area, &mut buffer);
    let text = buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(text.contains("Integration post"));
}

#[test]
fn compose_preview_and_submit_integration() {
    let (backend, _temp_dir) = temp_backend(PLAINTEXT);
    backend.generate_dev_key().expect("generate dev key");
    let keypair = agoramesh_cli::keyring::Keyring::new(&backend.data_dir().join("identity.key"))
        .dev_plaintext_load()
        .expect("load key");
    let created_at = truncate(Utc::now());
    let category_message = category::create(
        &keypair,
        created_at,
        "Compose Test",
        "Testing compose",
        "Charter",
    )
    .expect("create category");
    let category_id = category_message.signed_payload().scope().to_owned();
    let mut store = backend.store().expect("open store");
    store
        .insert(category_message, &agoramesh_core::SystemClock)
        .expect("insert category");

    let mut state = AppState::new();
    state.categories = vec![CategorySummary {
        object_id: "cat-oid".to_owned(),
        display_name: "Compose Test".to_owned(),
        description: String::new(),
        category_id: category_id.clone(),
        created_at,
    }];

    let compose = ComposeState {
        category_index: 0,
        text: "Submitted via integration".to_owned(),
        preview: true,
        status: None,
    };

    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 24));
    render_compose(&state, &compose, buffer.area, &mut buffer);
    let text = buffer
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(text.contains("Preview"));

    let post = submit_compose(&backend, &state, &compose).expect("submit");
    assert_eq!(post.text, "Submitted via integration");

    let posts = backend.load_feed(&category_id).expect("load feed");
    assert_eq!(posts.len(), 1);
    assert_eq!(
        posts.first().map_or("", |post| post.text.as_str()),
        "Submitted via integration"
    );
}
