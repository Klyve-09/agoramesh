//! Phase 1 object validation tests.
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::missing_const_for_fn,
        clippy::panic,
        reason = "integration tests use convenient panicking APIs"
    )
)]

use agoramesh_core::objects::{
    ParentKind, acceptance, category, comment, post, revocation_certificate, user_profile,
    validation,
};
use agoramesh_core::{Clock, Keypair, Message};
use base64::Engine;
use chrono::{DateTime, TimeDelta, Utc};

fn utc(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).expect("valid timestamp")
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn tamper_body_json(message: &Message, extra_field: (&str, serde_json::Value)) -> Message {
    let mut value: serde_json::Value = serde_json::to_value(message).expect("serialize");
    let payload = value.get_mut("signed_payload").expect("signed_payload");
    let body_b64 = payload
        .get_mut("body")
        .and_then(|body| body.as_str())
        .expect("body is base64 string");
    let mut body: serde_json::Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(body_b64)
            .expect("decode body"),
    )
    .expect("body json");
    body.as_object_mut()
        .expect("body object")
        .insert(extra_field.0.to_owned(), extra_field.1);
    let tampered_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&body).expect("encode body"));
    payload
        .as_object_mut()
        .expect("payload object")
        .insert("body".to_owned(), serde_json::Value::String(tampered_b64));
    serde_json::from_value(value).expect("deserialize tampered")
}

