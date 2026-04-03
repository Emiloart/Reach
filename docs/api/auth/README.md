# Auth Service API

## Current implemented surface

- `GET /health/live`
- `GET /health/ready`

## First application responsibilities

- bootstrap session issuance
- refresh token rotation
- session revocation
- access token validation

## Explicit non-responsibilities

- account creation
- device key registration
- conversation ownership
- message plaintext handling

