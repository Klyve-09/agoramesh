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
    ParentKind, category, comment, post, revocation_certificate, user_profile,
};
use agoramesh_core::{Keypair, Message};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

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

#[derive(Serialize)]
struct ExpectedCategoryIdInput<'a> {
    protocol_version: u32,
    creator_pubkey: &'a str,
    display_name: &'a str,
    created_at: &'a DateTime<Utc>,
    initial_charter_hash: &'a str,
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
    let input = ExpectedCategoryIdInput {
        protocol_version: PROTOCOL_VERSION,
        creator_pubkey: &creator_pubkey,
        display_name: "Local Tools",
        created_at: &created_at,
        initial_charter_hash: &charter_hash,
    };
    let expected_category_id = sha256_hex(&canonical::to_vec(&input).expect("id bytes"));

    assert_eq!(message.signed_payload().kind(), "category");
    assert_eq!(message.signed_payload().scope(), expected_category_id);
    assert_eq!(body.category_id, expected_category_id);
    assert_eq!(body.initial_charter_hash, charter_hash);
    assert_eq!(body.initial_charter, charter_anchor);
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
    let authority = Keypair::generate();
    let effective_at = utc(1_700_000_010);
    let before = post::create(&revoked, "category-123", "before", utc(1_700_000_009))
        .expect("create before");
    let after =
        post::create(&revoked, "category-123", "after", effective_at).expect("create after");
    let certificate = revocation_certificate::create(
        &authority,
        *revoked.identity().verifying_key().as_bytes(),
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
