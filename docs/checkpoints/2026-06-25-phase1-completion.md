# AgoraMesh Phase 1 Completion Checkpoint — 2026-06-25

## Status

Phase 1 is complete.

This checkpoint records the completion of the minimal P2P text prototype.

## Scope

Phase 1 covers:
- canonical message signing and verification
- verified local SQLite storage
- Phase 1 typed objects
- clap-only CLI
- manual peer configuration
- provisional localhost HTTP/JSON direct sync
- E2E validation for 2-peer and 3-peer sync

## Completed features

- Ed25519 identity and signing
- object_id = SHA-256(canonical signing_payload)
- RFC3339 timestamps
- encrypted keyring by default
- user_profile object
- category object
- post object
- comment object
- revocation_certificate object
- type-specific Phase 1 validation
- InMemoryStore
- SqliteStore
- verify-before-save
- verify-on-read
- deterministic ordering
- SQLite row metadata consistency check
- clap CLI commands:
  - key generate/show
  - category create
  - post create
  - comment create
  - feed
  - peer add/list
  - sync
  - run
- local/manual peer direct sync
- no default public peers
- no official server/relay/bootstrap node
- public bind guard with --allow-public-bind

## Phase 1 completion criteria

Mark each as completed:

- [x] Two CLI peers can exchange posts/comments in the same category.
- [x] Three or more peers can converge on the same object set.
- [x] Invalid signatures are rejected.
- [x] The same object produces the same hash.
- [x] Local state is restored after restart.
- [x] Duplicate object propagation is idempotent.
- [x] Future created_at policy is enforced.
- [x] Missing objects are synchronized after disconnect/reconnect.
- [x] Default configuration contains no remote public peers.

## Verification

Record the latest known passing checks:

- cargo fmt --check
- cargo check --workspace --all-targets
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace --all-targets
- ./dev ci
- GitHub Actions CI

## Deliberately excluded from Phase 1

- TUI/ratatui
- media_ref
- media-node
- moderation/reporting/admin governance
- category charter governance
- DM
- search
- web gateway
- recommendation
- token/coin
- official server
- official relay
- official bootstrap node
- default public peer list
- official category list

## Next phase

Phase 2: minimal client UI/TUI.

Phase 2 should focus on:
- feed view
- post creation flow
- comment/thread view
- category subscription status
- peer/sync status
- key management UX
- first-seen category/node warning draft

Do not start Phase 3 governance or Phase 4 media work before Phase 2 is scoped.
