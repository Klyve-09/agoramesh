//! Phase 1 typed object builder integration tests.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::missing_const_for_fn,
        reason = "integration test helpers use convenient panicking APIs"
    )
)]

use agoramesh_core::canonical;
use agoramesh_core::message::{MessageId, PROTOCOL_VERSION};
use agoramesh_core::objects::{
    ParentKind, category, category_id, comment, post, revocation_certificate, user_profile,
};
use agoramesh_core::{Keypair, Message};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

const CATEGORY_ID_FIXTURE_CREATOR: &str =
    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const CATEGORY_ID_FIXTURE_CANONICAL: &str = "{\"protocol_version\":1,\"creator_pubkey\":\"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\",\"display_name\":\"Local Tools\",\"created_at\":\"2024-01-02T03:04:05Z\",\"initial_charter_hash\":\"d969b390d6ebc04d0d4ce96fb5ac1627c6b8649b7d9b60943186f4cf3b370b52\"}";
const CATEGORY_ID_FIXTURE_CHARTER_HASH: &str =
    "d969b390d6ebc04d0d4ce96fb5ac1627c6b8649b7d9b60943186f4cf3b370b52";
const CATEGORY_ID_FIXTURE_ID: &str =
    "1b24f95eb2d42ba6df9e6eb7494184341bc11cf73a353350f583483579047e9d";

fn utc(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).expect("valid timestamp")
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex(&hasher.finalize())
}

#[test]
fn message_create_accepts_signed_payload_kind() {
    let keypair = Keypair::generate();
    let message = Message::create(
        &keypair,
        "post",
        utc(1_700_000_000),
        "category-id".to_owned(),
        b"{}",
    )
    .expect("create message");

    assert_eq!(message.signed_payload().kind(), "post");
}

#[test]
fn message_id_roundtrips_lowercase_hex() {
    let id = MessageId::from([0xab; 32]);
    let encoded = id.to_hex();

    assert_eq!(encoded.len(), 64);
    assert_eq!(encoded, "ab".repeat(32));
    assert_eq!(MessageId::from_hex(&encoded).expect("decode hex"), id);
}

#[test]
fn user_profile_builder_sets_author_scope_and_canonical_body() {
    let keypair = Keypair::generate();
    let created_at = utc(1_700_000_000);

    let message =
        user_profile::create(&keypair, created_at, "Ada", Some("builder")).expect("create profile");
    let body: user_profile::Body = serde_json::from_slice(message.body()).expect("decode body");
    let expected_body = user_profile::Body {
        display_name: "Ada".to_owned(),
        bio: Some("builder".to_owned()),
    };

    assert_eq!(message.signed_payload().kind(), "user_profile");
    assert_eq!(body, expected_body);
    assert_eq!(
        message.body(),
        canonical::to_vec(&expected_body).expect("canonical body")
    );
    assert_eq!(
        message.signed_payload().scope(),
        format!(
            "user:{}",
            hex(keypair.identity().verifying_key().as_bytes())
        )
    );
}

#[test]
fn category_builder_derives_deterministic_id_from_spec_inputs() {
    let keypair = Keypair::generate();
    let created_at = utc(1_700_000_000);
    let initial_charter_text = "Discuss durable local-first tools.";

    let message = category::create(
        &keypair,
        created_at,
        "Local Tools",
        "Local-first software discussion",
        initial_charter_text,
    )
    .expect("create category");
    let body: category::Body = serde_json::from_slice(message.body()).expect("decode body");
    let charter_anchor = category::CharterAnchorBody {
        text: initial_charter_text.to_owned(),
        protocol_version: PROTOCOL_VERSION,
        created_at,
    };
    let charter_hash = sha256_hex(&canonical::to_vec(&charter_anchor).expect("charter bytes"));
    let creator_pubkey = hex(keypair.identity().verifying_key().as_bytes());
    let input = category_id::CategoryIdParts {
        creator_pubkey: &creator_pubkey,
        display_name: "Local Tools",
        created_at: &created_at,
        initial_charter_hash: &charter_hash,
    };
    let expected_category_id = category_id::compute(&input).expect("id bytes");

    assert_eq!(message.signed_payload().kind(), "category");
    assert_eq!(message.signed_payload().scope(), expected_category_id);
    assert_eq!(body.category_id, expected_category_id);
    assert_eq!(body.initial_charter_hash, charter_hash);
    assert_eq!(body.initial_charter, charter_anchor);
}

