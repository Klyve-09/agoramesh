//! Identity primitives: stable identifiers backed by Ed25519 keypairs.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A stable, public-facing mesh identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Identity {
    verifying_key: VerifyingKey,
    id: [u8; 32],
}

impl Identity {
    /// Creates an identity from a raw Ed25519 verifying key.
    #[must_use]
    pub fn from_verifying_key(verifying_key: VerifyingKey) -> Self {
        let id = hash_key(&verifying_key);
        Self { verifying_key, id }
    }

    /// Returns the 32-byte stable identifier used in the mesh.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.id
    }

    /// Returns the underlying Ed25519 verifying key.
    #[must_use]
    pub const fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }
}

/// An owned Ed25519 keypair used to authenticate as a mesh peer.
#[derive(Clone, Debug)]
pub struct Keypair {
    signing_key: SigningKey,
}

impl Keypair {
    /// Generates a new random keypair.
    #[must_use]
    pub fn generate() -> Self {
        let mut csprng = rand::rngs::OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Returns the public identity corresponding to this keypair.
    #[must_use]
    pub fn identity(&self) -> Identity {
        Identity::from_verifying_key(self.signing_key.verifying_key())
    }

    /// Signs the given byte slice with this keypair.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Exports this keypair's 32-byte Ed25519 secret seed.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Restores a keypair from a 32-byte Ed25519 secret seed.
    ///
    /// # Errors
    /// Reserved for future seed validation failures.
    pub fn from_bytes(seed: &[u8; 32]) -> Result<Self, Error> {
        Ok(Self {
            signing_key: SigningKey::from_bytes(seed),
        })
    }

    /// Exports this keypair's secret seed as base64url without padding.
    #[must_use]
    pub fn to_base64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.to_bytes())
    }

    /// Restores a keypair from a base64url-encoded 32-byte secret seed.
    ///
    /// # Errors
    /// Returns [`Error::InvalidSeedEncoding`] when the seed is not base64url and
    /// [`Error::InvalidSeedLength`] when the decoded seed is not 32 bytes.
    pub fn from_base64(encoded: &str) -> Result<Self, Error> {
        let decoded = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|error| Error::InvalidSeedEncoding(error.to_string()))?;
        let seed: [u8; 32] = decoded
            .try_into()
            .map_err(|bytes: Vec<u8>| Error::InvalidSeedLength(bytes.len()))?;
        Self::from_bytes(&seed)
    }
}

/// Errors that can occur while importing key material.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
    /// The base64url seed could not be decoded.
    #[error("invalid seed encoding: {0}")]
    InvalidSeedEncoding(String),

    /// The decoded seed was not 32 bytes.
    #[error("invalid seed length: expected 32 bytes, got {0}")]
    InvalidSeedLength(usize),
}

fn hash_key(key: &VerifyingKey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_generates_different_identities() {
        let first = Keypair::generate().identity();
        let second = Keypair::generate().identity();
        assert_ne!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn identity_is_stable_for_same_key() {
        let keypair = Keypair::generate();
        let identity = keypair.identity();
        assert_eq!(keypair.identity(), identity);
    }

    #[test]
    fn identity_roundtrips_through_serde() {
        let original = Keypair::generate().identity();
        let serialized = serde_json::to_vec(&original).expect("serialize identity");
        let restored: Identity = serde_json::from_slice(&serialized).expect("deserialize identity");
        assert_eq!(original, restored);
    }

    #[test]
    fn keypair_roundtrips_through_seed_bytes() {
        let original = Keypair::generate();
        let restored = Keypair::from_bytes(&original.to_bytes()).expect("restore keypair");
        assert_eq!(original.identity(), restored.identity());
    }

    #[test]
    fn keypair_roundtrips_through_base64_seed() {
        let original = Keypair::generate();
        let restored = Keypair::from_base64(&original.to_base64()).expect("restore keypair");
        assert_eq!(original.identity(), restored.identity());
    }
}
