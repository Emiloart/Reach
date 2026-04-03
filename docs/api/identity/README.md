# Identity Service API

## Current implemented surface

- `GET /health/live`
- `GET /health/ready`

## First application responsibilities

- create opaque accounts
- register linked devices
- revoke devices
- mark accounts for deletion

## Explicit non-responsibilities

- token issuance
- push token storage
- key material storage
- message routing

