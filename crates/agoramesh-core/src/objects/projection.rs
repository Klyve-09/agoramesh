//! Typed Phase 1 object body projection.

use serde::de::DeserializeOwned;

use crate::Message;
use crate::objects::{category, comment, post, revocation_certificate, user_profile, validation};

/// A typed Phase 1 object that can be projected from a signed message body.
pub trait Phase1Object {
    /// Signed payload kind for this object type.
    const KIND: &'static str;
    /// Typed body carried by the signed payload.
    type Body: DeserializeOwned;
}

/// User profile projection marker.
#[derive(Clone, Copy, Debug)]
pub struct UserProfileObject;

/// Category projection marker.
#[derive(Clone, Copy, Debug)]
pub struct CategoryObject;

/// Post projection marker.
#[derive(Clone, Copy, Debug)]
pub struct PostObject;

/// Comment projection marker.
#[derive(Clone, Copy, Debug)]
pub struct CommentObject;

/// Revocation certificate projection marker.
#[derive(Clone, Copy, Debug)]
pub struct RevocationCertificateObject;

impl Phase1Object for UserProfileObject {
    const KIND: &'static str = "user_profile";
    type Body = user_profile::Body;
}

impl Phase1Object for CategoryObject {
    const KIND: &'static str = "category";
    type Body = category::Body;
}

impl Phase1Object for PostObject {
    const KIND: &'static str = "post";
    type Body = post::Body;
}

impl Phase1Object for CommentObject {
    const KIND: &'static str = "comment";
    type Body = comment::Body;
}

impl Phase1Object for RevocationCertificateObject {
    const KIND: &'static str = "revocation_certificate";
    type Body = revocation_certificate::Body;
}

/// Decodes a message body as the requested Phase 1 object type.
///
/// # Errors
/// Returns a validation error when the kind differs or the body is invalid.
pub fn decode_body<T>(message: &Message) -> Result<T::Body, validation::Error>
where
    T: Phase1Object,
{
    if message.signed_payload().kind() != T::KIND {
        return Err(validation::Error::WrongKind {
            expected: T::KIND.to_owned(),
            actual: message.signed_payload().kind().to_owned(),
        });
    }
    serde_json::from_slice(message.body()).map_err(|error| validation::Error::InvalidBody {
        kind: T::KIND.to_owned(),
        message: error.to_string(),
    })
}

/// Decodes the body when the message kind matches, otherwise returns `None`.
///
/// # Errors
/// Returns a validation error when the kind matches but the body is invalid.
pub fn maybe_decode_body<T>(message: &Message) -> Result<Option<T::Body>, validation::Error>
where
    T: Phase1Object,
{
    if message.signed_payload().kind() == T::KIND {
        return decode_body::<T>(message).map(Some);
    }
    Ok(None)
}
