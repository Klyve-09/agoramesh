# ADR 0003: Network Transport (Provisional Direct Sync)

## Status

Provisional

## Context

Agoramesh needs a way for peers to exchange verified objects. The long-term target is a decentralized mesh using QUIC and libp2p gossipsub, but that stack is too large to block Phase 1. Phase 1 must still prove end-to-end P2P text synchronization between manually configured peers.

This ADR records the provisional transport choice for Phase 1 and the criteria for replacing it.

## Decision

Phase 1 uses **manual peer address + localhost HTTP/JSON direct sync**.

### Why HTTP/JSON direct sync

- Minimal implementation surface: four endpoints are enough for object pull/push.
- Easy to test and debug with common tools (`curl`, `reqwest`, `axum`).
- No external bootstrap infrastructure required.
- Keeps the codebase free of libp2p/QUIC complexity while core signing, storage, and object semantics stabilize.

### Endpoint summary

See `docs/specs/message-exchange.md` for the full contract.

- `GET /health` — liveness check.
- `GET /objects?scope=<category_id>` — list objects in scope.
- `GET /objects/<object_id_hex>` — fetch one object.
- `POST /objects` — submit a verified object.

### Scope and security

- Default bind address is `127.0.0.1` or an ephemeral localhost port.
- Binding to a public address requires an explicit flag.
- There are no default public peers, official relays, or bootstrap nodes.
- TLS is not used in Phase 1 because traffic is localhost-only by default; a production deployment must add TLS before exposing the sync port publicly.

### Relationship to gossipsub

ADR 0008 specifies the gossipsub topic format `agoramesh/v0/<category_id>/objects` for future use. Phase 1 does not open gossipsub subscriptions or publish to those topics; the topic helper exists only to keep topic naming consistent across implementations.

## Consequences

- Phase 1 can deliver working two-peer and three-peer synchronization quickly.
- Tests and debugging are simpler than with a full P2P stack.
- The protocol is not optimized for large meshes or NAT traversal.
- Future phases will replace direct sync with QUIC + libp2p gossipsub, at which point this ADR will be superseded or updated.

## Future work

- Implement QUIC endpoint binding and certificate management.
- Implement libp2p gossipsub topic publishing/subscription.
- Implement NAT traversal and peer discovery (optional).
- Replace or augment direct sync with gossipsub once the mesh stack is ready.

## References

- `docs/specs/message-exchange.md`
- `docs/adr/0008-gossip-topic-and-timestamp-policy.md`
- `docs/adr/0001-object-serialization.md`
- `docs/adr/0006-envelope-signatures.md`
