# AGENTS.md

## Reach Repository Operating Standard

This file is the authoritative operating contract for:
- Codex
- AI code assistants
- autonomous engineering tools

It applies to all work in this repository unless an explicit repository-owner instruction for the current task says otherwise.

Precedence order:
1. explicit repository-owner instruction in the current task
2. accepted ADRs and implemented repository structure
3. this file

This file is an execution constraint. It is not aspirational prose.

---

## 1. Mission

Reach is a serious privacy-first communications platform for global adoption.

Core goals:
- true end-to-end encrypted messaging
- metadata minimization wherever realistically possible
- secure direct messaging
- secure group messaging
- disappearing messages and self-destructing lifecycle controls
- pseudonymous and scoped identity patterns
- abuse resistance without collapsing the privacy model
- production-grade reliability
- strong long-term maintainability
- serious trust posture from day one

This repository is not for demo-grade social features, shallow scaffolding, or convenience-first shortcuts on core systems.

When forced to choose, optimize in this order:
1. security
2. privacy
3. correctness
4. reliability
5. maintainability
6. scalability
7. developer speed
8. convenience

---

## 2. Engineering Philosophy

Agents and contributors must work like principal-level engineers building a high-trust messaging company.

Default engineering philosophy:
- security first
- privacy first
- modular monolith first, split later only when justified
- strongly typed interfaces
- explicit trust boundaries
- explicit service ownership
- thin transport layers
- application-layer invariants
- low implicit magic
- production-safe implementation over fast scaffolding
- honest reporting of blockers and gaps

Prefer boring, proven infrastructure for critical paths. Do not confuse novelty with quality.

---

## 3. Global Rules

Agents must:
- inspect the repo before proposing or making changes
- preserve the established architecture unless there is a strong reason to change it
- keep security and privacy tradeoffs explicit
- keep service ownership boundaries explicit
- keep durable state, ephemeral state, and encrypted blob boundaries separate
- keep HTTP and transport layers thin
- keep business logic in application and domain layers
- state blockers honestly
- state assumptions explicitly
- provide real validation after meaningful changes
- implement narrow, production-safe steps instead of speculative breadth

Agents must never:
- fabricate completion
- claim infrastructure is wired when it is not
- invent secrets, credentials, or cloud configuration
- present placeholder security as real security
- overstate correctness or readiness
- add broad feature sprawl when a narrow foundational step is required
- move security-sensitive logic into the wrong layer for convenience
- store privacy-critical durable truth in caches
- invent fake cryptographic behavior
- ship demo shortcuts into core systems

If blocked by missing infrastructure, secrets, environment, provider access, or runtime dependencies:
- say so clearly
- identify exactly what is missing
- identify the next concrete step required

---

## 4. Current Architecture Direction

Unless explicitly changed by repository owners, Reach currently assumes:

### Clients
- iOS: Swift + SwiftUI
- Android: Kotlin + Jetpack Compose
- Web: Next.js + TypeScript as a constrained secondary client

### Backend
- Rust for core messaging, security, and control-plane services
- Axum for HTTP
- Tonic/gRPC where internal RPC boundaries are justified
- Tokio runtime

### Data and Infrastructure
- CockroachDB for durable control-plane metadata
- encrypted object storage for media and backup blobs
- Valkey/Redis only for ephemeral state, never for durable truth
- Kubernetes + Terraform + GitOps
- Cloudflare edge
- privacy-minimized observability using OpenTelemetry-based tooling

### Crypto Direction
- Signal-style principles for 1:1 messaging
- MLS/OpenMLS later for group messaging
- per-device trust roots
- hardware-backed key storage where platform support exists

Do not casually replace these choices.

---

## 5. Service Ownership Boundaries

### Identity
Owns:
- accounts
- devices
- account lifecycle state
- device lifecycle state

Does not own:
- session issuance
- refresh token families
- key material
- messaging state

### Auth
Owns:
- sessions
- refresh-family persistence
- session lifecycle state

Does not own:
- account truth
- device truth
- key material
- messaging transport

Auth may read identity lifecycle state through explicit read-only contracts. It does not own identity data.

### Keys
Owns:
- signed prekeys
- one-time prekeys
- current key bundle state
- key lifecycle metadata

Does not own:
- account truth
- device truth
- session or auth truth
- ratchet or session crypto
- message transport

Keys may read identity lifecycle state through explicit read-only contracts. It does not own identity data.

### Messaging Ingress
Owns:
- message intake boundaries
- encrypted envelope intake metadata
- ingress-side validation concerns

Does not own:
- delivery
- fanout
- key lifecycle
- protocol establishment

Messaging ingress must not expand prematurely into delivery or protocol logic before trust and state layers are correct.

Agents must preserve these ownership boundaries unless explicitly instructed otherwise.

---

## 6. Persistence Rules

Durable truth must live in the correct durable store.

Agents must:
- keep CockroachDB as the source of truth for durable service metadata
- keep schema ownership clear per service
- keep object storage limited to encrypted blobs and related blob-manifest concerns
- keep Valkey/Redis limited to ephemeral state only
- define transactional boundaries explicitly when relevant
- define retention and deletion semantics intentionally

Agents must not:
- use caches as durable truth
- blur table ownership between services
- create hidden shared-write access across service domains

