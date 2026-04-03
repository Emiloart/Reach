# Messaging Ingress Service API

## Current implemented surface

- `GET /health/live`
- `GET /health/ready`

## First application responsibilities

- create direct-conversation metadata
- accept authenticated encrypted message envelopes
- enforce idempotency keys
- persist intake metadata and publish accepted envelopes

## Explicit non-responsibilities

- online delivery
- push notifications
- group membership state
- plaintext indexing or search

