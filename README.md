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

This milestone adds a minimal terminal UI client. The Phase 2 checkpoint is recorded at:

- docs/checkpoints/2026-06-26-phase2-tui-plan.md

Scope:

- ratatui-based feed view with subscribed categories and posts.
- Compose flow with category selection, preview, and signed post submission.
- Thread/comment view with nested replies and collapse support.
- Subscription, peer/sync status, key management, and first-seen warning panels.
- Persistence for subscriptions and acknowledged first-seen values.

Run the TUI:

```bash
cargo run --bin agoramesh-tui -- --data-dir /tmp/agoramesh-tui-data --plaintext
```

Keys:

- `1` Feed, `2` Subscriptions, `3` Sync status, `4` Key management
- `n` New post (compose)
- `t` Open thread view for the selected category's newest post
- `↑`/`k`, `↓`/`j` move selection
- `Enter` acknowledge first-seen warning, or open selected post/thread
- `Tab` toggle compose preview when in Compose
- `Backspace` delete last character when in Compose
- `Esc` back
- `g` Generate dev plaintext key when in Key management (only with `--plaintext`)
- `s` Toggle subscription for the selected category when in Feed or Subscriptions
- `Ctrl+q` quit

Phase 1 deliberately does **not** include:

- QUIC endpoint binding or libp2p/gossipsub propagation (direct sync only).
- A ratatui TUI.
- Moderation, reporting, admin actions, or charter governance.
- `media_ref`, media nodes, or external URL previews.
- An official server, official relay, official bootstrap node, default public peer list, recommended category list, search, web gateway, or token economy.

All remote peers must be added manually. The default peer list is empty.

The `run` command binds to `127.0.0.1:0` by default. Binding to a non-loopback address requires `--allow-public-bind`; public bind is experimental and not official infrastructure.

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
