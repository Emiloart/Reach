# Reach

**Privacy-first communications infrastructure, built around device trust, encrypted message transport, and metadata minimization.**

Reach is being engineered as privacy-first communications infrastructure. Its core scope is secure direct messaging and private group messaging, supported by strong device trust, end-to-end encryption, and metadata minimization. Reach is not an anonymous community or social-network product.

The goal is not to build another conventional social app. Reach is being built around a clear separation between identity, authentication, device keys, message intake, and eventual delivery, so each security-sensitive domain has explicit ownership and enforceable boundaries.

> **Project status:** Early implementation. The Rust workspace and foundational service boundaries are in place, including identity, authentication, key lifecycle, shared security libraries, and messaging-ingress foundations. Reach is not yet a production-ready messaging client or a complete end-to-end encrypted messaging service.

## Design principles

- **Client-side confidentiality.** Message plaintext should remain within client trust boundaries; servers handle encrypted payloads and the minimum routing metadata required.
- **Metadata minimization.** Collect, retain, and expose as little information as practical. Logs, traces, and metrics must not contain message plaintext or secret material.
- **Device-level trust.** Devices are explicit security principals with their own lifecycle and key material.
- **Communication-first scope.** Product capabilities must serve secure communication; anonymous communities, public social feeds, and social publishing are outside Reach's core scope.
- **Clear ownership boundaries.** Identity, authentication, key management, and messaging ingress have separate responsibilities and state ownership.
- **Established cryptography.** Build on reviewed protocols and libraries rather than inventing cryptographic primitives.
- **Privacy-compatible abuse controls.** Abuse response should use scoped evidence and explicit user action, not routine access to everyone’s message content.
- **Operational correctness.** Persistence, authorization, replay handling, retention, observability, and recovery are part of the security model.
- **Honest security claims.** Planned architecture is not presented as shipped functionality. Guarantees will be claimed only when implemented and validated.

## Current implementation

The active codebase is a Rust Cargo workspace. Current workspace members include:

| Component | Responsibility |
| --- | --- |
| `services/identity` | Account and device lifecycle |
| `services/auth` | Session and authentication lifecycle |
| `services/keys` | Device key and prekey lifecycle metadata |
| `services/messaging-ingress` | Authenticated message-envelope intake and ingress validation boundaries |
| `libs/config` | Shared configuration |
| `libs/telemetry` | Shared, privacy-conscious telemetry foundations |
| `libs/auth-types` | Shared authentication types |
| `libs/request-auth` | Request authentication primitives |
| `libs/identity-lifecycle` | Shared identity lifecycle contracts |
| `libs/key-material` | Shared key-material types and handling boundaries |
| `libs/testing` | Shared test support |

Workspace membership means a crate is part of the current codebase; it does not imply that every planned service, protocol, or production deployment is complete.

### What is not complete

Reach is still being built in stages. The broader product architecture includes planned components that are not yet implemented as complete production capabilities, including:

- Full client-side 1:1 session establishment and end-to-end encrypted messaging
- Recipient delivery, fan-out, offline queues, and delivery acknowledgements
- Production-ready iOS and Android applications
- MLS-backed group messaging
- Encrypted media upload and retrieval flows
- Key transparency and independent verification
- Device linking, encrypted backup, and recovery
- Production deployment automation, operational runbooks, and independent security review

Do not interpret the roadmap or architecture blueprint as evidence that these capabilities are already available.

## Architecture direction

Reach follows a **modular-monolith-first** approach: establish clear domain, interface, and persistence boundaries in the Rust workspace before extracting independently deployed services. Service extraction must be justified by fault isolation, security, or demonstrated scale requirements.

### Technology baseline

| Area | Direction |
| --- | --- |
| Core backend | Rust, Tokio, Axum |
| Internal service contracts | Tonic/gRPC where justified |
| Database access | SQLx |
| Durable metadata | CockroachDB |
| Ephemeral state | Valkey; never the source of durable truth |
| Event backbone | NATS JetStream |
| Encrypted object storage | Cloudflare R2 or compatible S3 storage |
| iOS | Swift, SwiftUI, Keychain, Secure Enclave where available |
| Android | Kotlin, Jetpack Compose, Android Keystore |
| Web | Next.js and TypeScript as a constrained secondary client |
| Infrastructure | GCP, Cloudflare, Kubernetes, Terraform, Helm, Argo CD |
| Secrets and key infrastructure | Vault and cloud KMS-backed services |
| Observability | OpenTelemetry with privacy-scrubbed logs, traces, and metrics |

