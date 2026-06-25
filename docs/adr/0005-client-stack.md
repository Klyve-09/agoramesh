# ADR 0005: Client Stack

## Status

Accepted

## Context

Phase 1 needs a command-line client for AgoraMesh. The client must let a user:

- generate and manage identities,
- create categories and first charters,
- write posts and comments,
- subscribe to categories and sync objects from peers,
- run a local peer/node.

The roadmap places a minimal UI/TUI in Phase 2, so Phase 1 should stay focused on protocol correctness rather than terminal UI complexity.

## Decision

Phase 1 client is a **clap subcommand CLI only**. No TUI library is included.

Planned top-level subcommands for Phase 1:

- `key` — generate, show, backup, restore identities.
- `category` — create category and first charter.
- `post` — create a post in a category.
- `comment` — create a comment on a post or comment.
- `peer` — manage manual peer addresses.
- `sync` — trigger sync with known peers.
- `run` — run a local peer/node.

All human-readable output supports a `--json` flag so that future TUI/GUI clients can reuse the CLI as a backend if desired.

## Identity and Key Storage

By default, secret keys are stored encrypted at rest.

- Key derivation: Argon2id.
- Encryption: XChaCha20-Poly1305 or an equivalent authenticated encryption construction from a well-reviewed Rust crate.
- `key generate` prompts for a passphrase.
- `key show` prints the public key by default.
- Secret key export requires an explicit `--show-secret` flag and prints a strong warning.
- For CI, development, and automated tests only, an explicit `--dev-insecure-plaintext-key` flag permits plaintext key storage. This flag is never the default and is named to discourage production use.

## Consequences

- Phase 1 implementation effort stays focused on core protocol, storage, and P2P correctness.
- Key security defaults are set correctly from the beginning, avoiding a later migration.
- The CLI is scriptable via `--json`, making integration and E2E tests simpler.
- TUI work is intentionally deferred to Phase 2.

## References

- `docs/v1.0-roadmap.md` Phase 1 and Phase 2 sections
- `docs/adr/0006-envelope-signatures.md`

## Future Work

- Phase 2 may add a `ratatui`-based TUI that calls the same core APIs.
- Mobile/desktop GUI clients can reuse the `--json` CLI surface or the library crate directly.
