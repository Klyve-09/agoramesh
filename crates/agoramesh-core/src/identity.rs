//! Identity primitives: stable identifiers backed by Ed25519 keypairs.

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
}
