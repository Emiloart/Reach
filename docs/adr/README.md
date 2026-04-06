# ADR Process

This document is the authoritative process for recording architecture decisions in Reach.

Use ADRs to capture decisions that change structure, ownership, trust boundaries, persistence, operational posture, or long-lived implementation direction.

## Purpose

ADRs exist so that Reach architecture changes are:
- explicit
- reviewable
- traceable over time
- comparable against alternatives
- understandable by both human contributors and autonomous tools

If a structural decision matters enough to shape code, data ownership, trust boundaries, or operations, it must be recorded as an ADR.

## When an ADR Is Required

Create a new ADR for changes such as:
- new service boundaries
- new shared library boundaries that affect multiple services
- changes to durable storage strategy
- changes to trust or authorization architecture
- new cryptographic architecture choices
- changes to key lifecycle or recovery model
- infra posture changes with operational or security impact
- introducing or deactivating a deployable service
- changes to client trust-anchor strategy

Do not hide structural decisions inside code review comments or commit messages.

## ADR Numbering Format

Use zero-padded numeric prefixes with a short slug:

`0001-platform-foundations.md`

Rules:
- numbering is sequential
- never reuse a number
- never renumber existing ADRs
- one file per ADR

## ADR Lifecycle

Allowed statuses:
- Proposed
- Accepted
- Superseded
- Deprecated
- Rejected

Lifecycle rules:
- new ADRs start as `Proposed` unless already accepted by repository owners
- once accepted, implementation should align with the ADR
- if a later decision replaces an earlier one, create a new ADR and mark the older one `Superseded`
- do not silently edit history by rewriting the meaning of an accepted ADR

## Required ADR Template

Use this structure:

```md
# ADR 000X: Short Title

- Status: Proposed | Accepted | Superseded | Deprecated | Rejected
- Date: YYYY-MM-DD

## Context

What problem exists now?
What constraints matter?
What current implementation or architecture is relevant?

## Decision

What is being decided?
What is in scope?
What is explicitly out of scope?

## Consequences

### Positive

- ...

### Negative

- ...

### Operational implications

- ...

## Alternatives considered

### Option A

Why it was not chosen.

### Option B

Why it was not chosen.
```

## ADR Expectations

An ADR should be:
- specific
- narrow enough to be reviewable
- concrete about impact
- honest about tradeoffs
- aligned with implemented repository boundaries

An ADR should not be:
- marketing language
- vague future vision
- a substitute for threat-modeling notes when security-sensitive details are needed

## Relationship to Code and Docs

Structural architecture changes must create a new ADR before or alongside the implementation.

Supporting docs may live elsewhere, including:
- `docs/architecture/`
- `docs/schema/`
- `docs/threat-model/`
- service-level API docs

Those docs do not replace the need for an ADR when the change is architectural.
