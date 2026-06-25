# report_bundle Spec

## Status

`report_bundle` is a V1.0 admin UI view, not a protocol object.

- Source of truth: `report`
- Local derivation: `report_bundle`
- Network scope: none
- Gossip: no
- Signature scope: no direct signature, no standalone network hash

`report_bundle` exists so an admin can review related reports as one deterministic card in the local dashboard. It is regenerated from local `report` state whenever the admin UI or moderation workflow needs it.

## Definitions

- `report`: the original signed protocol object created by a reporter.
- `report_bundle`: a deterministic local aggregation of matching `report` objects.
- `bundle_view_hash`: the stable local hash for a grouped bundle view.
- `target_kind`: one of `post`, `comment`, or `media_ref`.
- `target_hash`: the canonical hash of the reported target object.
- `reporter_pubkey`: the public key of the reporter who signed the `report`.
- `media_node_key`: the media node that serves or owns the reported media, or `null` for text only targets.
- `reason_code`: the report reason enum.
- `time_bucket`: the canonical UTC bucket used for grouping.

## Grouping Algorithm

`report_bundle` is produced by grouping local `report` objects that match all of the following fields:

1. Same `target_kind`
2. Same `target_hash`
3. Same `reporter_pubkey`
4. Same `media_node_key`
5. Same `reason_code`
6. Same `time_bucket`

### Canonical time bucket

- Use UTC only.
- Bucket width is fixed at 24 hours.
- The bucket key is `floor(report.created_at / 86400 seconds)`.
- A report belongs only to the bucket that contains its `created_at` timestamp.

### Canonical bundle contents

For each matching group, the local view stores:

- The grouping key fields above
- The sorted list of underlying `report` hashes
- The earliest `created_at` in the group
- The latest `created_at` in the group
- The local `bundle_view_hash`

### Sort order inside the bundle

Reports inside a bundle are sorted by:

1. `created_at` ascending
2. `report_hash` ascending

If two reports are identical across these fields, their canonical hashes still keep the order stable.

## Admin UI Mapping

The admin dashboard shows one card per `report_bundle`.

Each card displays:

- Target summary, such as post, comment, or media reference
- Reason code
- Report count
- Time bucket range
- Evidence status
- `bundle_view_hash` for local audit comparison only

Available actions:

- Open the bundle and inspect each underlying `report`
- View decrypted evidence, when the admin key can decrypt it
- Issue a `moderation_action`
- Issue a `tombstone`
- Dismiss the bundle as reviewed, which is only a local UI state and does not create a protocol object

The UI must not treat `report_bundle` as something that can be shared, imported, or gossiped as a protocol object.

## Moderation Action References

`moderation_action` and `tombstone` reference the underlying `report` hashes directly.

Required rule:

- Store `related_report_hashes`
- Sort `related_report_hashes` lexicographically
- Do not store `report_bundle` IDs in signatures
- Do not store `report_bundle` IDs in `audit_log`
- Do not store `bundle_view_hash` in signatures
- Do not store `bundle_view_hash` in `audit_log`

The signed payload for moderation must be built from the action data and the sorted `report` hashes, not from a bundle identifier.

Example reference shape:

```text
moderation_action.related_report_hashes = [report_hash_1, report_hash_2, report_hash_3]
```

## Determinism

Two admins who review the same local `report` set must derive the same `report_bundle` membership and the same `bundle_view_hash`.

Determinism rules:

- Use only canonical report fields for grouping
- Use UTC 24 hour buckets
- Use stable sorting for bundle membership
- Exclude local ingest order
- Exclude UI order
- Exclude database row ids
- Exclude admin identity
- Exclude wall clock time at view generation

The hash input is the canonical bundle view, not a live UI snapshot. If the same reports are present, the same grouping key produces the same `bundle_view_hash`.

## Privacy

`report_bundle` must not reveal reporter identity beyond what the encrypted evidence already permits.

- The bundle card may show aggregate evidence status, but it must not expose reporter identity by default.
- Reporter identity is shown only when the admin can already decrypt the related evidence package.
- `report_bundle` does not add new identity fields.
- Aggregated counts are allowed.
- The bundle view must not leak reporter metadata through bundle hashes, ordering, or summary labels.

## Tests

Conformance tests for `report_bundle` must verify:

1. Same target, same reporter key, same media node, same reason code, and same UTC bucket produce one bundle.
2. Changing any one grouping field produces a different bundle.
3. Two admins with the same local reports derive the same `bundle_view_hash`.
4. Bundle membership is stable when ingest order changes.
5. `moderation_action.related_report_hashes` contains report hashes only, not `report_bundle` IDs.
6. `audit_log` entries do not include `report_bundle` IDs.
7. `report_bundle` regeneration from local state is idempotent.
8. The bundle view does not expose reporter identity unless the evidence payload already allows it.
