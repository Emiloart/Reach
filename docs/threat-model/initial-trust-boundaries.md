# Initial Trust Boundaries

## Client trust boundary

The client is responsible for generating and protecting private key material, establishing end-to-end encrypted sessions, and encrypting message payloads before network transmission.

## Server trust boundary

The initial server implementation may observe:

- account identifiers
- device identifiers
- session metadata
- public device key bundles
- ciphertext size
- message intake timing
- direct-conversation routing metadata

The initial server implementation must not observe:

- plaintext message content
- private keys
- plaintext media content
- raw refresh tokens at rest

## Admin trust boundary

No admin surface is being implemented in this phase. No privileged bypass path should be added to mutate account, session, key, or message-ingress state outside normal service-owned code paths.
