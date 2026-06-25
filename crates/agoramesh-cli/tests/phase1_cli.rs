#![allow(missing_docs, reason = "integration tests are executable specs")]
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "integration tests use convenient panicking APIs"
    )
)]

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

const CATEGORY_CREATED_AT: &str = "2024-01-02T03:04:05Z";
const POST_CREATED_AT: &str = "2024-01-02T03:04:06Z";
const COMMENT_CREATED_AT: &str = "2024-01-02T03:04:07Z";

#[test]
fn category_post_comment_create_and_feed_json_roundtrip() {
    let data_dir = tempfile::tempdir().expect("create tempdir");
    let key_path = data_dir.path().join("identity.key");
    let data_dir_arg = path_text(data_dir.path());
    let key_path_arg = path_text(&key_path);

    generate_plaintext_key(data_dir_arg, key_path_arg);

    let category = run_json(
        Command::cargo_bin("agoramesh-cli")
            .expect("binary exists")
            .args([
                "--data-dir",
                data_dir_arg,
                "--key-path",
                key_path_arg,
                "--dev-insecure-plaintext-key",
                "category",
                "create",
                "--display-name",
                "Local Mesh",
                "--description",
                "A local test category",
                "--charter",
                "Keep tests deterministic",
                "--created-at",
                CATEGORY_CREATED_AT,
                "--json",
            ]),
    );
    let category_id = category["category_id"].as_str().expect("category id");
    assert_eq!(category["kind"], "category");
    assert_hex_id(category_id);
    assert_hex_id(category["object_id"].as_str().expect("object id"));

    let post = run_json(
        Command::cargo_bin("agoramesh-cli")
            .expect("binary exists")
            .args([
                "--data-dir",
                data_dir_arg,
                "--key-path",
                key_path_arg,
                "--dev-insecure-plaintext-key",
                "post",
                "create",
                "--category-id",
                category_id,
                "--text",
                "Hello mesh",
                "--created-at",
                POST_CREATED_AT,
                "--json",
            ]),
    );
    let post_id = post["object_id"].as_str().expect("post id");
    assert_eq!(post["kind"], "post");
    assert_hex_id(post_id);

    let comment = run_json(
        Command::cargo_bin("agoramesh-cli")
            .expect("binary exists")
            .args([
                "--data-dir",
                data_dir_arg,
                "--key-path",
                key_path_arg,
                "--dev-insecure-plaintext-key",
                "comment",
                "create",
                "--category-id",
                category_id,
                "--parent-kind",
                "post",
                "--parent-id",
                post_id,
                "--text",
                "First reply",
                "--created-at",
                COMMENT_CREATED_AT,
                "--json",
            ]),
    );
    assert_eq!(comment["kind"], "comment");
    assert_hex_id(comment["object_id"].as_str().expect("comment id"));

    let feed = run_json(
        Command::cargo_bin("agoramesh-cli")
            .expect("binary exists")
            .args(["--data-dir", data_dir_arg, "feed", category_id, "--json"]),
    );
    let feed_items = feed.as_array().expect("feed array");
    assert_eq!(feed_items.len(), 3);
    assert_eq!(feed_items[0]["kind"], "category");
    assert_eq!(feed_items[1]["kind"], "post");
    assert_eq!(feed_items[2]["kind"], "comment");
    assert_eq!(feed_items[0]["created_at"], CATEGORY_CREATED_AT);
    assert_eq!(feed_items[1]["body_json"]["text"], "Hello mesh");
    assert_eq!(feed_items[2]["body_json"]["parent_id"], post_id);
}

#[test]
fn feed_human_output_prints_created_at_kind_and_object_id() {
    let data_dir = tempfile::tempdir().expect("create tempdir");
    let key_path = data_dir.path().join("identity.key");
    let data_dir_arg = path_text(data_dir.path());
    let key_path_arg = path_text(&key_path);

    generate_plaintext_key(data_dir_arg, key_path_arg);
    let category = run_json(
        Command::cargo_bin("agoramesh-cli")
            .expect("binary exists")
            .args([
                "--data-dir",
                data_dir_arg,
                "--key-path",
                key_path_arg,
                "--dev-insecure-plaintext-key",
                "category",
                "create",
                "--display-name",
                "Human Feed",
                "--description",
                "Readable feed",
                "--charter",
                "One line per object",
                "--created-at",
                CATEGORY_CREATED_AT,
                "--json",
            ]),
    );
    let category_id = category["category_id"].as_str().expect("category id");

    Command::cargo_bin("agoramesh-cli")
        .expect("binary exists")
        .args(["--data-dir", data_dir_arg, "feed", category_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("2024-01-02T03:04:05Z [category]"));
}

#[test]
fn sync_without_peers_prints_zero_json_totals() {
    let data_dir = tempfile::tempdir().expect("create tempdir");

    Command::cargo_bin("agoramesh-cli")
        .expect("binary exists")
        .args([
            "--data-dir",
            path_text(data_dir.path()),
            "sync",
            "0".repeat(64).as_str(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::eq(
            "{\"objects_pulled\":0,\"objects_pushed\":0,\"objects_rejected\":0}\n",
        ));
}

fn generate_plaintext_key(data_dir: &str, key_path: &str) {
    Command::cargo_bin("agoramesh-cli")
        .expect("binary exists")
        .args([
            "--data-dir",
            data_dir,
            "--key-path",
            key_path,
            "--dev-insecure-plaintext-key",
            "key",
            "generate",
        ])
        .assert()
        .success();
}

fn run_json(command: &mut Command) -> Value {
    let output = command.output().expect("run command");
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid json")
}

fn assert_hex_id(value: &str) {
    assert_eq!(value.len(), 64);
    assert!(value.chars().all(|character| character.is_ascii_hexdigit()));
}

fn path_text(path: &std::path::Path) -> &str {
    path.to_str().expect("utf-8 temp path")
}
