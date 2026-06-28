use super::*;
use agoramesh_core::Keypair;
use agoramesh_core::objects::{ParentKind, category, comment as comment_obj};
use agoramesh_store::Store;
use chrono::Timelike;

fn backend_fixture(plaintext: bool) -> (Backend, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("create tempdir");
    let backend =
        Backend::open(Some(temp_dir.path().to_path_buf()), plaintext).expect("open backend");
    (backend, temp_dir)
}

#[test]
fn backend_loads_feed_from_sqlite_scope() {
    let (backend, _temp_dir) = backend_fixture(true);
    let keypair = Keypair::generate();
    let now = Utc::now();
    let created_at = now.with_nanosecond(0).expect("truncate to seconds");
    let category = category::create(
        &keypair,
        created_at,
        "Test Category",
        "A test category",
        "Initial charter text",
    )
    .expect("create category");
    let category_id = category.signed_payload().scope().to_owned();
    let mut store = backend.store().expect("open store");
    let clock = SystemClock;
    store.insert(category, &clock).expect("insert category");

    let post = post_obj::create(
        &keypair,
        &category_id,
        "Hello from the TUI feed",
        created_at,
    )
    .expect("create post");
    store.insert(post, &clock).expect("insert post");

    let posts = backend.load_feed(&category_id).expect("load feed");
    assert_eq!(posts.len(), 1);
    assert_eq!(
        posts.first().map_or("", |post| post.text.as_str()),
        "Hello from the TUI feed"
    );
}

#[test]
fn backend_loads_nested_thread_replies() {
    let (backend, _temp_dir) = backend_fixture(true);
    let keypair = Keypair::generate();
    let created_at = Utc::now().with_nanosecond(0).expect("truncate to seconds");
    let category = category::create(
        &keypair,
        created_at,
        "Thread Category",
        "A test category",
        "Initial charter text",
    )
    .expect("create category");
    let category_id = category.signed_payload().scope().to_owned();
    let mut store = backend.store().expect("open store");
    let clock = SystemClock;
    store.insert(category, &clock).expect("insert category");

    let post = post_obj::create(
        &keypair,
        &category_id,
        "Hello from the thread view",
        created_at,
    )
    .expect("create post");
    let post_id = post.id().to_hex();
    store.insert(post, &clock).expect("insert post");

    let top_comment = comment_obj::create(
        &keypair,
        &category_id,
        ParentKind::Post,
        agoramesh_core::MessageId::from_hex(&post_id).expect("parse post id"),
        "Top-level comment",
        created_at,
    )
    .expect("create top comment");
    let top_comment_id = top_comment.id().to_hex();
    store
        .insert(top_comment, &clock)
        .expect("insert top comment");

    let first_reply = comment_obj::create(
        &keypair,
        &category_id,
        ParentKind::Comment,
        agoramesh_core::MessageId::from_hex(&top_comment_id).expect("parse top comment id"),
        "First reply",
        created_at,
    )
    .expect("create first reply");
    let first_reply_id = first_reply.id().to_hex();
    store
        .insert(first_reply, &clock)
        .expect("insert first reply");

    let second_reply = comment_obj::create(
        &keypair,
        &category_id,
        ParentKind::Comment,
        agoramesh_core::MessageId::from_hex(&top_comment_id).expect("parse top comment id"),
        "Second reply",
        created_at,
    )
    .expect("create second reply");
    let second_reply_id = second_reply.id().to_hex();
    store
        .insert(second_reply, &clock)
        .expect("insert second reply");

    let thread = backend.load_thread(&post_id).expect("load thread");
    assert_eq!(thread.post.text, "Hello from the thread view");
    assert_eq!(thread.comments.len(), 1);

    let comment = thread.comments.first().expect("top-level comment");
    assert_eq!(comment.text, "Top-level comment");
    assert_eq!(comment.replies.len(), 2);

    let expected_reply_ids = {
        let mut ids = vec![first_reply_id, second_reply_id];
        ids.sort();
        ids
    };
    let loaded_reply_ids: Vec<_> = comment
        .replies
        .iter()
        .map(|reply| reply.object_id.as_str())
        .collect();
    assert_eq!(loaded_reply_ids, expected_reply_ids);
}

