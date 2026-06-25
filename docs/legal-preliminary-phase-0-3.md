# AgoraMesh Preliminary Legal Note, Phase 0 to Phase 3

## Scope

This note covers the project state from Phase 0 through Phase 3 only. It is a preliminary wording review for the period before media-node work and before any admin dashboard or node operator packaging lands.

AgoraMesh is being built as a decentralized self-governing community protocol. The wording below should be read with Korea as the primary reference point, while still keeping broader jurisdiction awareness in mind.

## Current Features

During Phases 0 to 3, AgoraMesh is limited to these pieces:

1. Phase 0, design freeze for the core principles, threat model, responsibility boundaries, and document structure.
2. Phase 1, a minimal P2P text prototype with signed objects, posts, comments, categories, and basic gossip or direct peer exchange.
3. Phase 2, a minimal client UI or TUI for writing, reading, subscribing, and key handling.
4. Phase 3, the first governance layer, including category charters, elected admins, reports, hiding, tombstones, appeals, and public moderation logs with private evidence kept separate.

These phases are enough to test protocol behavior and user flow. They are not enough to imply a finished public service.

## Maintainer Position

For this phase range, the maintainer is a protocol and client developer only.

The maintainer does not operate an official server, relay, search service, gateway, category list, or media node. The maintainer also does not run a default public infrastructure layer for the project. Any public node in this stage, if one exists at all, should be treated as a third-party test environment, not as an official AgoraMesh service.

## Risk Areas

### 1. Operator-like appearance from a public test node

Even a temporary node can look like an official service if it is public, always on, or presented as the default place to connect. That can create confusion about who is responsible for availability, moderation, and content handling.

### 2. Default settings that look like curation or endorsement

If the client ships with a default peer list, pre-populated categories, hidden ranking choices, or any other opinionated starting state, users may read that as project endorsement or editorial selection. That is a legal and reputational risk, even if the code only meant to help onboarding.

### 3. Illegal user content in a P2P prototype

Once the prototype can move user generated text between peers, the system can carry illegal or harmful content even if the maintainer never hosts it. The early design should make clear that the protocol transports user content and that moderation responsibility lives with the local category governance and the people running their own nodes, not with an assumed central operator.

## Recommended Wording

The project should use plain, narrow language in the README and related notes.

Suggested wording:

> AgoraMesh is a protocol and client project. It does not include an official server, relay, search service, gateway, category list, or media node.

> The maintainer does not operate a permanent public network node for AgoraMesh. Any test node is temporary, experimental, and not an official service.

> The prototype may carry user generated content between peers. Each category and each local node is responsible for its own moderation and governance decisions.

> Default client settings are for testing only. They are not an endorsement, ranking, or recommendation system.

> Reports may be collected for protocol testing, but they are not a centralized judgment system.

## Prototype Testing Rules

For public testing during Phases 0 to 3:

1. Do not operate a permanent public node.
2. Do not publish a default peer list.
3. Do not pre-populate categories.
4. Do collect reports, but do not centralize judgment.
5. Do keep moderation evidence separate from public logs.
6. Do make it clear that the client is experimental and may carry user generated content without project endorsement.

These rules are meant to reduce confusion about official status and to keep the prototype from looking like a centralized service.

## User-Facing Disclaimers

Use short warnings in the prototype client and in any test documentation.

Suggested text:

> Experimental prototype. This client is under active development and may behave unpredictably.

> No official AgoraMesh server, relay, search service, gateway, category list, or media node is provided by the maintainer.

> Content shown here is user generated. It is not reviewed, endorsed, or curated by the maintainer.

> Moderation actions are local to the category or node you are using. They do not mean a central AgoraMesh operator made the decision.

> Do not rely on this prototype for legal, safety, or availability guarantees.

## Next Review Gates

Revisit this note before Phase 4 and again before Phase 8.

Before Phase 4, confirm:

1. How media-node responsibility is separated from text-node responsibility.
2. Whether any image handling creates new hosting, caching, or takedown exposure.
3. Whether media-related warnings need stronger user messaging.
4. Whether the README still avoids any language that sounds like a maintained public service.

Before Phase 8, confirm:

1. How node operator packaging changes the maintainer risk profile.
2. Whether operator docs need separate jurisdiction review.
3. Whether default settings in packaging could be read as endorsement or recommendation.
4. Whether any official-looking distribution channel needs extra disclaimer text.
5. Whether third-party operators need their own terms, notices, or support boundaries.

## Disclaimer

This document is not legal advice.

It is a preliminary wording note for internal review only. Formal legal review is required before any public release, before any official-looking test network, and before any move into media-node or node operator packaging.
