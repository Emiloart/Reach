# Repository Structure

This document is the authoritative map of repository ownership and structure for Reach.

Directory presence does not automatically mean active implementation or deployable status. Active workspace members and accepted ADRs are the source of truth for what is currently implemented.

## Top-Level Areas

### `services/`

Service-owned application crates and their schema ownership.

Current active workspace services:
- `services/identity`
- `services/auth`
- `services/keys`
- `services/messaging-ingress`

Future-oriented service directories may exist, but they are not active deployable scope until explicitly added to the workspace and justified by the current milestone.

### `libs/`

Shared libraries with narrow responsibilities. Shared crates must not become a hidden monolith.

Current active shared crates:
- `libs/config`: typed configuration loading and validation
- `libs/telemetry`: tracing and observability setup
- `libs/auth-types`: shared principal, scope, and auth-domain types
- `libs/request-auth`: HTTP-boundary internal request authentication
- `libs/identity-lifecycle`: read-only lifecycle checks for dependent services
- `libs/testing`: Cockroach-backed test support

Future-oriented library directories may exist, but they are not automatically active design commitments.

### `apps/`

Client applications and future product surfaces.

Current architectural direction:
- native mobile is the primary trust anchor
- web is a constrained secondary client

This directory is part of the intended monorepo shape even where implementation is not yet active.

### `infra/`

Infrastructure-as-code, deployment topology, and operational configuration.

Current direction includes:
- Terraform
- Helm
- Argo CD
- Cloudflare edge posture
- Kubernetes deployment topology

Infrastructure code must follow the same security and auditability standards as service code.

### `docs/`

Repository documentation spine.

This area contains:
- ADRs
- architecture docs
- API notes
- schema notes
- runbooks
- threat-model notes
- repository governance documents

## Service Ownership

### Identity Service

Owns:
- accounts
- devices
- account lifecycle state
- device lifecycle state

Does not own:
- sessions
- refresh families
- key material
- message delivery or routing

### Auth Service

Owns:
- sessions
- refresh-family persistence
- session lifecycle state

Does not own:
- account truth
- device truth
- key material
- messaging delivery

Auth may read identity lifecycle state through explicit read-only contracts. It does not own identity data.

### Keys Service

Owns:
- signed prekeys
- one-time prekeys
- current bundle state
- key lifecycle metadata

Does not own:
- account truth
- device truth
- session truth
- ratchet/session crypto
- message delivery

Keys may read identity lifecycle state through explicit read-only contracts. It does not own identity data.

### Messaging Ingress Service

Owns:
- ingress boundaries for encrypted message envelopes
- intake-side metadata and validation concerns

Does not own:
- delivery
- fanout
- broader protocol logic
- key lifecycle

Messaging ingress must remain narrow until earlier trust and lifecycle layers are stable.

## Why CockroachDB Owns Durable State

Reach requires durable metadata with:
- explicit transactional semantics
- clear uniqueness constraints
- auditable schema ownership
- multi-region-ready control-plane posture

CockroachDB is used for durable service metadata because it provides relational integrity for accounts, devices, sessions, and key lifecycle state. These are not cache-like concerns and should not be modeled as ephemeral state.

Durable truth belongs in service-owned Cockroach schemas.

## Why Redis or Valkey Is Ephemeral Only

Redis or Valkey may be used only for:
- short-lived routing hints
- rate windows
- temporary presence state
- other bounded transient state

Redis or Valkey must not become:
- the primary source of truth
- the only record of security-sensitive state
- a hidden replacement for Cockroach-backed invariants

Using ephemeral stores as durable truth would degrade correctness, auditability, and security posture.
