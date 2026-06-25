#![allow(missing_docs, reason = "integration tests are executable specs")]
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::unwrap_used,
        reason = "integration tests use convenient panicking APIs"
    )
)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::Duration;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::Value;
use tempfile::TempDir;

const BASE_TIME: &str = "2024-01-01T00:00:00Z";

/// A spawned CLI node that is killed when dropped.
struct RunningNode {
    child: Child,
    addr: String,
}

impl Drop for RunningNode {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn cli() -> Command {
    Command::cargo_bin("agoramesh-cli").expect("find cli binary")
}

fn run(args: &[&str]) -> String {
    let output = cli().args(args).output().expect("run cli");
    assert!(
        output.status.success(),
        "cli failed: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8 stdout")
        .trim()
        .to_owned()
}

fn data_dir(temp: &TempDir) -> PathBuf {
    temp.path().to_path_buf()
}

fn key_generate(data_dir: &Path) {
    run(&[
        "--data-dir",
        data_dir.to_str().expect("utf8 path"),
        "--dev-insecure-plaintext-key",
        "key",
        "generate",
    ]);
}

fn category_create(data_dir: &Path, created_at: &str) -> String {
    let json = run(&[
        "--data-dir",
        data_dir.to_str().expect("utf8 path"),
        "--dev-insecure-plaintext-key",
        "category",
        "create",
        "--display-name",
        "E2E Category",
        "--description",
        "End-to-end test category",
        "--charter",
        "Minimal charter for e2e",
        "--created-at",
        created_at,
        "--json",
    ]);
    let value: Value = serde_json::from_str(&json).expect("category json");
    value["category_id"]
        .as_str()
        .expect("category_id")
        .to_owned()
}

fn post_create(data_dir: &Path, category_id: &str, text: &str, created_at: &str) -> String {
    let json = run(&[
        "--data-dir",
        data_dir.to_str().expect("utf8 path"),
        "--dev-insecure-plaintext-key",
        "post",
        "create",
        "--category-id",
        category_id,
        "--text",
        text,
        "--created-at",
        created_at,
        "--json",
    ]);
    let value: Value = serde_json::from_str(&json).expect("post json");
    value["object_id"].as_str().expect("object_id").to_owned()
}

fn comment_create(
    data_dir: &Path,
    category_id: &str,
    parent_kind: &str,
    parent_id: &str,
    text: &str,
    created_at: &str,
) -> String {
    let json = run(&[
        "--data-dir",
        data_dir.to_str().expect("utf8 path"),
        "--dev-insecure-plaintext-key",
        "comment",
        "create",
        "--category-id",
        category_id,
        "--parent-kind",
        parent_kind,
        "--parent-id",
        parent_id,
        "--text",
        text,
        "--created-at",
        created_at,
        "--json",
    ]);
    let value: Value = serde_json::from_str(&json).expect("comment json");
    value["object_id"].as_str().expect("object_id").to_owned()
}

fn peer_add(data_dir: &Path, address: &str) {
    run(&[
        "--data-dir",
        data_dir.to_str().expect("utf8 path"),
        "peer",
        "add",
        address,
    ]);
}

fn sync(data_dir: &Path, category_id: &str) -> SyncTotals {
    let json = run(&[
        "--data-dir",
        data_dir.to_str().expect("utf8 path"),
        "sync",
        category_id,
        "--json",
    ]);
    let value: Value = serde_json::from_str(&json).expect("sync json");
    SyncTotals {
        pulled: usize::try_from(value["objects_pulled"].as_u64().expect("pulled"))
            .expect("pulled fits usize"),
        rejected: usize::try_from(value["objects_rejected"].as_u64().expect("rejected"))
            .expect("rejected fits usize"),
    }
}

fn feed_json(data_dir: &Path, category_id: &str) -> Vec<Value> {
    let json = run(&[
        "--data-dir",
        data_dir.to_str().expect("utf8 path"),
        "feed",
        category_id,
        "--json",
    ]);
    serde_json::from_str(&json).expect("feed json")
}

fn feed_object_ids(data_dir: &Path, category_id: &str) -> Vec<String> {
    feed_json(data_dir, category_id)
        .into_iter()
        .map(|item| item["object_id"].as_str().expect("object_id").to_owned())
        .collect()
}

fn start_node(data_dir: &Path) -> RunningNode {
    let mut child = std::process::Command::new(cargo_bin("agoramesh-cli"))
        .args([
            "--data-dir",
            data_dir.to_str().expect("utf8 path"),
            "--dev-insecure-plaintext-key",
            "run",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn node");

    let stdout = child.stdout.take().expect("node stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read listening address");
    let addr = line
        .trim()
        .strip_prefix("listening on ")
        .expect("listening line")
        .to_owned();

    // Give the server a moment to finish binding.
    std::thread::sleep(Duration::from_millis(50));

    RunningNode { child, addr }
}

#[derive(Debug, Default)]
struct SyncTotals {
    pulled: usize,
    rejected: usize,
}

#[tokio::test]
async fn two_peer_post_comment_sync() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_b = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_b));

