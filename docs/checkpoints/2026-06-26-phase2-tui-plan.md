# AgoraMesh Phase 2 Minimal TUI Client Checkpoint — 2026-06-26

## Status

Phase 2 minimal TUI client implementation is in progress.

## Scope

Phase 2 covers a minimal terminal UI for the AgoraMesh text prototype:

- Feed view with subscribed categories and posts for the selected category
- Post creation / compose flow with category selection and preview
- Thread / comment view with collapse support
- Category subscription status panel
- Peer and sync status panel
- Key management UX
- First-seen category and peer warnings with acknowledgement persistence

## Completed features

- New workspace crate `crates/agoramesh-tui`
- Binary target `agoramesh-tui` with `--data-dir` and `--dev-insecure-plaintext-key` flags
- Crossterm + ratatui event loop with Drop-based terminal restoration guard
- Central `AppState` reducer plus backend-backed controller/effect dispatch
- Backend gateway over `agoramesh-store`/`agoramesh-cli`/`agoramesh-core`
- Feed screen reading subscribed category posts from a per-category cache populated on startup
- Compose screen with Unicode text input, multiline editor, preview, and signed post submission
- Thread screen rendering root post and nested comment tree
- Subscription management over all known categories with persistence in `subscriptions.json`
- First-seen acknowledgement persistence in `seen.json`
- First-seen warnings computed from categories and peers
- Key status panel with encrypted key generation/unlock plus backup/restore; dev plaintext is explicit only
- Sync status panel showing manually configured peers and making the absence of background sync explicit
- Event/controller/backend/persistence integration coverage for compose, subscriptions, warnings, keys, and thread loading

## Bug fix included

`crates/agoramesh-store/src/db.rs`: new writes use a canonical `Z` suffix for SQLite `created_at` metadata, while old Phase 1 rows with `+00:00` remain readable through RFC3339 parsed-instant comparison. The migration path does not rewrite signed payloads or object IDs.

## Verification

- In progress; latest checks must be recorded in the PR before review.

## Remaining before merge

- Full verification: `cargo fmt --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --all-targets`, `./dev ci`
- Manual TUI UX pass by a user before declaring Phase 2 complete
- Any feedback-driven protocol/UI corrections from that UX pass

## Deliberately excluded from Phase 2

- Real-time backend polling / background sync in the TUI event loop
- Network peer discovery or automatic peer configuration
- Moderation, reporting, admin actions, or charter governance
- Media support, web gateway, search, or token economy
- Official server, relay, bootstrap node, or default public peers
