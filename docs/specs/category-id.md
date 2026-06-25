# Category ID Spec

## Overview

`category_id` is the stable, internal identifier for a category in AgoraMesh. It is derived once, at creation time, from the category's first charter and creation metadata, then treated as immutable for the life of that category.

The hash input is:

1. `initial_charter_hash`
2. `creator_pubkey`
3. `display_name`
4. `created_at`
5. `protocol_version`

`initial_charter_hash` is the hash of the first `category_charter` object's `signing_payload`, computed before any amendment exists. After that first charter is published, later charter changes do not rewrite the original hash input, so `category_id` stays fixed.

Users and nodes MUST reference a category by `category_id` internally. `display_name` is for UI and presentation only.

## ID Generation Formula

Create a canonical JSON object with exactly these fields and values:

```json
{
  "protocol_version": 1,
  "creator_pubkey": "<creator pubkey>",
  "display_name": "<display name>",
  "created_at": "<RFC 3339 timestamp>",
  "initial_charter_hash": "<charter hash>"
}
```

Compute `category_id` as:

```text
category_id = SHA-256(canonical_json_bytes(category_id_input))
```

The output is encoded as a lowercase hex string.

`initial_charter_hash` is computed the same way, but from the first `category_charter` object's `signing_payload`.

## Input Canonicalization

Canonicalization rules are strict. Two implementations that receive the same logical category data MUST produce the same bytes before hashing.

1. `protocol_version` is the AgoraMesh major protocol version, encoded as a JSON number with no leading zeroes.
2. `creator_pubkey` is the exact creator public key value used by the protocol, encoded as a JSON string with no extra formatting.
3. `display_name` is the exact published display name, encoded as a JSON string. Do not trim, case fold, or normalize it for hashing.
4. `created_at` is the exact RFC 3339 creation timestamp, encoded as a JSON string.
5. `initial_charter_hash` is the lowercase hex hash of the first charter's `signing_payload`.
6. Canonical JSON MUST use a fixed field order: `protocol_version`, `creator_pubkey`, `display_name`, `created_at`, `initial_charter_hash`.
7. Canonical JSON MUST use UTF-8, no insignificant whitespace, and no extra fields.
8. String values MUST be serialized exactly once. No locale rules, display rules, or transport rewrites are allowed before hashing.

These rules are what make the identifier portable across clients and nodes.

## Validation

When a client or node receives a category object, it MUST apply all of the following checks:

1. The object MUST include `protocol_version`, `creator_pubkey`, `display_name`, `created_at`, `initial_charter_hash`, and `category_id`.
2. The receiver MUST recompute `category_id` from the canonical input object and reject the category if the supplied value does not match.
3. The category object's signature MUST verify against `creator_pubkey`.
4. `initial_charter_hash` MUST resolve to a valid first `category_charter` object whose `signing_payload` hash matches the value in the category object.
5. The first `category_charter` signature MUST verify against the charter author's public key.
6. The receiver MUST treat the category object as immutable. A later `display_name` change or charter amendment does not modify the original `category_id`.
7. If the category receives a new display name, that change MUST appear as a new `category_charter` or `charter_amendment` object that still points at the same `category_id`.
8. The client MUST NOT create a new `category_id` when only `display_name` changes.
9. The client MUST NOT create a new `category_id` when the charter is amended.
10. If a received object looks like the same community but hashes to a different `category_id`, the receiver MUST treat it as a different category.
11. The receiver SHOULD reject malformed timestamps, invalid public keys, and empty display names as invalid category input.

## Fork Behavior

A hard fork creates a new category, not a modified copy of the old one.

The fork starts with a new first `category_charter` object. That charter has a new `signing_payload`, so it produces a new `initial_charter_hash`, which in turn produces a new `category_id`.

This means a fork may reuse the old creator, a similar display name, or related charter text, but it still becomes a new category because the initial charter hash changes. Existing subscriptions and references stay attached to the old `category_id` unless users explicitly move.

## Security Notes

1. Collision resistance depends on the hash function and on keeping all five inputs in the hash preimage. Do not drop fields, truncate the digest, or compare only hash prefixes.
2. `display_name` is not identity. Two categories can share the same name, and one category can change its name without changing identity.
3. `initial_charter_hash` is the anchor that prevents charter edits from rewriting history. Amendments add history, they do not replace the first charter.
4. Canonical serialization must stay boring and exact. Any extra normalization, field reordering, or whitespace handling change can split identities across clients.
5. There is no centralized registry assumption in this spec. Identity comes from the signed object chain and the deterministic hash input, not from a server side lookup table.
