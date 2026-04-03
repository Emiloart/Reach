# CockroachDB Schema Ownership

## Initial schema owners

- `identity`: owned by `reach-identity-service`
- `auth`: owned by `reach-auth-service`
- `keys`: owned by `reach-key-service`
- `messaging_ingress`: owned by `reach-messaging-ingress-service`

## Ownership rule

Each service may write only to its own schema. Cross-service reads must happen through service interfaces, not direct table writes into another schema.

## Initial retention notes

- `identity.accounts`: retain until account purge workflow completes
- `identity.devices`: retain until account purge or explicit device revocation retention window ends
- `auth.sessions`: retain through active lifetime plus bounded audit window
- `auth.refresh_token_families`: retain until expiration plus compromise investigation window
- `keys.key_bundles`: retain current plus bounded prior bundle history
- `keys.one_time_prekeys`: retain only until claim retry safety window closes
- `messaging_ingress.message_intake`: retain only bounded delivery metadata, never plaintext content

