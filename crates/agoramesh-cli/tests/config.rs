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

#[test]
fn config_peer_list_json_is_empty_when_data_dir_is_fresh() {
    let data_dir = tempfile::tempdir().expect("create tempdir");

    Command::cargo_bin("agoramesh-cli")
        .expect("binary exists")
        .args([
            "--data-dir",
            data_dir.path().to_str().expect("utf-8 temp path"),
            "peer",
            "list",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::eq("[]\n"));

    assert!(data_dir.path().join("store.db").is_file());
    assert!(data_dir.path().join("peers.json").is_file());
}

#[test]
fn config_peer_add_persists_address_for_json_list() {
    let data_dir = tempfile::tempdir().expect("create tempdir");
    let data_dir_arg = data_dir.path().to_str().expect("utf-8 temp path");

    Command::cargo_bin("agoramesh-cli")
        .expect("binary exists")
        .args([
            "--data-dir",
            data_dir_arg,
            "peer",
            "add",
            "http://127.0.0.1:8080",
        ])
        .assert()
        .success();

    let output = Command::cargo_bin("agoramesh-cli")
        .expect("binary exists")
        .args(["--data-dir", data_dir_arg, "peer", "list", "--json"])
        .output()
        .expect("run peer list");

    assert!(output.status.success());
    let peers: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(peers[0]["name"], Value::Null);
    assert_eq!(peers[0]["address"], "http://127.0.0.1:8080");
    assert!(peers[0]["added_at"].as_str().is_some());
}

#[test]
fn config_peer_add_rejects_non_http_addresses() {
    let data_dir = tempfile::tempdir().expect("create tempdir");

    Command::cargo_bin("agoramesh-cli")
        .expect("binary exists")
        .args([
            "--data-dir",
            data_dir.path().to_str().expect("utf-8 temp path"),
            "peer",
            "add",
            "udp://127.0.0.1:8080",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "peer address must start with http://",
        ));
}
