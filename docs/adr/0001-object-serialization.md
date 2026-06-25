# ADR 0001: Object Serialization and Hashing

## Status

Accepted

## Context

AgoraMesh protocol objects need a canonical serialization format so that:

- the same logical object produces the same `object_id` across every implementation,
- signatures can be verified against a deterministic byte payload,
- test fixtures and cross-implementation conformance vectors are easy to read and compare.

The roadmap originally left the canonical format open (Canonical JSON or CBOR) and deferred the decision to ADR 0001.

## Decision

Phase 1 will use **Canonical JSON** for all signed and hashed protocol data.

Specifically:

- `signing_payload` is serialized as Canonical JSON before signing.
- `object_id` is `SHA-256(canonical_json_bytes(signing_payload))`.
- `category_id` preimage is also Canonical JSON + SHA-256, as required by `docs/specs/category-id.md`.
- Transport may later use a more compact encoding, but the signed/hash canonical form remains Canonical JSON.

### Canonical JSON rules

1. UTF-8 output only.
2. Object keys are sorted lexicographically by UTF-8 byte value.
3. No insignificant whitespace.
4. No Unicode escapes for printable characters.
5. Numbers are integers only in signed/hash payloads; floating-point values are forbidden.
6. Timestamps are RFC 3339 strings, not numeric epochs.
7. Binary data is encoded as base64url strings inside JSON values.
8. No trailing commas, no optional formats, no locale rules.

### Hash function

Phase 1 uses SHA-256 for all protocol hashing:

- `object_id`
- `category_id`
- internal references/hashes

BLAKE3 may be reconsidered later for transport or non-normative local indexes, but it will not change the canonical `object_id` format.

## Consequences

- Implementations only need a JSON serializer and SHA-256 to verify object identity.
- Debugging and conformance test vectors are human-readable.
- We avoid the cross-language CBOR profile complexity in the first release.
- Payloads are larger than CBOR, but Phase 1 is text-only and this is acceptable.

## Notes

- The serialization rules above are intentionally strict. Any deviation produces a different `object_id`, so the spec must be followed exactly.
- A reference `canonical_json` helper will live in `crates/agoramesh-core` and be pinned by golden test vectors.

## References

- `docs/specs/category-id.md`
- `docs/adr/0006-envelope-signatures.md`
