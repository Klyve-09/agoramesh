//! Key command integration tests.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        reason = "integration tests use convenient panicking APIs"
    )
)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn key_generate_writes_encrypted_key_file_by_default() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let key_path = tempdir.path().join("identity.key");

    Command::cargo_bin("agoramesh-cli")
        .expect("find cli binary")
        .args(["--key-path", path_text(&key_path), "key", "generate"])
        .write_stdin("correct horse battery staple\ncorrect horse battery staple\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("encrypted key"));

    let key_file = std::fs::read_to_string(&key_path).expect("read generated key file");
    assert!(key_file.contains("argon2id"));
    assert!(key_file.contains("ciphertext"));
    assert!(!key_file.contains("secret_seed"));
}

#[test]
fn key_show_prints_public_identity_without_secret_by_default() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let key_path = tempdir.path().join("identity.key");

    generate_encrypted_key(&key_path);

    Command::cargo_bin("agoramesh-cli")
        .expect("find cli binary")
        .args(["--key-path", path_text(&key_path), "key", "show"])
        .write_stdin("correct horse battery staple\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("identity: "))
        .stdout(predicate::str::contains("secret_seed:").not());
}

#[test]
fn key_show_secret_requires_explicit_flag() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let key_path = tempdir.path().join("identity.key");

    generate_encrypted_key(&key_path);

    Command::cargo_bin("agoramesh-cli")
        .expect("find cli binary")
        .args([
            "--key-path",
            path_text(&key_path),
            "key",
            "show",
            "--show-secret",
        ])
        .write_stdin("correct horse battery staple\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("identity: "))
        .stdout(predicate::str::contains("secret_seed: "))
        .stderr(predicate::str::contains("WARNING"));
}

#[test]
fn key_show_with_wrong_passphrase_fails_closed() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let key_path = tempdir.path().join("identity.key");

    generate_encrypted_key(&key_path);

    Command::cargo_bin("agoramesh-cli")
        .expect("find cli binary")
        .args([
            "--key-path",
            path_text(&key_path),
            "key",
            "show",
            "--show-secret",
        ])
        .write_stdin("wrong passphrase\n")
        .assert()
        .failure()
        .stdout(predicate::str::contains("secret_seed:").not())
        .stdout(predicate::str::contains("identity:").not())
        .stderr(predicate::str::contains("failed to decrypt key"));
}

#[test]
fn key_dev_insecure_plaintext_flag_roundtrips_without_passphrase() {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let key_path = tempdir.path().join("identity.key");

    Command::cargo_bin("agoramesh-cli")
        .expect("find cli binary")
        .args([
            "--key-path",
            path_text(&key_path),
            "--dev-insecure-plaintext-key",
            "key",
            "generate",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("plaintext key"));

    let key_file = std::fs::read_to_string(&key_path).expect("read generated key file");
    assert!(key_file.contains("dev-plaintext"));
    assert!(key_file.contains("secret_seed"));

    Command::cargo_bin("agoramesh-cli")
        .expect("find cli binary")
        .args([
            "--key-path",
            path_text(&key_path),
            "--dev-insecure-plaintext-key",
            "key",
            "show",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identity: "))
        .stdout(predicate::str::contains("secret_seed:").not());
}

fn generate_encrypted_key(key_path: &std::path::Path) {
    Command::cargo_bin("agoramesh-cli")
        .expect("find cli binary")
        .args(["--key-path", path_text(key_path), "key", "generate"])
        .write_stdin("correct horse battery staple\ncorrect horse battery staple\n")
        .assert()
        .success();
}

fn path_text(path: &std::path::Path) -> &str {
    path.to_str().expect("utf-8 temp path")
}
