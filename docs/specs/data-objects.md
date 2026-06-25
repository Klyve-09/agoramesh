# AgoraMesh Data Objects — Phase 1

## Scope

This spec defines the Phase 1 typed objects implemented in `agoramesh-core/src/objects/`. Phase 1 is text-only: no `media_ref`, no edit revisions, no external URL previews, and no Phase 3 governance objects (`category_charter`, `charter_amendment`, `vote`, etc.).

All Phase 1 objects share the common envelope defined in ADR 0006 and use Canonical JSON + SHA-256 for signing and object identity (ADR 0001).

## Common envelope

Every object is transported as a `Message`:

```text
Message {
  id: MessageId,              // SHA-256(canonical(signed_payload))
  author_id: Identity,
  signature: Signature,       // over canonical(signed_payload)
  signed_payload: SignedPayload,
  transport_metadata: TransportMetadata,
}

SignedPayload {
  type: String,               // object kind: "user_profile", "category", "post", "comment", "revocation_certificate"
  protocol_version: u32,      // 1 in Phase 1
  schema_version: u32,        // 1 in Phase 1
  created_at: RFC3339 String,
  author_pubkey: [u8; 32],
  scope: String,
  body: Body,                 // canonical JSON bytes, base64url-encoded on the wire
}
```

`transport_metadata` is excluded from the signature and object ID.

## Object kinds

### `user_profile`

Scope: `user:<author_pubkey_hex>`

Body:

```json
{
  "display_name": "string",
  "bio": "optional string"
}
```

Rules:
- `display_name` is required.
- `bio` is optional.
- No `avatar_url` and no external URL references.
- The scope is deterministic from the author's public key.

### `category`

Scope: `<category_id>`

Body:

```json
{
  "protocol_version": 1,
  "creator_pubkey": "<hex>",
  "category_id": "<hex>",
  "display_name": "string",
  "description": "string",
  "created_at": "<RFC3339>",
  "initial_charter_hash": "<hex>",
  "initial_charter": {
    "text": "string",
    "protocol_version": 1,
    "created_at": "<RFC3339>"
  }
}
```

Rules:
- `category_id` is computed once at creation and is immutable.
- `display_name` is for UI only and MUST NOT be used as identity.
- `category_id` = SHA-256(canonical_json(category_id_input)) where:
  ```json
  {
    "protocol_version": 1,
    "creator_pubkey": "<hex>",
    "display_name": "<exact display name>",
    "created_at": "<RFC3339>",
    "initial_charter_hash": "<hex>"
  }
  ```
- `initial_charter_hash` = SHA-256(canonical_json(initial_charter_body)).
- Phase 1 uses a minimal charter anchor. Full governance (`category_charter`, amendments, votes) is Phase 3.
- Changing `display_name` or charter text later does NOT create a new `category_id`; such changes must be expressed as new objects that still reference the same `category_id`.

### `post`

Scope: `<category_id>`

Body:

```json
{
  "category_id": "<hex>",
  "text": "string",
  "created_at": "<RFC3339>"
}
```

Rules:
- `text` is plain text.
- Posts are immutable in Phase 1; there is no `edit_revision`.
- No `media_ref` in Phase 1.
- No external URL preview generation.

### `comment`

Scope: `<category_id>`

Body:

```json
{
  "category_id": "<hex>",
  "parent_kind": "post" | "comment",
  "parent_id": "<object_id hex>",
  "text": "string",
  "created_at": "<RFC3339>"
}
```

Rules:
- `parent_kind` indicates whether the comment replies to a `post` or another `comment`.
- `parent_id` is the object ID of the parent.
- Comments are immutable in Phase 1.

### `revocation_certificate`

Scope: `revocation:<revoked_pubkey_hex>`

Body:

```json
{
  "revoked_pubkey": "<hex>",
  "replacement_pubkey": "<hex> | null",
  "effective_at": "<RFC3339>",
  "reason_code": "string"
}
```

Rules:
- Revocation is prospective, not retroactive.
- Objects whose `author_pubkey` equals `revoked_pubkey` and whose `created_at >= effective_at` MAY be rejected by validators.
- Objects created before `effective_at` are preserved.
- Full key-rotation UX is Phase 2+, but the core validation helper and tests exist in Phase 1.

## Deterministic identity

The same logical object MUST produce the same `object_id` on every implementation. This requires:

1. Canonical JSON serialization (sorted keys, no whitespace, RFC 3339 timestamps, base64url binary).
2. The exact same `type`, `protocol_version`, `schema_version`, `created_at`, `author_pubkey`, `scope`, and `body` values.
3. SHA-256 over the canonical signed payload bytes.

Any deviation in canonicalization, field order, or encoding forks object identity.

## Not in Phase 1

- `category_charter`, `charter_amendment`, `vote`
- `media_ref`, `media_node_manifest`
- `report`, `report_bundle`, `moderation_action`
- `category_state`, `category_bundle`, `tombstone`
- Edit revisions for posts/comments
- External URL previews
