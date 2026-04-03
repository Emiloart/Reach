# Reach

Reach is a privacy-first communications platform for direct messaging, small private groups, and pseudonymous communities.

The product is being designed as a communications security system with consumer-grade UX, not as a conventional social app. That means the architecture is optimized around end-to-end encryption, metadata minimization, scoped identity, ephemeral behavior, and abuse controls that do not depend on blanket content visibility.

## Current State

This repository currently contains the foundational architecture documentation for the first production build of Reach. The implementation should follow the monorepo and service boundaries described in the blueprint before feature work begins.

## Core Principles

- Privacy is a product requirement, not a settings screen.
- The server may route encrypted messages, but it should learn as little as possible about who said what and when.
- Per-device trust is the root of account security.
- Messaging, control-plane, and infrastructure concerns must remain separated.
- Observability must be privacy-minimized and operationally useful at the same time.
- Moderation must rely on scoped evidence and explicit user action, not default surveillance.

## Docs

- [Production Architecture Blueprint](./docs/architecture/production-blueprint.md)
- [ADR 0001: Platform Foundations](./docs/adr/0001-platform-foundations.md)

## Initial Repository Shape

The intended implementation layout is documented in the blueprint, but the target top-level structure is:

```text
apps/
services/
libs/
infra/
docs/
```

## Reference Inputs

This initial blueprint is based on the product and stack direction already chosen for Reach:

- Native mobile clients: Swift/SwiftUI and Kotlin/Jetpack Compose
- Web as a constrained secondary client: Next.js and TypeScript
- Core backend: Rust, Axum, Tonic, Tokio
- 1:1 encryption: Signal-style sessions via libsignal
- Group roadmap: MLS/OpenMLS
- Primary metadata store: CockroachDB
- Encrypted media and backups: R2 or S3-compatible object storage
- Ephemeral state: Valkey
- Edge and security perimeter: Cloudflare
- Orchestration and deployment: Kubernetes, Terraform, Argo CD

## Next Documentation Additions

- Threat model and trust boundaries
- Protocol notes for 1:1, group, and device-linking flows
- Service-level API contracts
- Operational runbooks and incident classes
- Data retention and deletion policy
