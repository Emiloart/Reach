# ADR 0002: Messaging Ingress Envelope Intake Boundaries

- Status: Accepted
- Date: 2026-04-06

## Context

Reach already has service-owned boundaries for identity, auth, keys, and messaging-ingress, but the messaging-ingress service is still scaffold-only and still reflects an earlier direct-conversation placeholder shape.

The next correct milestone is not delivery or fanout. It is validated encrypted-envelope intake with:
- explicit sender and recipient lifecycle validation
- replay protection
- durable acceptance records
- bounded metadata validation
- narrow key-material resolution sufficient for ingress acceptance only

The repository already contains:
- internal service request authentication
- explicit application-layer authorization
- identity lifecycle read contracts
- keys-owned bundle, signed-prekey, and one-time-prekey persistence

The missing architectural decision is how far messaging-ingress should go at this stage without collapsing into delivery or protocol logic.

## Decision

Messaging-ingress will own validated encrypted-envelope intake only.

For this stage, messaging-ingress will:
- accept encrypted envelopes for a single recipient device
- validate sender and recipient account and device lifecycle state
- validate ingress metadata bounds and timestamp sanity
- enforce durable replay protection
- persist only accepted ingress envelopes plus minimal crypto-resolution metadata
- verify recipient device has current bundle material
- optionally reserve a one-time prekey when the ingress command contract explicitly requires it

Messaging-ingress will not:
- deliver messages
- fan out to recipients
- create inbox or status read models
- dispatch push notifications
- interpret plaintext
- implement session ratchets
- implement contact discovery
- implement moderation workflows

The durable model for this stage is:
- `messaging_ingress.envelope_replay_records`
- `messaging_ingress.accepted_envelopes`

Replay protection must be durable and must occur before one-time-prekey reservation inside the acceptance transaction so duplicate or replayed envelopes do not burn prekeys.

The encrypted envelope model remains minimal and contains only:
- `envelope_id`
- `sender_account_id`
- `sender_device_id`
- `recipient_account_id`
- `recipient_device_id`
- `encrypted_payload`
- `content_type`
- `client_timestamp`
- `replay_nonce`
- `payload_version`

The ingress command contract may carry an additional prekey-resolution requirement because the envelope metadata alone does not state whether a one-time-prekey reservation is required. That requirement is transport metadata for ingress validation, not part of the persisted envelope model.

## Consequences

### Positive

- Messaging-ingress gains a correct, narrow, production-grade responsibility.
- Replay protection is durable and explicit.
- One-time-prekey reservation remains atomic with envelope acceptance.
- Delivery and fanout remain out of scope, reducing false completeness.
- The service can reject envelopes safely when recipient key material is unavailable.

### Negative

- Messaging-ingress now depends on read-only identity lifecycle checks and a narrow keys-material contract.
- Lifecycle validation is still a cross-boundary read and is not yet a serialized multi-service transaction boundary.
- The ingress command contract is slightly broader than the persisted envelope model because it must express whether one-time-prekey reservation is required.

### Operational implications

- Observability must remain metadata-minimized and must not log payload bytes, replay nonces, or raw internal credentials.
- Future delivery extraction must consume accepted-ingress records rather than bypassing ingress validation.
- If the keys service is extracted later, the current modular-monolith key-material contract becomes the replacement boundary for a service client.

## Alternatives considered

### Option A: Keep the existing direct-conversation/message-intake placeholder shape

Rejected because it encodes future delivery concerns too early and does not model recipient device key-material resolution or durable replay protection cleanly.

### Option B: Jump directly to delivery queues and fanout state

Rejected because Reach has not yet completed the ingress correctness layer and doing so would create fake completeness around delivery.

### Option C: Accept envelopes without durable replay protection and rely on cache dedupe

Rejected because replay protection is security-sensitive and durable truth must not live in Valkey or Redis.