#[test]
fn thread_ignores_comment_with_post_id_but_comment_parent_kind() {
    let (backend, _temp_dir) = backend_fixture(true);
    let keypair = Keypair::generate();
    let created_at = Utc::now().with_nanosecond(0).expect("truncate to seconds");
    let category = category::create(
        &keypair,
        created_at,
        "Thread Category",
        "A test category",
        "Initial charter text",
    )
    .expect("create category");
    let category_id = category.signed_payload().scope().to_owned();
    let mut store = backend.store().expect("open store");
    let clock = SystemClock;
    store.insert(category, &clock).expect("insert category");

    let post = post_obj::create(
        &keypair,
        &category_id,
        "Hello from the thread view",
        created_at,
    )
    .expect("create post");
    let post_id = post.id().to_hex();
    store.insert(post, &clock).expect("insert post");

    let mismatched = comment_obj::create(
        &keypair,
        &category_id,
        ParentKind::Comment,
        agoramesh_core::MessageId::from_hex(&post_id).expect("parse post id"),
        "Mismatched parent kind",
        created_at,
    )
    .expect("create mismatched comment");
    store.insert(mismatched, &clock).expect("insert mismatched");

    let thread = backend.load_thread(&post_id).expect("load thread");
    assert!(
        thread.comments.is_empty(),
        "a comment with parent_kind=Comment and parent_id=post_id should not appear as a top-level post comment"
    );
}

#[test]
fn load_thread_rejects_non_post_root() {
    let (backend, _temp_dir) = backend_fixture(true);
    let keypair = Keypair::generate();
    let created_at = Utc::now().with_nanosecond(0).expect("truncate to seconds");
    let category = category::create(
        &keypair,
        created_at,
        "Thread Category",
        "A test category",
        "Initial charter text",
    )
    .expect("create category");
    let category_id = category.signed_payload().scope().to_owned();
    let mut store = backend.store().expect("open store");
    let clock = SystemClock;
    store.insert(category, &clock).expect("insert category");

    let post = post_obj::create(
        &keypair,
        &category_id,
        "Hello from the thread view",
        created_at,
    )
    .expect("create post");
    let post_id = post.id();
    store.insert(post, &clock).expect("insert post");

    let comment = comment_obj::create(
        &keypair,
        &category_id,
        ParentKind::Post,
        post_id,
        "A comment body is not a thread root post",
        created_at,
    )
    .expect("create comment");
    let comment_id = comment.id().to_hex();
    store.insert(comment, &clock).expect("insert comment");

    let error = backend
        .load_thread(&comment_id)
        .expect_err("load_thread should reject a comment object as the thread root");
    assert!(
        error
            .to_string()
            .contains("스레드 루트가 게시글이 아닙니다"),
        "expected non-post thread root error, got {error}"
    );
}

#[test]
fn thread_rejects_malformed_comment_parent_id() {
    let (backend, _temp_dir) = backend_fixture(true);
    let keypair = Keypair::generate();
    let created_at = Utc::now().with_nanosecond(0).expect("truncate to seconds");
    let category = category::create(
        &keypair,
        created_at,
        "Thread Category",
        "A test category",
        "Initial charter text",
    )
    .expect("create category");
    let category_id = category.signed_payload().scope().to_owned();
    let mut store = backend.store().expect("open store");
    let clock = SystemClock;
    store.insert(category, &clock).expect("insert category");

    let post = post_obj::create(
        &keypair,
        &category_id,
        "Hello from the thread view",
        created_at,
    )
    .expect("create post");
    let post_id = post.id().to_hex();
    store.insert(post, &clock).expect("insert post");

    let message = {
        let body = comment_obj::Body {
            category_id: category_id.clone(),
            parent_kind: ParentKind::Post,
            parent_id: "not-a-valid-id".to_owned(),
            text: "bad parent id".to_owned(),
            created_at,
        };
        let canonical = agoramesh_core::canonical::to_vec(&body).expect("canonicalize");
        agoramesh_core::message::Message::create(
            &keypair,
            "comment",
            created_at,
            category_id,
            canonical,
        )
        .expect("create bad comment")
    };
    store.insert(message, &clock).expect("insert bad comment");

    let result = backend.load_thread(&post_id);
    let error =
        result.expect_err("load_thread should fail when a comment has an invalid parent_id");
    assert!(
        error
            .to_string()
            .contains("댓글 parent_id가 올바르지 않습니다"),
        "expected malformed parent_id error, got {error}"
    );
}
