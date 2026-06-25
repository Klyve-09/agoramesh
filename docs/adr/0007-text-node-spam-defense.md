# ADR 0007, Text Node Spam Defense

## Status
Accepted

## Context
AgoraMesh treats text nodes as independently operated components. The roadmap already frames `text_node_manifest` as the node policy declaration and `node_policy` as the category-level policy surface for text nodes. That means spam defense belongs to the node operator and the client, not to the protocol as a global guarantee.

This distinction matters for three reasons.

1. AgoraMesh has no official or central text infrastructure, so there is no protocol authority that can enforce one universal spam posture.
2. Different nodes run under different legal, operational, and moderation constraints, so a single fixed spam policy would be both unrealistic and brittle.
3. Category charters define community rules. Node policy can narrow how a node serves or propagates content, but it must not override charter decisions about what the category allows.

The roadmap requires the following baseline defenses for text nodes.

1. Per-key posting rate limit.
2. Per-category burst limit.
3. New-key posting restriction.
4. Duplicate-content detection.
5. Temporary propagation restriction when mass reports accumulate.

## Decision
AgoraMesh will define text-node spam defense as a local, node-level policy controlled by `node_policy` and advertised through `text_node_manifest`.

### 1. Scope boundary
Spam defense is not a protocol invariant. The protocol only defines the data objects, manifest fields, and client behavior needed for nodes to advertise and enforce their own local policy.

Nodes may choose stricter local limits than another node. They may not use `node_policy` to negate category charter permissions, invent new category rules, or claim a global block for the network.

### 2. `node_policy` fields
`node_policy` is the policy payload a category uses to describe the text-node behavior it expects or accepts.

Required fields.

1. `per_key_posting_rate_limit`, the maximum number of posts a key may submit per time unit.
2. `per_category_burst_limit`, the maximum short-window posting burst a category may accept from a key or key group.
3. `new_key_grace_period`, the time window after key creation during which the node applies stricter treatment.
4. `duplicate_content_window`, the time window used to compare content hashes for repetition.
5. `mass_report_threshold`, the report count that triggers temporary propagation restriction.

`node_policy` may include other operator settings, but these five fields are the spam-defense contract required by the roadmap.

### 3. Per-key posting rate limit
Each node tracks posting activity per signing key. If a key exceeds `per_key_posting_rate_limit`, the node rejects new posts or queues them according to its local implementation.

This limit is local to the node. It protects that node from being used as a spam amplifier, but it does not say the post is invalid everywhere else.

### 4. Per-category burst limit
Each node also tracks burst behavior within a category. This prevents a key, or a small set of related keys, from flooding one category with a short spike of posts even if the long-term rate stays under the per-key limit.

The burst window is short and explicit. The node should treat this as a local load and abuse control, not as a category-wide punishment.

### 5. New-key posting restriction
Fresh keys are treated cautiously during `new_key_grace_period`.

The node may do either of the following.

1. Apply a tighter posting limit during the grace period.
2. Reduce propagation priority for posts from the new key.

The grace period is meant to slow drive-by spam and key rotation abuse. It is not a ban on new members and not a category membership rule.

### 6. Duplicate-content detection
The node computes a canonical hash of the post body and compares it against recent posts within `duplicate_content_window`.

If the same canonical body hash repeats too often within that window, the node can down-rank, delay, or reject the content locally. The intent is to catch copy-paste floods and near-repeat spam without inventing a global uniqueness rule.

Canonicalization must be stable enough that the same semantic body produces the same hash under the node’s agreed serialization rules.

### 7. Mass-report propagation restriction
When reports for a post or key reach `mass_report_threshold`, the node may place that content into a temporary propagation pause.

This is a local safety action only.

1. It is not automatic deletion.
2. It is not a global block.
3. It does not change the category charter.
4. It only stops or slows further local propagation while moderators or operators review the case.

If the reports later prove unfounded, the node can lift the pause without any protocol-wide correction.

### 8. `text_node_manifest` and client behavior
Each text node publishes a `text_node_manifest` that declares its policy surface, including the `node_policy` values it applies.

Clients use the manifest to decide how to interact with that node.

1. They display the node’s spam-defense posture before trusting it.
2. They respect stricter posting and propagation limits when they submit content to that node.
3. They avoid assuming that one trusted node’s policy applies to every other trusted node.

### 9. When trusted nodes differ
Clients may trust more than one node, and those nodes may advertise different policy values.

In that case, the client treats each node separately.

1. The strictest policy on a given node applies on that node.
2. A stricter node does not weaken a looser node.
3. A looser node does not override a stricter node.
4. The client should surface the difference so users understand that moderation and propagation behavior varies by node.

The client must not collapse these differences into a fake protocol-wide rule.

### 10. Conformance tests and abuse simulations
The implementation set must include conformance tests for the local policy contract and abuse simulations for the main failure modes.

Required tests.

1. Per-key rate limit test, verify a key is constrained after repeated posting.
2. Per-category burst test, verify a short spike is controlled even when the long-term rate passes.
3. New-key grace test, verify fresh keys get stricter treatment or lower propagation priority.
4. Duplicate-content test, verify repeated canonical body hashes are detected inside the configured window.
5. Mass-report test, verify a threshold triggers local propagation pause, not deletion.
6. Manifest interoperability test, verify clients read `text_node_manifest` and apply node-specific policy.
7. Multi-node trust test, verify a client handles two trusted nodes with different policy values without treating them as one policy.

Required abuse simulations.

1. Bot account spam with one key.
2. Burst spam across one category.
3. New-key churn abuse.
4. Repost flooding with identical canonical bodies.
5. Report brigading that triggers a temporary local pause.

## Consequences
This design keeps AgoraMesh honest about where control lives.

1. Nodes stay free to protect themselves and their communities.
2. Clients must understand node-specific policy and cannot assume a universal moderation posture.
3. The protocol stays smaller, because it defines policy exchange and local enforcement inputs instead of a global spam authority.
4. Operators retain room to tune limits to their own risk tolerance.

The tradeoff is inconsistency across nodes. A post may be accepted on one trusted node and slowed or rejected on another. That is expected, and clients must present it clearly.

## Security Notes
Spam defense here is a local safety control, not a rights system and not a global moderation verdict.

1. `node_policy` cannot override category charter permissions.
2. Temporary propagation pause is not deletion.
3. Mass reports only affect the node that received them unless another node independently reaches the same conclusion.
4. Duplicate-content detection should operate on canonical body hashes, not raw transport text, so equivalent bodies are treated consistently.
5. Clients should warn when trusted nodes advertise different spam policies, because behavior can diverge in ways users need to see.

## References
1. `docs/v1.0-roadmap.md`, Phase 8 text-node spam defense and Phase 6 manifest fields.
2. `docs/v1.0-roadmap.md`, `text_node_manifest` and `node_policy` object list.
3. `docs/v1.0-roadmap.md`, protocol conformance test and abuse simulation requirements.
