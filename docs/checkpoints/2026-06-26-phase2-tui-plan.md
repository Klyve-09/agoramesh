# AgoraMesh Phase 2 Minimal TUI Client Checkpoint — 2026-06-26

## Status

PR #6 brings the minimal TUI client implementation to its current review state, including the follow-up module split/refactor and restored thread-loader invariants. Phase 2 itself is not complete yet: the required user UX review is still pending before Phase 2 can be declared complete.

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
- Key status panel with encrypted key generation/unlock plus nonfatal backup/restore status handling; dev plaintext is explicit only
- Sync status panel showing manually configured peers and making the absence of background sync explicit
- Event/controller/backend/persistence integration coverage for compose, subscriptions, warnings, keys, and thread loading
- PR #6 module split/refactor:
  - `keyring.rs` split into `crypto.rs`, `files.rs`, and `schema.rs`
  - backend split into focused `content`, `file_io`, `key_mgmt`, `local_state`, and `peers` modules
  - controller split into focused compose, key-management, navigation, and dispatcher modules
- Thread loader regression coverage restored after the split:
  - `ParentKind` filtering is enforced so a comment with `parent_kind=Comment` and a post id is not rendered as a top-level post comment
  - malformed comment `parent_id` values are rejected instead of silently ignored
  - non-post thread roots are rejected before root post body decoding

## Bug fix included

`crates/agoramesh-store/src/db.rs`: new writes use a canonical `Z` suffix for SQLite `created_at` metadata, while old Phase 1 rows with `+00:00` remain readable through RFC3339 parsed-instant comparison. The migration path does not rewrite signed payloads or object IDs.

## Verification

- Previous review baseline: 2026-06-26 PR #5 code head `e14ed9a` verified
  the earlier Phase 2 blocker fixes for subscription cache, compose selection,
  key overwrite protection, terminal setup cleanup, nonfatal key backup/restore
  handling, and strict restore validation for locked encrypted backups.
- Current PR #6 review state includes the subsequent module split/refactor,
  restored `ParentKind` filtering, and the non-post thread-root rejection in
  `Backend::load_thread`.
- Final automated checks for the current PR #6 state passed:
  - `cargo fmt --check`
  - `cargo check --workspace --all-targets`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --all-targets`
  - `./dev ci`
- Focused regressions passed:
  - `load_thread_rejects_non_post_root`
  - `thread_ignores_comment_with_post_id_but_comment_parent_kind`
  - `thread_rejects_malformed_comment_parent_id`
  - `subscription_toggle_loads_existing_feed_without_restart`
  - `compose_submit_selects_submitted_category_and_new_post`
  - `encrypted_key_generate_does_not_overwrite_existing_key`
  - `dev_plaintext_key_generate_does_not_overwrite_existing_key`
  - `terminal_setup_cleanup_attempts_all_completed_steps_after_failure`
  - `backup_without_key_sets_status_and_does_not_exit`
  - `restore_without_backup_sets_status_and_does_not_exit`
  - `restore_corrupt_backup_sets_status_and_preserves_existing_key`
  - `restore_structured_invalid_encrypted_backup_without_session_preserves_existing_key`
  - `restore_encrypted_backup_with_bad_ciphertext_without_session_fails_and_preserves_existing_key`
  - `restore_encrypted_backup_with_unauthenticated_ciphertext_without_session_preserves_existing_key`
  - `restore_encrypted_backup_missing_required_fields_without_session_fails`
  - `restore_failed_validation_removes_temp_file`
  - `restore_structured_invalid_dev_plaintext_backup_preserves_existing_key`
  - `backup_write_failure_sets_status_and_does_not_exit`
  - `key_management_help_matches_event_bindings`
- Manual TUI smoke pass in a throwaway data directory confirmed:
  - subscribing to a category with an existing post makes that post visible in
    Feed without restarting;
  - composing in the second subscribed category returns to Feed with the new post
    selected in the posts pane;
  - Ctrl+d on an existing plaintext dev key leaves the public key intact and
    shows “Key overwrite disabled; use backup/restore instead”.
  - Ctrl+b with no encrypted key stays on Key Management and shows a nonfatal
    backup failure message;
  - Ctrl+r with no backup stays on Key Management and shows that the existing
    key was not changed;
  - corrupt or structured-invalid backup restore stays on Key Management,
    preserves the displayed public key, keeps existing `identity.key` bytes, and
    removes the temporary restore file.

## Remaining before Phase 2 completion

- UX review pending: manual TUI UX pass by a user is still required before declaring Phase 2 complete
- Any feedback-driven protocol/UI corrections from that UX pass

## Deliberately excluded from Phase 2

- Real-time backend polling / background sync in the TUI event loop
- Network peer discovery or automatic peer configuration
- Moderation, reporting, admin actions, or charter governance
- Media support, web gateway, search, or token economy
- Official server, relay, bootstrap node, or default public peers
