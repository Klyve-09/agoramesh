# Agoramesh

A decentralized mesh-messaging network built in Rust.

## Crates

- `crates/agoramesh-core` – shared primitives: identities, keys, messages, signing/verification.
- `crates/agoramesh-store` – persistent SQLite-backed storage with verified read paths.
- `crates/agoramesh-net` – provisional localhost HTTP/JSON direct sync for Phase 1; QUIC/libp2p/gossipsub are deferred.
- `crates/agoramesh-cli` – command-line entry point.
- `crates/agoramesh-tui` – minimal ratatui terminal client (Phase 2).

## Phase 1 boundary

This milestone implements a minimal P2P text prototype. The Phase 1 completion checkpoint is recorded at:

- docs/checkpoints/2026-06-25-phase1-completion.md

Scope:

- Canonical message signing/verification (ADR 0001, ADR 0006).
- Verified local SQLite storage.
- Phase 1 typed objects: `user_profile`, `category`, `post`, `comment`, `revocation_certificate`.
- clap-only CLI: `key`, `category`, `post`, `comment`, `feed`, `peer`, `sync`, `run`.
- Provisional direct sync over localhost HTTP/JSON between manually configured peers.

## Phase 2 boundary

This milestone adds a minimal terminal UI client. The Phase 2 plan and completion checkpoints are recorded at:

- docs/checkpoints/2026-06-26-phase2-tui-plan.md
- docs/checkpoints/2026-06-27-phase2-completion.md

Scope:

- ratatui-based feed view with subscribed categories and posts.
- Compose flow with category selection, preview, and signed post submission.
- Thread/comment view with nested replies and collapse support.
- Subscription, peer/sync status, key management, and first-seen warning panels.
- Persistence for subscriptions and acknowledged first-seen values.

Run the TUI:

```bash
cargo run --bin agoramesh-tui -- --data-dir /tmp/agoramesh-tui-data
```

For CI or disposable local demos only, plaintext key storage requires the explicit flag:

```bash
cargo run --bin agoramesh-tui -- --data-dir /tmp/agoramesh-tui-data --dev-insecure-plaintext-key
```

Keys:

- `1` Feed, `2` Subscriptions, `3` Sync status, `4` Key management
- `n` New post (compose)
- `Tab` in Feed switches movement focus between categories and posts
- `↑`/`k`, `↓`/`j` move selection
- `Enter` opens the selected feed post, toggles the selected thread comment, or unlocks/submits on the active screen
- `a` acknowledges the current first-seen warning outside Key Management; `Ctrl+a` does so in Key Management
- `Tab` toggles compose editor/preview
- `Enter` inserts a newline in the compose editor and submits only from preview
- `Backspace` delete last character when in Compose
- `Esc` back
- Key Management: type passphrase, `Ctrl+g` generate encrypted key, `Enter` unlock, `Ctrl+b` backup, `Ctrl+r` restore, `Ctrl+d` generate dev plaintext key only with `--dev-insecure-plaintext-key`
- `Space`/`Enter` toggle subscription in Subscriptions; `s` toggles the selected subscribed category in Feed
- `Ctrl+q` quit

Phase 2 deliberately does **not** include:

- Background realtime sync, peer discovery, default peers, or official infrastructure.
- Moderation, reporting, admin actions, or charter governance.
- `media_ref`, media nodes, or external URL previews.
- Search, web gateway, recommendation, or token economy.

All remote peers must be added manually. The default peer list is empty.

The CLI `run` command binds to `127.0.0.1:0` by default. The TUI does not start a sync server or bind a public address.

## Development

```bash
./dev check   # cargo check
./dev test    # cargo test
./dev lint    # rustfmt + clippy
./dev fmt     # format
./dev ci      # full CI-quality check
```

## License

Apache-2.0 OR MIT