    let server_a = start_node(&data_dir(&node_a));
    let category_id = category_create(&data_dir(&node_a), BASE_TIME);
    let post_id = post_create(
        &data_dir(&node_a),
        &category_id,
        "Hello mesh",
        "2024-01-01T00:01:00Z",
    );
    let comment_id = comment_create(
        &data_dir(&node_a),
        &category_id,
        "post",
        &post_id,
        "First reply",
        "2024-01-01T00:02:00Z",
    );

    peer_add(&data_dir(&node_b), &server_a.addr);
    let totals = sync(&data_dir(&node_b), &category_id);
    assert_eq!(totals.pulled, 3);

    let ids_b = feed_object_ids(&data_dir(&node_b), &category_id);
    assert!(ids_b.contains(&post_id));
    assert!(ids_b.contains(&comment_id));

    // Same object has same hash on both nodes.
    let ids_a = feed_object_ids(&data_dir(&node_a), &category_id);
    assert_eq!(ids_a, ids_b);
}

#[tokio::test]
async fn three_peer_consistency() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_b = tempfile::tempdir().expect("tempdir");
    let node_c = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_b));
    key_generate(&data_dir(&node_c));

    let server_a = start_node(&data_dir(&node_a));
    let server_b = start_node(&data_dir(&node_b));

    let category_id = category_create(&data_dir(&node_a), BASE_TIME);
    let post_id = post_create(
        &data_dir(&node_a),
        &category_id,
        "Post",
        "2024-01-01T00:01:00Z",
    );
    let _comment_id = comment_create(
        &data_dir(&node_a),
        &category_id,
        "post",
        &post_id,
        "Comment",
        "2024-01-01T00:02:00Z",
    );

    peer_add(&data_dir(&node_b), &server_a.addr);
    sync(&data_dir(&node_b), &category_id);

    peer_add(&data_dir(&node_c), &server_b.addr);
    sync(&data_dir(&node_c), &category_id);

    let ids_a = feed_object_ids(&data_dir(&node_a), &category_id);
    let ids_b = feed_object_ids(&data_dir(&node_b), &category_id);
    let ids_c = feed_object_ids(&data_dir(&node_c), &category_id);

    assert_eq!(ids_a, ids_b);
    assert_eq!(ids_b, ids_c);
}

#[tokio::test]
async fn invalid_signature_is_rejected() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_b = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_b));

    let server_a = start_node(&data_dir(&node_a));
    let server_b = start_node(&data_dir(&node_b));
    let category_id = category_create(&data_dir(&node_a), BASE_TIME);
    let post_id = post_create(
        &data_dir(&node_a),
        &category_id,
        "Valid",
        "2024-01-01T00:01:00Z",
    );

    peer_add(&data_dir(&node_b), &server_a.addr);
    sync(&data_dir(&node_b), &category_id);
    assert!(feed_object_ids(&data_dir(&node_b), &category_id).contains(&post_id));

    // Build a tampered message by fetching the valid one and mutating its body.
    let client = reqwest::Client::new();
    let valid: Value = client
        .get(format!("{}/objects/{post_id}", server_a.addr))
        .send()
        .await
        .expect("fetch valid object")
        .json()
        .await
        .expect("decode object");

    let mut tampered = valid.clone();
    tampered["signed_payload"]["body"] = Value::String(base64_encode(b"tampered text"));

    let response = client
        .post(format!("{}/objects", server_b.addr))
        .json(&tampered)
        .send()
        .await
        .expect("post tampered object");
    assert_eq!(response.status(), 422);

    // Node B's store should still contain exactly the original valid object.
    let ids_b = feed_object_ids(&data_dir(&node_b), &category_id);
    assert_eq!(ids_b.len(), 2); // category + post
    assert!(ids_b.contains(&post_id));
}

#[tokio::test]
async fn duplicate_object_is_stored_once() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_b = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_b));

    let server_a = start_node(&data_dir(&node_a));
    let category_id = category_create(&data_dir(&node_a), BASE_TIME);
    let post_id = post_create(
        &data_dir(&node_a),
        &category_id,
        "Once",
        "2024-01-01T00:01:00Z",
    );

    peer_add(&data_dir(&node_b), &server_a.addr);
    sync(&data_dir(&node_b), &category_id);
    let totals = sync(&data_dir(&node_b), &category_id);

    assert_eq!(totals.pulled, 0);
    let ids_b = feed_object_ids(&data_dir(&node_b), &category_id);
    assert_eq!(ids_b.iter().filter(|id| **id == post_id).count(), 1);
}

