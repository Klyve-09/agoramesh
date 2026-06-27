//! Characterization tests for keyring file structure and public-key metadata.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::unwrap_used,
    missing_docs,
    reason = "keyring structure tests assert failure paths with exact errors"
)]

use std::io::Write;

use agoramesh_cli::keyring::{
    Keyring, KeyringError, generate, load, public_key_hex, read_encrypted_public_key_for_display,
    validate_dev_plaintext_key_bytes_structure, validate_encrypted_key_bytes_structure,
};
use agoramesh_core::Keypair;
use base64::Engine;

#[test]
fn encrypted_key_roundtrips_and_exposes_metadata() {
    let encrypted = generate("correct passphrase").expect("generate encrypted key");

    let metadata =
        validate_encrypted_key_bytes_structure(&encrypted).expect("valid encrypted structure");

    let keypair = load(&encrypted, "correct passphrase").expect("load with correct passphrase");
    assert_eq!(public_key_hex(&keypair).len(), 64);
    assert_eq!(metadata.public_key_hex, public_key_hex(&keypair));
}

#[test]
fn encrypted_key_load_fails_closed_for_wrong_passphrase() {
    let encrypted = generate("correct passphrase").expect("generate encrypted key");

    let result = load(&encrypted, "wrong passphrase");

    assert!(matches!(result, Err(KeyringError::Decrypt)));
}

#[test]
fn encrypted_key_structure_rejects_missing_public_key_hex() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value
        .as_object_mut()
        .expect("object")
        .remove("public_key_hex");
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn encrypted_key_structure_rejects_invalid_public_key_hex() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value["public_key_hex"] = serde_json::Value::String("not-hex".to_owned());
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn encrypted_key_structure_rejects_wrong_public_key_hex_length() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value["public_key_hex"] = serde_json::Value::String("00".to_owned());
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn encrypted_key_structure_rejects_missing_salt() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value.as_object_mut().expect("object").remove("salt");
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn encrypted_key_structure_rejects_invalid_salt_base64() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value["salt"] = serde_json::Value::String("!!!".to_owned());
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn encrypted_key_structure_rejects_invalid_ciphertext_length() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value["ciphertext"] = serde_json::Value::String(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0_u8; 16]),
    );
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn encrypted_key_structure_rejects_unsupported_version() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value["version"] = serde_json::Value::Number(99.into());
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn encrypted_key_structure_rejects_unsupported_kdf_algorithm() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let mut value: serde_json::Value = serde_json::from_slice(&encrypted).expect("parse JSON");
    value["kdf"]["algorithm"] = serde_json::Value::String("scrypt".to_owned());
    let bytes = serde_json::to_vec(&value).expect("serialize");

    let result = validate_encrypted_key_bytes_structure(&bytes);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn dev_plaintext_structure_roundtrips() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let path = temp_dir.path().join("identity.key");
    let keyring = Keyring::new(&path);
    keyring.dev_plaintext_save().expect("save plaintext key");
    let bytes = std::fs::read(&path).expect("read key file");

    let result = validate_dev_plaintext_key_bytes_structure(&bytes);

    assert!(result.is_ok());
    let loaded = keyring.dev_plaintext_load().expect("load plaintext key");
    assert_eq!(public_key_hex(&loaded).len(), 64);
}

#[test]
fn dev_plaintext_structure_rejects_encrypted_bytes() {
    let encrypted = generate("passphrase").expect("generate encrypted key");

    let result = validate_dev_plaintext_key_bytes_structure(&encrypted);

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn dev_plaintext_structure_rejects_malformed_json() {
    let result = validate_dev_plaintext_key_bytes_structure(b"not json");

    assert!(matches!(result, Err(KeyringError::InvalidFormat(_))));
}

#[test]
fn read_encrypted_public_key_for_display_reads_metadata() {
    let encrypted = generate("passphrase").expect("generate encrypted key");
    let metadata =
        validate_encrypted_key_bytes_structure(&encrypted).expect("valid encrypted structure");
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let path = temp_dir.path().join("identity.key");
    std::fs::File::create(&path)
        .and_then(|mut file| file.write_all(&encrypted))
        .expect("write key file");

    let public_key = read_encrypted_public_key_for_display(&path).expect("read public key");

    assert_eq!(public_key, Some(metadata.public_key_hex));
}

#[test]
fn load_encrypted_key_with_passphrase_roundtrips_via_path() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let path = temp_dir.path().join("identity.key");
    let keyring = Keyring::new(&path);
    keyring.generate("secret").expect("generate key file");

    let loaded = agoramesh_cli::keyring::load_encrypted_key_with_passphrase(&path, "secret")
        .expect("load via path helper");

    assert_eq!(public_key_hex(&loaded).len(), 64);
}

#[test]
fn keypair_to_bytes_roundtrips() {
    let keypair = Keypair::generate();
    let bytes = keypair.to_bytes();
    let roundtripped =
        agoramesh_core::Keypair::from_bytes(&bytes).expect("import keypair from bytes");

    assert_eq!(public_key_hex(&keypair), public_key_hex(&roundtripped));
}
