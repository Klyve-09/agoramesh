# AgoraMesh Message Exchange — Phase 1

## Status

Phase 1 uses a **provisional direct sync** protocol over localhost HTTP/JSON. QUIC, libp2p, and gossipsub propagation are intentionally deferred to later phases. The gossipsub topic format is specified in ADR 0008 for forward compatibility, but Phase 1 implementations exchange objects through the direct sync endpoints below.

## Principles

- No official server, relay, bootstrap node, or default public peer.
- Peer addresses are configured manually by the user.
- All received objects are verified before storage.
- Invalid or untrusted objects are rejected and not stored in the accepted object store.
- Duplicate `object_id` values are stored only once.
- Future-dated objects follow ADR 0008:
  - `created_at > now + 5 minutes` → reject, do not store or propagate.
  - `now < created_at <= now + 5 minutes` → accept with warning.
  - `created_at <= now` → accept normally.
- `receive_time` is never part of the signed payload or object ID.

## Gossipsub topic format (specified, not used in Phase 1)

For future gossipsub compatibility, the topic for a category is:

```text
agoramesh/v0/<category_id>/objects
```

- `v0` is a protocol compatibility namespace, not a full semver.
- `category_id` is the lowercase hex stable identifier.
- `display_name` is never included in the topic.
- Tests may override the prefix (e.g. `agoramesh-dev-<run_id>`) to avoid colliding with any main network.

## Direct sync endpoints

A Phase 1 peer exposes a small HTTP/JSON API for direct object exchange. All endpoints are localhost-only by default; binding to a public address requires an explicit flag.

### `GET /health`

Returns plain text `ok` when the peer is reachable.

### `GET /objects?scope=<category_id>`

Returns all objects in the given scope, sorted by `(created_at, object_id)` ascending.

Response: `200 OK`
```json
[
  { /* Message JSON */ },
  ...
]
```

The response body is a JSON array of `Message` objects.

### `GET /objects/<object_id_hex>`

Returns a single object by its lowercase hex object ID.

Response: `200 OK` with the `Message` JSON, or `404 Not Found` if the object is absent.

### `POST /objects`

Accepts a `Message` JSON body. The receiver:

1. Deserializes the message.
2. Verifies the signature, object ID, author consistency, and clock skew.
3. Inserts the object into the accepted store if verification succeeds and the object ID is not already present.
4. Rejects the object (returning `422 Unprocessable Entity`) if verification fails, clock skew is too large, or the object is otherwise invalid.
5. Returns `409 Conflict` if the object ID already exists.
6. Returns `201 Created` on successful insertion.

Invalid objects are not stored in the accepted store and are not propagated further.

## Sync procedure

`agoramesh sync` iterates over the manually configured peer list and, for a given category:

1. Pulls remote objects with `GET /objects?scope=<category_id>`.
2. Attempts to insert each pulled object into the local store.
3. Optionally pushes local objects to the peer with `POST /objects`.

Because the store deduplicates by object ID, re-running `sync` is idempotent.

## Not in Phase 1

- Real-time gossipsub propagation
- QUIC transport
- libp2p peer discovery
- DHT-based object lookup
- Official relays, bootstrap nodes, or seed lists
- NAT traversal / hole punching
