# Threat Model Expectations

This document is the authoritative threat-modeling baseline for Reach.

Reach is a privacy-first communications platform. Security-sensitive work in this repository must be accompanied by threat reasoning, not just implementation.

## Scope

Threat modeling in Reach covers:
- client trust boundaries
- server trust boundaries
- inter-service trust
- durable metadata exposure
- encrypted blob handling
- auth and session lifecycle
- device trust and key lifecycle
- abuse, spam, and adversarial user behavior
- operational and insider risk where relevant

Threat modeling is required for major changes in:
- auth
- keys
- device enrollment
- recovery
- messaging ingress
- media handling
- delivery metadata
- moderation and abuse controls
- admin or privileged tooling

The current baseline trust notes live in [initial-trust-boundaries.md](./initial-trust-boundaries.md).

## Attacker Categories

At minimum, consider these attacker classes:
- external attacker without valid credentials
- malicious or abusive user
- spammer or raider trying to create operational abuse
- attacker with a compromised client device
- attacker with a stolen internal service credential
- malicious insider or over-privileged operator
- cloud or infrastructure-side compromise
- dependency or supply-chain compromise

Not every feature needs deep analysis for every attacker, but major changes must state which categories matter.

## Trust Boundaries

Reach must reason explicitly about:
- client boundary: device-held private keys, local secure storage, message plaintext before encryption
- service boundary: each service owns only its own durable metadata and commands
- inter-service boundary: authenticated internal service calls, explicit authz scopes, no hidden trust leaps
- database boundary: CockroachDB holds durable service metadata, not plaintext message content
- object storage boundary: encrypted blobs only
- ephemeral store boundary: Valkey or Redis for transient state only
- observability boundary: logs, traces, and metrics must not become a privacy backchannel
- admin boundary: privileged access is dangerous by default and must remain narrow and auditable

## Sensitive Assets

At minimum, treat these as sensitive:
- device private keys
- recovery material
- bearer credentials and internal service credentials
- raw refresh tokens
- key lifecycle metadata that enables impersonation or replay abuse
- session lifecycle state
- account and device lifecycle state
- delivery and routing metadata
- encrypted media manifests and retention metadata
- abuse evidence bundles

Some of these assets should never exist server-side in plaintext.

## Key Compromise Scenarios

Threat notes for key-related work must consider:
- compromised end-user device private key
- stale signed prekey or failed prekey rotation
- reuse or exhaustion of one-time prekeys
- server presenting inconsistent public key state
- stolen internal caller credential publishing malicious key state
- delayed revocation after device compromise

If the implementation does not yet mitigate a scenario fully, state that explicitly.

## Metadata Exposure Risks

Reach must treat metadata as a real product risk.

Major changes should consider exposure of:
- account identifiers
- device identifiers
- session creation and revocation timing
- message intake timing
- ciphertext size
- key fetch patterns
- delivery attempts
- presence signals
- abuse-report workflows

Do not assume end-to-end encryption solves metadata risk.

## Abuse and Spam Considerations

Threat notes for user-facing flows should consider:
- scripted account creation
- device churn for ban evasion
- invite abuse
- prekey exhaustion or retrieval abuse
- spam amplification through future delivery paths
- malicious reporting or evidence poisoning

Privacy does not remove the need for abuse resistance.

## Moderation and Privacy Tension

Reach must not assume either extreme:
- "privacy means no moderation"
- "moderation requires total visibility"

Security-sensitive features that affect abuse handling should document:
- what evidence is available
- what evidence requires user action
- what the server can evaluate without plaintext content
- what operational limits remain

## Required Threat-Model Notes for Major Features

All major security-sensitive features must reference threat-model documentation.

At minimum, a feature note should state:
- scope
- assets involved
- relevant attacker categories
- trust boundaries crossed
- what the server can see
- what the server cannot see
- main abuse or compromise scenarios
- major mitigations
- known residual risks

Threat-model notes may live:
- in a dedicated file under `docs/threat-model/`
- in an ADR when the decision is architectural
- in service-specific docs if the scope is narrow

They must be linkable and reviewable.
