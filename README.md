# Agoramesh

A decentralized mesh-messaging network built in Rust.

## Crates

- `crates/agoramesh-core` – shared primitives: identities, keys, messages.
- `crates/agoramesh-store` – persistent SQLite-backed storage.
- `crates/agoramesh-net` – QUIC-based peer-to-peer transport.
- `crates/agoramesh-cli` – command-line entry point.

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
