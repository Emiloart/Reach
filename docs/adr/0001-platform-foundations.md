# ADR 0001: Platform Foundations

- Status: Accepted
- Date: 2026-04-03

## Context

Reach is being built as a privacy-first communications platform, not as a conventional social product. The initial repository is empty, so the first architectural decision needs to define the baseline choices that all future work inherits unless explicitly changed.

The main constraints are:

- privacy guarantees must shape the architecture from the start
- mobile security and device trust are first-class requirements
- metadata minimization matters almost as much as message encryption
- the system needs a credible path from MVP to multi-region global scale

## Decision

Reach will adopt the following platform foundations:

1. Native mobile clients are the primary trust anchor.
   - iOS will use Swift and SwiftUI.
   - Android will use Kotlin and Jetpack Compose.
   - Web will be a constrained secondary client using Next.js and TypeScript.

2. Core backend services will be written in Rust.
   - Axum will serve public and internal HTTP APIs.
   - Tonic/gRPC will be used for internal service contracts.
   - Tokio will be the async runtime.

3. Reach will use established cryptographic building blocks.
   - libsignal-style sessions for 1:1 messaging
   - MLS/OpenMLS roadmap for group messaging
   - append-only key transparency for auditability

4. Durable metadata will live in CockroachDB.
   - object storage will hold encrypted blobs only
   - Valkey will hold ephemeral state only

5. The first production shape will be a modular monolith with strict boundaries.
   - service ownership is defined before service extraction
   - schema ownership is explicit from day one

6. Production traffic will run behind Cloudflare on Kubernetes in multiple regions.
   - GCP is the primary cloud
   - Terraform, Helm, and Argo CD are the infrastructure control path

7. Observability is mandatory but privacy-minimized.
   - OpenTelemetry is standard everywhere
   - logs, traces, and dashboards must avoid plaintext user content

## Consequences

### Positive

- Strong alignment between product promise and technical implementation
- Better long-term security posture for mobile key handling and backend memory safety
- Cleaner path to multi-region durability and service extraction
- Lower risk of rebuilding critical pieces after MVP traction

### Negative

- Higher engineering complexity than a conventional chat app stack
- Slower initial implementation compared with a JavaScript-heavy backend
- More up-front architecture work before product iteration feels fast

### Operational implications

- security review and protocol review must happen before broad launch
- web client capabilities will remain intentionally constrained
- admin, analytics, and trust tooling must respect privacy boundaries by design

## Follow-up ADRs Required

- protocol selection for first group messaging release
- contact discovery design
- encrypted backup and recovery design
- self-managed versus managed CockroachDB posture
- RTC architecture and call metadata policy
