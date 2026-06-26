# ADR 0006: Envelope Signatures

## Status
Accepted

## Context
AgoraMesh uses one common object envelope for protocol objects such as `post`, `comment`, `vote`, `report`, `category_state`, `moderation_action`, `category_bundle`, `charter_amendment`, and recovery declarations.

The roadmap already fixes the shared envelope fields as `type`, `protocol_version`, `schema_version`, `object_id`, `created_at`, `author_pubkey`, `scope` or `category_id`, `body`, and `signature`. It also fixes `signing_payload` as `type`, `protocol_version`, `schema_version`, `created_at`, `author_pubkey`, `scope`, and `body`, with `signature` and transport or local metadata excluded. `object_id` is the hash of `canonical(signing_payload)`.

That leaves two design questions open.

1. Some objects need one author signature.
2. Other objects need a quorum of signers, and the quorum must stay stable even when signatures arrive later.

This ADR defines how both forms work without changing the immutable object model.

## Decision
### Single signature envelopes
User authored objects such as `post`, `comment`, `vote`, and similar one author records use a single signature.

The `signature` field holds one signature from `author_pubkey` over `canonical(signing_payload)`.

For these objects, the envelope is valid only when that one signature verifies against the declared author key and the declared schema version.

### Multi signature envelopes
Objects that represent shared authority, such as `category_state`, `moderation_action`, `category_bundle`, `charter_amendment`, and recovery declarations, use a multi signature form when the charter or recovery policy requires it.

The signed body of the object declares the authorization set with `required_quorum` and `signer_pubkeys`. Those fields are part of `body`, so they are included in `signing_payload` and are covered by the object hash.

The `signature` field then stores a deterministic set of signature records. Each record contains the signer public key and that signer’s signature over the same `canonical(signing_payload)`.

### Ordering and canonicalization
Multiple signers are ordered by the canonical byte encoding of `signer_pubkey` in ascending lexicographic order.

The canonical body for a multi signature object must list `signer_pubkeys` in that same order. The signature set in the envelope must also serialize in that same order.

Every signer signs the exact same `canonical(signing_payload)`. No signer signs another signer’s signature record, and the object hash never depends on the signature set.

Because `object_id` is computed only from `canonical(signing_payload)`, the same logical object always has the same `object_id` across implementations, even when its signature set is still growing.

### Quorum declaration and verification
`required_quorum` is the minimum number of valid signatures needed for acceptance.

`signer_pubkeys` is the full authorized signer set for that object.

Verification rules are:

1. The declared signer set is canonical and unique.
2. `required_quorum` is at least 1 and no larger than the size of `signer_pubkeys`.
3. Each signature must verify against `canonical(signing_payload)`.
4. Each valid signature must belong to one of the declared signer pubkeys.
5. A signer may appear only once.
6. The object is accepted only when the count of valid signatures reaches `required_quorum`.

### Signatures added after object creation
Adding a signature later does not create a new logical object and does not change `object_id`.

Instead, the network treats the object as the same content addressed record with a larger signature set. A `category_state` object signed by 2 of 3 admins is the same object before and after the second signature arrives. It becomes valid only once the quorum is met.

Implementations may merge later signatures into the stored envelope, but they must preserve the original `body` and `object_id`. They must not rewrite the payload to chase new signatures.

### Revocation
Revocation is prospective, not retroactive.

A revoked key cannot validate future signatures once its revocation certificate is effective. Objects signed before that effective point remain valid and keep their original `object_id`.

Validation uses the revocation state that applies to the object’s `created_at`, not the local receive time. A revoked key does not rewrite history, and already accepted objects do not become invalid just because the key was later revoked.

If a multi signature object still reaches quorum using only active keys, it remains valid. If quorum depends on a revoked key after the revocation is effective, the object fails verification.

### Deterministic hash requirement
The same logical object bytes must produce the same `object_id` across every implementation.

That means canonical serialization of `signing_payload` must be stable, field order must not depend on language runtime behavior, and the canonical ordering rules for `signer_pubkeys` and signature records must be identical everywhere.

If two implementations disagree on `object_id` for the same logical object, one of them is wrong.

### Security considerations
Replay, signature stripping, and downgrade attacks are all handled by the same rules.

1. Replay is blocked by content addressing and by type specific freshness rules in the body, such as `previous_state_hash` and `state_epoch` for state objects.
2. Signature stripping does not create a valid smaller object. A multi signature object without quorum is simply invalid.
3. Downgrade attacks fail because quorum data lives in the signed body, so an attacker cannot lower `required_quorum` or swap `signer_pubkeys` without breaking the hash and the signatures.
4. Implementations must reject any object whose schema version changes signature semantics without a matching protocol rule.

## Consequences
This design keeps object identity immutable while still letting governance objects accumulate signatures over time.

It also makes verification rules straightforward. Single author objects have one signature path. Quorum objects have one canonical payload, one canonical signer order, and a clear accept or reject threshold.

The tradeoff is that every implementation must follow the same canonical ordering rules exactly. Any drift in serialization, signer ordering, or quorum checks will fork validation.

## References
1. `docs/v1.0-roadmap.md`
2. `docs/agoramesh-design-notes.md`
