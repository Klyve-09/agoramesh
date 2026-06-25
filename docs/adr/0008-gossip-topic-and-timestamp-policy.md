# ADR 0008: Gossipsub Topic Naming and Future Timestamp Policy

## Status

Accepted

## Context

Two operational details were not covered by earlier ADRs and affect Phase 1 implementation directly:

1. How `libp2p` gossipsub topics are named so peers isolate traffic by category without leaking display names.
2. How to handle objects whose `created_at` is in the future.

## Decision

### Gossipsub topic format

Topic strings use a compatibility namespace, not a full semantic version:

```text
agoramesh/v0/<category_id>/objects
```

- `v0` is a protocol compatibility namespace, not a full semver.
- Patch and minor protocol changes do not create new topic namespaces.
- Only breaking wire-format or signature changes trigger a new namespace (`v1`, `v2`, etc.).
- `category_id` is the stable lowercase hex identifier.
- `display_name` is never included in the topic.
- Local or development tests may use a topic prefix override such as `--topic-prefix agoramesh-dev-<run_id>` to avoid interfering with any main network.

### Future `created_at` policy

Validation compares `created_at` to the local system clock at receive time:

| Skew | Action |
|---|---|
| `created_at` > now + 5 minutes | **Reject.** Do not store in the accepted object store, do not gossip. May log to a local rejected/quarantine diagnostic store. |
| now < `created_at` ≤ now + 5 minutes | **Accept with warning.** Store the object and set a diagnostic flag. |
| `created_at` ≤ now | Accept normally. |

Important invariants:

- `receive_time` is never included in `signing_payload` or `object_id`.
- The 5-minute threshold is a local node policy, not a protocol-wide consensus rule.
- Nodes may choose a different threshold, but the default is 5 minutes.

## Consequences

- Category gossip is isolated by stable ID, avoiding name collisions and display-name leakage.
- Future-dated spam and ordering attacks are mitigated without requiring global clock consensus.
- Test cases for timestamp handling are explicit and deterministic.

## References

- `docs/v1.0-roadmap.md` Phase 1 completion conditions
- `docs/specs/category-id.md`
