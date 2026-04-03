# Key Service API

## Current implemented surface

- `GET /health/live`
- `GET /health/ready`

## First application responsibilities

- publish current public key bundle per device
- publish signed prekeys
- accept one-time prekey batches
- atomically claim one-time prekeys

## Explicit non-responsibilities

- private key storage
- Double Ratchet session state
- account sessions
- message transport