#[tokio::test]
async fn restart_restores_local_state() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_b = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_b));

    let server_a = start_node(&data_dir(&node_a));
    let category_id = category_create(&data_dir(&node_a), BASE_TIME);
    let post_id = post_create(
        &data_dir(&node_a),
        &category_id,
        "Restore me",
        "2024-01-01T00:01:00Z",
    );

    {
        let _server_b = start_node(&data_dir(&node_b));
        peer_add(&data_dir(&node_b), &server_a.addr);
        sync(&data_dir(&node_b), &category_id);
        assert!(feed_object_ids(&data_dir(&node_b), &category_id).contains(&post_id));
    } // _server_b dropped here.

    let _server_b2 = start_node(&data_dir(&node_b));
    let ids_after_restart = feed_object_ids(&data_dir(&node_b), &category_id);
    assert!(ids_after_restart.contains(&post_id));
}

#[tokio::test]
async fn disconnect_reconnect_catch_up() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_c = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_c));

    let server_a = start_node(&data_dir(&node_a));
    let category_id = category_create(&data_dir(&node_a), BASE_TIME);
    let post1 = post_create(
        &data_dir(&node_a),
        &category_id,
        "Before",
        "2024-01-01T00:01:00Z",
    );

    {
        let _server_c = start_node(&data_dir(&node_c));
        peer_add(&data_dir(&node_c), &server_a.addr);
        sync(&data_dir(&node_c), &category_id);
        assert!(feed_object_ids(&data_dir(&node_c), &category_id).contains(&post1));
    }

    let post2 = post_create(
        &data_dir(&node_a),
        &category_id,
        "After disconnect",
        "2024-01-01T00:02:00Z",
    );

    let _server_c = start_node(&data_dir(&node_c));
    sync(&data_dir(&node_c), &category_id);
    let ids_c = feed_object_ids(&data_dir(&node_c), &category_id);
    assert!(ids_c.contains(&post2));
}

#[tokio::test]
async fn no_default_public_peers() {
    let node = tempfile::tempdir().expect("tempdir");
    let json = run(&[
        "--data-dir",
        data_dir(&node).to_str().expect("utf8 path"),
        "peer",
        "list",
        "--json",
    ]);
    let peers: Vec<Value> = serde_json::from_str(&json).expect("peer list json");
    assert!(peers.is_empty());
}

#[tokio::test]
async fn future_timestamp_policy() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_b = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_b));

    let server_a = start_node(&data_dir(&node_a));
    let category_id = category_create(&data_dir(&node_a), BASE_TIME);

    let now = Utc::now();
    let far_future = format_rfc3339(now + TimeDelta::minutes(10));
    let near_future = format_rfc3339(now + TimeDelta::minutes(2));

    let _far_id = post_create(&data_dir(&node_a), &category_id, "Far future", &far_future);
    let near_id = post_create(
        &data_dir(&node_a),
        &category_id,
        "Near future",
        &near_future,
    );

    peer_add(&data_dir(&node_b), &server_a.addr);
    let totals = sync(&data_dir(&node_b), &category_id);

    // Category + near-future post are pulled; the far-future post is rejected.
    assert_eq!(totals.rejected, 1);
    assert_eq!(totals.pulled, 2);

    let ids_b = feed_object_ids(&data_dir(&node_b), &category_id);
    assert!(ids_b.contains(&near_id));
}

fn format_rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// Ensure each scenario also exercises deterministic object identity.
#[tokio::test]
async fn same_object_same_hash() {
    let node_a = tempfile::tempdir().expect("tempdir");
    let node_b = tempfile::tempdir().expect("tempdir");
    key_generate(&data_dir(&node_a));
    key_generate(&data_dir(&node_b));

    let category_id_a = category_create(&data_dir(&node_a), BASE_TIME);
    let category_id_b = category_create(&data_dir(&node_b), BASE_TIME);

    // Same display_name/charter/created_at with different keys produce different category IDs.
    assert_ne!(category_id_a, category_id_b);

    // Same post inputs on the same node produce identical object IDs.
    let post_id_1 = post_create(
        &data_dir(&node_a),
        &category_id_a,
        "Identical",
        "2024-01-01T00:01:00Z",
    );
    let post_id_2 = post_create(
        &data_dir(&node_a),
        &category_id_a,
        "Identical",
        "2024-01-01T00:01:00Z",
    );
    assert_eq!(post_id_1, post_id_2);
}
