# Messaging Ingress Envelope Intake Threat Model

## Scope

This note covers validated encrypted-envelope intake in the messaging-ingress service.

In scope:
- sender and recipient lifecycle validation
- replay protection
- bounded envelope metadata validation
- recipient bundle verification
- optional one-time-prekey reservation for accepted envelopes
- durable persistence of accepted ingress records

Out of scope:
- delivery
- fanout
- push dispatch
- group messaging
- moderation workflows
- full session-ratchet establishment

## Assets

Sensitive assets involved in this step:
- internal service credentials used at the HTTP boundary
- sender and recipient account/device lifecycle metadata
- recipient current bundle identifiers
- one-time-prekey availability state
- encrypted payload bytes
- replay nonces
- accepted-envelope timing metadata

The server can see:
- account IDs
- device IDs
- encrypted payload bytes
- payload size
- content type label
- payload version label
- client timestamp
- replay nonce
- accepted-at time
- recipient bundle and prekey identifiers used for acceptance

The server cannot see:
- plaintext message content
- private keys
- ratchet state
- decrypted media

## Attacker Categories

Relevant attacker classes for this scope:
- external attacker without valid internal credentials
- malicious or abusive user sending malformed or replayed envelopes
- attacker with a compromised sender device
- attacker with a stolen internal service credential
- malicious insider attempting to observe or mutate ingress data

## Trust Boundaries

- HTTP boundary: only authenticated internal service callers may invoke ingress commands in this phase
- application boundary: validation and authorization must happen before persistence
- identity boundary: sender and recipient lifecycle state is read from identity-owned durable truth
- keys boundary: recipient bundle state is read from keys-owned durable truth and one-time-prekey reservation is explicit and narrow
- database boundary: CockroachDB is the durable source of truth for replay protection and accepted envelopes

## Key Compromise and Resolution Risks

Relevant risks:
- sender submits to a recipient device with stale or missing current bundle material
- one-time-prekey reservation is consumed for a duplicate or replayed envelope
- concurrent duplicates attempt to reserve multiple one-time prekeys
- recipient device is revoked between lifecycle read and acceptance commit

Mitigations in this step:
- require recipient current bundle presence
- reserve replay protection before one-time-prekey claim in the acceptance transaction
- keep one-time-prekey claim and accepted-envelope insert in one Cockroach transaction
- reject when recipient key material is unavailable

Residual risk:
- lifecycle checks are still cross-boundary reads and not yet serialized with the acceptance transaction

## Metadata Exposure Risks

Ingress acceptance exposes metadata by necessity:
- sender and recipient identifiers
- timing of accepted envelopes
- payload size
- replay nonce uniqueness
- recipient key-material availability

Controls:
- do not log encrypted payload bytes
- do not log replay nonces
- do not log raw internal credentials
- keep persisted fields minimal and ingress-specific

## Abuse and Spam Considerations

Current ingress controls help with:
- replay attempts using the same nonce
- malformed oversized payloads
- envelopes targeting inactive accounts or devices
- envelopes targeting devices without required key material

This step does not solve:
- spam campaigns
- invite abuse
- moderation workflows
- rate-based throttling

Those remain later milestones.