#[test]
fn category_id_golden_vector_uses_spec_field_order() {
    let created_at = DateTime::parse_from_rfc3339("2024-01-02T03:04:05Z")
        .expect("parse fixture time")
        .with_timezone(&Utc);
    let parts = category_id::CategoryIdParts {
        creator_pubkey: CATEGORY_ID_FIXTURE_CREATOR,
        display_name: "Local Tools",
        created_at: &created_at,
        initial_charter_hash: CATEGORY_ID_FIXTURE_CHARTER_HASH,
    };

    let canonical_bytes = category_id::canonical_bytes(&parts).expect("canonical bytes");
    let category_id = category_id::compute(&parts).expect("category id");

    assert_eq!(canonical_bytes, CATEGORY_ID_FIXTURE_CANONICAL.as_bytes());
    assert_eq!(CATEGORY_ID_FIXTURE_CHARTER_HASH, sha256_hex(br#"{"created_at":"2024-01-02T03:04:05Z","protocol_version":1,"text":"Keep tests deterministic"}"#));
    assert_eq!(category_id, CATEGORY_ID_FIXTURE_ID);
}

#[test]
fn category_id_canonical_bytes_use_utc_seconds_precision() {
    let created_at = DateTime::parse_from_rfc3339("2024-01-02T03:04:05.987654321+00:00")
        .expect("parse fixture time")
        .with_timezone(&Utc);
    let parts = category_id::CategoryIdParts {
        creator_pubkey: CATEGORY_ID_FIXTURE_CREATOR,
        display_name: "Local Tools",
        created_at: &created_at,
        initial_charter_hash: CATEGORY_ID_FIXTURE_CHARTER_HASH,
    };

    let canonical_bytes = category_id::canonical_bytes(&parts).expect("canonical bytes");

    assert_eq!(canonical_bytes, CATEGORY_ID_FIXTURE_CANONICAL.as_bytes());
}

#[test]
fn post_and_comment_builders_use_category_scope_and_typed_parent() {
    let keypair = Keypair::generate();
    let category_id = "category-123";
    let created_at = utc(1_700_000_000);

    let post = post::create(&keypair, category_id, "First post", created_at).expect("create post");
    let comment = comment::create(
        &keypair,
        category_id,
        ParentKind::Post,
        post.id(),
        "First reply",
        created_at,
    )
    .expect("create comment");
    let post_body: post::Body = serde_json::from_slice(post.body()).expect("decode post");
    let comment_body: comment::Body =
        serde_json::from_slice(comment.body()).expect("decode comment");

    assert_eq!(post.signed_payload().kind(), "post");
    assert_eq!(post.signed_payload().scope(), category_id);
    assert_eq!(post_body.category_id, category_id);
    assert_eq!(comment.signed_payload().kind(), "comment");
    assert_eq!(comment.signed_payload().scope(), category_id);
    assert_eq!(comment_body.parent_kind, ParentKind::Post);
    assert_eq!(comment_body.parent_id, post.id().to_hex());
}

#[test]
fn revocation_certificate_rejects_messages_at_or_after_effective_time() {
    let revoked = Keypair::generate();
    let replacement = Keypair::generate();
    let effective_at = utc(1_700_000_010);
    let before = post::create(&revoked, "category-123", "before", utc(1_700_000_009))
        .expect("create before");
    let after =
        post::create(&revoked, "category-123", "after", effective_at).expect("create after");
    let certificate = revocation_certificate::create(
        &revoked,
        Some(*replacement.identity().verifying_key().as_bytes()),
        effective_at,
        "key_compromised",
    )
    .expect("create revocation");

    assert!(
        revocation_certificate::allows_message(&certificate, &before).expect("validate before")
    );
    assert!(!revocation_certificate::allows_message(&certificate, &after).expect("validate after"));
    assert_eq!(
        certificate.signed_payload().scope(),
        format!(
            "revocation:{}",
            hex(revoked.identity().verifying_key().as_bytes())
        )
    );
}
