# Category ID Spec

## Phase 1 provisional

Phase 1 uses a minimal embedded charter anchor inside the `category` object instead of a separate `category_charter` object. The embedded anchor contains the charter text, protocol version, and creation timestamp. The category ID is derived from the hash of that anchor plus creator metadata, exactly as specified below under "ID Generation Formula".

Full `category_charter` object validation, charter amendments, and governance rules are deferred to Phase 3. In Phase 1, receivers only need to verify that the embedded `initial_charter` hashes to `initial_charter_hash` and that the category ID is recomputed from the stable preimage order.

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

`initial_charter_hash` is computed as `SHA-256(canonical_json_bytes(initial_charter_anchor_body))` for the embedded Phase 1 charter anchor. A separate `category_charter` object is deferred to Phase 3.

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

This fixed-order category-id preimage is deliberately narrower than the shared AgoraMesh canonical JSON encoder used for signed payloads and object bodies, which sorts object keys recursively. `category_id` implementations MUST NOT sort the five input fields alphabetically before hashing.

These rules are what make the identifier portable across clients and nodes.

## Golden Test Vector

Input fixture:

```json
{
  "creator_pubkey": "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
  "display_name": "Local Tools",
  "created_at": "2024-01-02T03:04:05Z",
  "initial_charter": {
    "created_at": "2024-01-02T03:04:05Z",
    "protocol_version": 1,
    "text": "Keep tests deterministic"
  }
}
```

Initial charter canonical bytes:

```text
{"created_at":"2024-01-02T03:04:05Z","protocol_version":1,"text":"Keep tests deterministic"}
```

`initial_charter_hash`:

```text
d969b390d6ebc04d0d4ce96fb5ac1627c6b8649b7d9b60943186f4cf3b370b52
```

Category ID canonical bytes:

```text
{"protocol_version":1,"creator_pubkey":"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f","display_name":"Local Tools","created_at":"2024-01-02T03:04:05Z","initial_charter_hash":"d969b390d6ebc04d0d4ce96fb5ac1627c6b8649b7d9b60943186f4cf3b370b52"}
```

`category_id`:

```text
1b24f95eb2d42ba6df9e6eb7494184341bc11cf73a353350f583483579047e9d
```

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