Changes should make clear where relevant:
- source of truth
- transaction boundary
- uniqueness and invariants
- deletion semantics
- revocation semantics

---

## 7. Security Rules

Security claims must match actual behavior.

Agents must:
- avoid logging plaintext sensitive content
- avoid logging secrets, token material, token hashes in raw form, recovery material, private keys, or plaintext message content
- avoid storing plaintext message content server-side unless explicitly required and approved
- prefer short-lived credentials
- define trust boundaries clearly
- define threat implications of major changes
- treat admin capability as dangerous by default
- keep authentication and authorization logic explicit
- ensure privileged actions are auditable where relevant

Agents must never:
- add insecure backdoors
- add hidden bypass logic
- add temporary unsafe admin shortcuts into production paths
- roll custom cryptographic primitives
- imply cryptographic guarantees not backed by implementation

For work on auth, keys, device trust, recovery, moderation, or lifecycle controls, agents should state:
- what the server can see
- what the server cannot see
- important attack or abuse considerations
- revocation or failure behavior where relevant

---

## 8. Privacy Rules

Privacy is a first-class requirement.

Agents must prefer:
- data minimization
- bounded retention
- encrypted transport
- encrypted storage where appropriate
- client-side encryption where required
- narrow telemetry
- scoped or pseudonymous identity patterns where needed

Agents must not casually introduce:
- invasive analytics
- centralized tracking assumptions
- Firebase-style behavioral overcollection
- plaintext moderation visibility as a default
- unnecessary long-lived metadata retention

For any telemetry or observability work, agents must specify:
- what is collected
- why it is needed
- why it does not violate Reach's privacy posture

---

## 9. Architecture Constraints

The repository should be treated as a serious monorepo with explicit boundaries.

Current top-level direction:
- `services/`
- `libs/`
- `docs/`
- `apps/` and `infra/` as documented expansion areas

Current shared crates:
- `libs/config`
- `libs/telemetry`
- `libs/auth-types`
- `libs/request-auth`
- `libs/identity-lifecycle`
- `libs/key-material`
- `libs/testing`

Current active service crates:
- `services/identity`
- `services/auth`
- `services/keys`
- `services/messaging-ingress`

Directory presence does not imply active ownership or deployable status. Some future-oriented directories exist in the repo but are not active workspace members yet.

Agents must:
- follow existing naming and workspace patterns
- preserve modular boundaries
- avoid introducing a competing architectural style
- avoid extracting microservices early without a fault-isolation, scale, or security reason

---

## 10. Scope Control Rules

Reach must be built in the correct order.

Current preferred progression:
1. workspace and repository foundations
2. persistence correctness and service ownership
3. application command paths and invariants
4. trust enforcement and explicit authorization
5. key lifecycle completion
6. controlled read-only cross-service lifecycle checks
7. messaging ingress correctness
8. later delivery, transparency, MLS, and broader features

Agents must resist:
- premature breadth
- jumping to delivery before trust and state layers are correct
- bolting on product features before invariants are stable

---

## 11. Testing Expectations

Serious changes require meaningful validation.

Agents should add tests appropriate to the change, including where relevant:
- unit tests
- repository tests
- application-command tests
- concurrency tests
- invalid-state tests
- replay or reuse tests
- migration-related checks
- lifecycle or authorization regression tests

The current repository already uses Cockroach-backed repository and application tests. Do not replace real storage tests with fake in-memory substitutes for core persistence logic.

Validation output should be concrete, for example:
- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test --workspace`

Agents must not claim readiness without identifying remaining untested areas.

---

## 12. Documentation Rules

For meaningful work, agents should provide concise but complete documentation of:
- what changed
- why it changed
- security and privacy implications
- validation performed
- remaining gaps

Use ADRs, schema notes, API notes, and threat-model docs where appropriate.

All structural architecture changes must create or update an ADR.
All major security-sensitive features must reference threat-model documentation.

Avoid filler and generic commentary.

---

## 13. Response Format Expectations

For serious engineering work, agents should usually respond in this shape:
1. Objective
2. Current state observed
3. Risks / constraints
4. Proposed implementation
5. Concrete file or code changes
6. Validation run
7. Remaining gaps

For architecture-specific tasks, agents should usually respond in this shape:
1. Problem framing
2. Recommended design
3. Service boundaries
4. Data model domains
5. Security/privacy implications
6. Operational implications
7. Phased rollout

Responses should be concrete, implementation-aware, and concise.

---

## 14. Explicit Non-Goals

Do not introduce these unless explicitly requested and justified by the current milestone:
- full messaging send or delivery logic
- transparency log implementation
- MLS or group crypto implementation
- contact discovery
- moderation platform expansion
- broad analytics systems
- speculative multi-cloud complexity
- rich admin or control-plane features unrelated to the current step
- fake cryptographic session establishment
- user-facing auth or token systems beyond the current milestone

---

## 15. What Best Result Means Here

For Reach, best result means:
- trustworthy architecture
- clear service ownership
- strong persistence semantics
- narrow and correct state transitions
- explicit trust enforcement
- real privacy and security posture
- honest reporting of blockers
- no fluff
- no fake completeness
- no weak shortcuts on core systems

Agents working in this repository must act accordingly.
