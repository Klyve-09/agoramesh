# Agoramesh

A decentralized mesh-messaging network built in Rust.

## Crates

- `crates/agoramesh-core` – shared primitives: identities, keys, messages, signing/verification.
- `crates/agoramesh-store` – persistent SQLite-backed storage with verified read paths.
- `crates/agoramesh-net` – network transport crate (intentionally empty in Phase 1; QUIC/libp2p/gossipsub will land here in a later phase).
- `crates/agoramesh-cli` – command-line entry point.

## Phase 1 boundary

This milestone implements canonical message signing/verification and verified local storage. It deliberately does **not** include QUIC endpoint binding, libp2p/gossipsub propagation, a ratatui TUI, moderation/reporting, or media features.

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