#[derive(Debug, Default)]
struct FixedClock {
    now: DateTime<Utc>,
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

fn tamper_signature(message: &Message) -> Message {
    let mut value = serde_json::to_value(message).expect("serialize message");
    let signature = value
        .get_mut("signature")
        .and_then(serde_json::Value::as_array_mut)
        .expect("signature array");
    let first = signature.first_mut().expect("signature byte");
    *first = serde_json::Value::Number(0_u64.into());
    serde_json::from_value(value).expect("deserialize tampered signature")
}

fn tamper_object_id(message: &Message) -> Message {
    let mut value = serde_json::to_value(message).expect("serialize message");
    value.as_object_mut().expect("message object").insert(
        "id".to_owned(),
        serde_json::Value::Array(vec![0_u64.into(); 32]),
    );
    serde_json::from_value(value).expect("deserialize tampered id")
}

#[test]
fn user_profile_validates() {
    let keypair = Keypair::generate();
    let message =
        user_profile::create(&keypair, utc(1_700_000_000), "Ada", Some("builder")).expect("create");
    validation::validate_phase1_message(&message).expect("validate");
}

#[test]
fn user_profile_rejects_empty_display_name() {
    let keypair = Keypair::generate();
    let message =
        user_profile::create(&keypair, utc(1_700_000_000), "", None::<String>).expect("create");
    assert!(matches!(
        validation::validate_phase1_message(&message),
        Err(validation::Error::EmptyField { field }) if field == "display_name"
    ));
}

#[test]
fn user_profile_rejects_mismatched_scope() {
    let keypair = Keypair::generate();
    let mut message =
        user_profile::create(&keypair, utc(1_700_000_000), "Ada", None::<&str>).expect("create");
    let mut value: serde_json::Value = serde_json::to_value(&message).expect("serialize");
    value
        .as_object_mut()
        .expect("object")
        .get_mut("signed_payload")
        .expect("payload")
        .as_object_mut()
        .expect("payload object")
        .insert(
            "scope".to_owned(),
            serde_json::Value::String("wrong".to_owned()),
        );
    message = serde_json::from_value(value).expect("deserialize");
    assert!(matches!(
        validation::validate_phase1_message(&message),
        Err(validation::Error::FieldMismatch { field, .. }) if field == "scope"
    ));
}

#[test]
fn category_validates() {
    let keypair = Keypair::generate();
    let message = category::create(
        &keypair,
        utc(1_700_000_000),
        "Local Tools",
        "discussion",
        "charter text",
    )
    .expect("create");
    validation::validate_phase1_message(&message).expect("validate");
}

#[test]
fn category_rejects_unknown_body_field() {
    let keypair = Keypair::generate();
    let message = category::create(
        &keypair,
        utc(1_700_000_000),
        "Local Tools",
        "discussion",
        "charter text",
    )
    .expect("create");
    let tampered = tamper_body_json(
        &message,
        ("extra_field", serde_json::Value::String("oops".to_owned())),
    );
    assert!(matches!(
        validation::validate_phase1_message(&tampered),
        Err(validation::Error::InvalidBody { kind, .. }) if kind == "category"
    ));
}

#[test]
fn category_rejects_tampered_category_id() {
    let keypair = Keypair::generate();
    let message = category::create(
        &keypair,
        utc(1_700_000_000),
        "Local Tools",
        "discussion",
        "charter text",
    )
    .expect("create");
    let tampered = tamper_body_json(
        &message,
        ("category_id", serde_json::Value::String("0".repeat(64))),
    );
    assert!(matches!(
        validation::validate_phase1_message(&tampered),
        Err(validation::Error::FieldMismatch { field, .. }) if field == "category_id"
    ));
}

#[test]
fn category_rejects_tampered_initial_charter_hash() {
    let keypair = Keypair::generate();
    let message = category::create(
        &keypair,
        utc(1_700_000_000),
        "Local Tools",
        "discussion",
        "charter text",
    )
    .expect("create");
    let tampered = tamper_body_json(
        &message,
        (
            "initial_charter_hash",
            serde_json::Value::String("0".repeat(64)),
        ),
    );
    assert!(matches!(
        validation::validate_phase1_message(&tampered),
        Err(validation::Error::HashMismatch { field }) if field == "initial_charter_hash"
    ));
}

#[test]
fn post_validates() {
    let keypair = Keypair::generate();
    let message =
        post::create(&keypair, "category-123", "hello", utc(1_700_000_000)).expect("create");
    validation::validate_phase1_message(&message).expect("validate");
}

#[test]
fn post_rejects_empty_text() {
    let keypair = Keypair::generate();
    let message = post::create(&keypair, "category-123", "", utc(1_700_000_000)).expect("create");
    assert!(matches!(
        validation::validate_phase1_message(&message),
        Err(validation::Error::EmptyField { field }) if field == "text"
    ));
}

#[test]
fn comment_validates() {
    let keypair = Keypair::generate();
    let post = post::create(&keypair, "category-123", "hello", utc(1_700_000_000)).expect("create");
    let message = comment::create(
        &keypair,
        "category-123",
        ParentKind::Post,
        post.id(),
        "reply",
        utc(1_700_000_001),
    )
    .expect("create");
    validation::validate_phase1_message(&message).expect("validate");
}

#[test]
fn comment_rejects_invalid_parent_id() {
    let keypair = Keypair::generate();
    let message = comment::create(
        &keypair,
        "category-123",
        ParentKind::Post,
        post::create(&keypair, "category-123", "hello", utc(1_700_000_000))
            .expect("create")
            .id(),
        "reply",
        utc(1_700_000_001),
    )
    .expect("create");
    let tampered = tamper_body_json(
        &message,
        ("parent_id", serde_json::Value::String("not-hex".to_owned())),
    );
    assert!(matches!(
        validation::validate_phase1_message(&tampered),
        Err(validation::Error::InvalidHex { field, .. }) if field == "parent_id"
    ));
}

#[test]
fn revocation_certificate_validates_self_revocation() {
    let keypair = Keypair::generate();
    let message =
        revocation_certificate::create(&keypair, None, utc(1_700_000_010), "key_compromised")
            .expect("create");
    validation::validate_phase1_message(&message).expect("validate");
}

#[test]
fn revocation_certificate_rejects_third_party_revocation() {
    let target = Keypair::generate();
    let attacker = Keypair::generate();
    let message =
        revocation_certificate::create(&attacker, None, utc(1_700_000_010), "key_compromised")
            .expect("create self-revocation");

    let tampered = tamper_body_json(
        &message,
        (
            "revoked_pubkey",
            serde_json::Value::String(to_hex(target.identity().verifying_key().as_bytes())),
        ),
    );
    assert!(matches!(
        validation::validate_phase1_message(&tampered),
        Err(validation::Error::AuthorMismatch { field, .. }) if field == "revoked_pubkey"
    ));
}

#[test]
fn unknown_type_rejects() {
    let keypair = Keypair::generate();
    let message = Message::create(
        &keypair,
        "unknown_type",
        utc(1_700_000_000),
        "scope".to_owned(),
        b"{}",
    )
    .expect("create");
    assert!(matches!(
        validation::validate_phase1_message(&message),
        Err(validation::Error::UnknownType(kind)) if kind == "unknown_type"
    ));
}

#[test]
fn acceptance_rejects_invalid_signature_before_clock_and_semantics() {
    let keypair = Keypair::generate();
    let now = utc(1_700_000_000);
    let message = post::create(
        &keypair,
        "category-123",
        "",
        now + TimeDelta::seconds(agoramesh_core::message::CLOCK_SKEW_REJECT_SECONDS + 1),
    )
    .expect("create post");
    let tampered = tamper_signature(&message);
    let clock = FixedClock { now };
    let context = acceptance::AcceptanceContext::phase1(&clock);

    assert!(matches!(
        acceptance::validate_phase1_for_acceptance(&tampered, &context),
        Err(acceptance::Error::Integrity(
            agoramesh_core::message::Error::InvalidSignature { .. }
        ))
    ));
}

#[test]
fn acceptance_rejects_object_id_before_clock_and_semantics() {
    let keypair = Keypair::generate();
    let now = utc(1_700_000_000);
    let message = post::create(
        &keypair,
        "category-123",
        "",
        now + TimeDelta::seconds(agoramesh_core::message::CLOCK_SKEW_REJECT_SECONDS + 1),
    )
    .expect("create post");
    let tampered = tamper_object_id(&message);
    let clock = FixedClock { now };
    let context = acceptance::AcceptanceContext::phase1(&clock);

    assert!(matches!(
        acceptance::validate_phase1_for_acceptance(&tampered, &context),
        Err(acceptance::Error::Integrity(
            agoramesh_core::message::Error::ObjectIdMismatch
        ))
    ));
}

#[test]
fn acceptance_rejects_clock_before_phase1_semantics() {
    let keypair = Keypair::generate();
    let now = utc(1_700_000_000);
    let message = post::create(
        &keypair,
        "category-123",
        "",
        now + TimeDelta::seconds(agoramesh_core::message::CLOCK_SKEW_REJECT_SECONDS + 1),
    )
    .expect("create post");
    let clock = FixedClock { now };
    let context = acceptance::AcceptanceContext::phase1(&clock);

    assert!(matches!(
        acceptance::validate_phase1_for_acceptance(&message, &context),
        Err(acceptance::Error::Clock(
            agoramesh_core::message::Error::ClockSkewTooLarge { .. }
        ))
    ));
}

#[test]
fn acceptance_rejects_phase1_semantics_after_integrity_and_clock() {
    let keypair = Keypair::generate();
    let now = utc(1_700_000_000);
    let message = post::create(&keypair, "category-123", "", now).expect("create post");
    let clock = FixedClock { now };
    let context = acceptance::AcceptanceContext::phase1(&clock);

    assert!(matches!(
        acceptance::validate_phase1_for_acceptance(&message, &context),
        Err(acceptance::Error::Semantic(validation::Error::EmptyField { field }))
            if field == "text"
    ));
}
