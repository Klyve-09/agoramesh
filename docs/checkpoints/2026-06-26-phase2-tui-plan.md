# AgoraMesh Phase 2 Minimal TUI Client Checkpoint — 2026-06-26

## Status

Phase 2 minimal TUI client implementation is complete and verified.

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
- Binary target `agoramesh-tui` with `--data-dir`, `--plaintext`, and `--allow-public-bind` flags
- Crossterm + ratatui event loop and terminal lifecycle
- Central `AppState` reducer with `Action` dispatch
- Backend gateway over `agoramesh-store`/`agoramesh-cli`/`agoramesh-core`
- Feed screen reading posts from a per-category cache populated on startup
- Compose screen with live preview and signed post submission
- Thread screen rendering root post and nested comment tree
- Subscription persistence in `subscriptions.json`
- First-seen acknowledgement persistence in `seen.json`
- First-seen warnings computed from categories and peers
- Key status panel with dev plaintext key generation helper
- Sync status panel showing last sync totals
- 20 unit tests + 7 integration tests in the TUI crate

## Bug fix included

`crates/agoramesh-store/src/db.rs`: `created_at` metadata was written with the `+00:00` offset instead of the canonical `Z` suffix, causing metadata verification to fail on read. The fix truncates timestamps to seconds and uses `to_rfc3339_opts(SecondsFormat::Secs, true)` so SQLite metadata matches the signed payload.

## Verification

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace --all-targets`

## Remaining before merge

- User review and approval
- Atomic commit with lore-format message
- Push `phase-2/minimal-tui-client` branch
- Open a draft PR against `Klyve-09/agoramesh:main`

## Deliberately excluded from Phase 2

- Real-time backend polling / background sync in the TUI event loop
- Network peer discovery or automatic peer configuration
- Moderation, reporting, admin actions, or charter governance
- Media support, web gateway, search, or token economy
- Official server, relay, bootstrap node, or default public peers