These are the accepted architectural direction, not a claim that every component is currently provisioned or wired into the running application.

## Security and privacy boundaries

Reach's implementation and review process is governed by the following constraints:

- Authentication does not own account or device truth.
- Identity does not own session credentials or cryptographic key material.
- The key service manages key lifecycle metadata; it does not implement client-side session cryptography.
- Messaging ingress accepts and validates envelopes; it must not silently grow into delivery, fan-out, or protocol-establishment logic.
- Durable state belongs in its designated database domain. Valkey is reserved for ephemeral state.
- Plaintext message bodies, private keys, tokens, recovery secrets, and sensitive payloads must not be written to logs or telemetry.
- Cross-service lifecycle checks must use explicit contracts rather than shared-write access to another service's state.
- Cryptographic protocol choices and security-sensitive architectural changes require explicit documentation and review.

These are engineering requirements. The actual guarantees of any release depend on implementation and verification.

## Message path: intended model

The planned direct-message path is:

1. The sender resolves the recipient's eligible devices and public key material.
2. The sender's client establishes or advances a reviewed end-to-end encryption session.
3. The client encrypts the message locally and submits an encrypted envelope.
4. Messaging ingress authenticates the request and applies intake validation and replay/idempotency controls.
5. An eventual delivery subsystem routes the encrypted payload to recipient devices.
6. Recipient clients verify and decrypt the message locally.

Steps involving full client cryptography and recipient delivery remain roadmap work until implemented. Messaging ingress is not, by itself, a complete messaging system.

## Repository layout

```text
.
├── Cargo.toml
├── Cargo.lock
├── AGENTS.md
├── services/
│   ├── identity/
│   ├── auth/
│   ├── keys/
│   └── messaging-ingress/
├── libs/
│   ├── config/
│   ├── telemetry/
│   ├── auth-types/
│   ├── request-auth/
│   ├── identity-lifecycle/
│   ├── key-material/
│   └── testing/
└── docs/
    ├── adr/
    └── architecture/
```

The layout above describes the current workspace at a high level. Future client, delivery, media, group, transparency, and infrastructure directories should be added as implementation begins rather than treated as already shipped components.

## Development

### Prerequisites

- Rust toolchain compatible with the workspace's declared minimum Rust version
- Cargo
- Any database or environment dependencies required by the tests you intend to run

### Validate the workspace

From the repository root:

```bash
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
```

Some integration tests may require a configured CockroachDB-compatible test database or other environment dependencies. Check the relevant test configuration before interpreting an environment-related failure as a code failure.

### Engineering rules

Read [AGENTS.md](./AGENTS.md) before making changes. It defines repository-specific constraints for AI agents and contributors, including service ownership, persistence, security, privacy, testing, and documentation.

## Documentation

- [Production Architecture Blueprint](./docs/architecture/production-blueprint.md) — target architecture, service responsibilities, data domains, infrastructure direction, and phased roadmap.
- [ADR 0001: Platform Foundations](./docs/adr/0001-platform-foundations.md) — accepted foundational technology and architecture decisions.
- [AGENTS.md](./AGENTS.md) — binding repository operating standards for engineering work.

Protocol specifications, threat models, API contracts, retention rules, incident runbooks, and release gates should be added as the corresponding systems are designed and implemented.

## Roadmap

Reach is being developed in dependency order:

1. **Foundations:** workspace conventions, persistence semantics, configuration, telemetry, and shared types.
2. **Identity and trust:** account/device lifecycle, authentication, key lifecycle, authorization, and revocation behavior.
3. **Messaging intake:** authenticated encrypted-envelope intake, validation, and idempotency/replay protections.
4. **End-to-end messaging:** client cryptography, recipient delivery, offline behavior, and delivery acknowledgements.
5. **Private communications:** encrypted attachments, disappearing-message semantics, device linking, and secure group messaging.
6. **Verification and scale:** key transparency, MLS-backed groups, operational hardening, multi-region deployment, and independent security review.

Roadmap order may change when implementation evidence or security review identifies a dependency that must be resolved first.

## Contributing

Reach prioritizes security, privacy, correctness, and maintainability over feature breadth or implementation speed. Changes should be narrow, tested, and documented. Architectural changes require an ADR; major security-sensitive changes should reference the relevant threat model.

Before proposing a change, inspect the existing code and accepted ADRs. Do not introduce custom cryptography, treat caches as durable truth, expand ingress into delivery prematurely, or claim security properties that have not been implemented and validated.

## License

Reach is currently declared as `Apache-2.0` in the Cargo workspace metadata. Consult the repository's root license file for the authoritative license text.
