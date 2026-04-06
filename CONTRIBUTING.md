# Contributing to Reach

This document is the human-oriented contributor workflow for Reach.

Reach is a serious privacy-first communications platform. Contributions must preserve the repository's security, privacy, persistence, and service-boundary rules.

## Repository Setup

Required baseline:
- Git
- Rust toolchain `1.94.1`
- `rustfmt`
- `clippy`

The repository pins the toolchain in [rust-toolchain.toml](./rust-toolchain.toml).

Recommended setup:

```powershell
rustup toolchain install 1.94.1
rustup component add rustfmt clippy --toolchain 1.94.1
```

## CockroachDB Test Requirement

Repository and application persistence tests use a Cockroach-backed local harness through `libs/testing`.

Provide a Cockroach binary in one of these ways:
- set `REACH_COCKROACH_BIN`
- place the binary at `.tools/cockroach/cockroach` or `.tools/cockroach/cockroach.exe`
- ensure `cockroach` is available on `PATH`

Do not replace these tests with fake in-memory substitutes for core persistence logic.

## Workspace Commands

Common commands:

```powershell
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Equivalent `just` targets:

```powershell
just fmt
just check
just test
just lint
```

## Formatting Rules

Contributors must:
- keep code formatted with `rustfmt`
- keep warnings under control
- prefer clear names and explicit types
- keep HTTP thin and application/domain logic explicit
- preserve existing workspace and module patterns

Do not mix unrelated refactors into focused changes.

## Testing Rules

Before opening a pull request, run:

```powershell
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
```

If a change cannot pass one of these in the current environment:
- say so clearly
- state what blocked validation
- do not claim completion without that context

## Commit Conventions

Use concise, scoped commit messages. The current repo already follows conventional prefixes such as:
- `docs:`
- `feat:`
- `fix:`
- `build:`
- `test:`
- `refactor:`

Keep commits focused. Do not bury unrelated changes inside one commit.

## Migration Practices

Each service owns its own database migrations.

Rules:
- place migrations under the owning service
- do not write across another service's schema ownership
- make invariants explicit with constraints and indexes
- describe schema ownership clearly in docs where needed
- avoid speculative tables for future features

Durable truth belongs in CockroachDB, not in Redis or Valkey.

## Service Ownership Boundaries

Contributors must preserve service boundaries.

Identity owns:
- accounts
- devices
- lifecycle state

Auth owns:
- sessions
- refresh-family persistence

Keys owns:
- signed prekeys
- one-time prekeys
- current bundle state

Messaging ingress owns:
- encrypted envelope intake boundaries

Do not move ownership casually between services. If the boundary changes structurally, create an ADR.

## Security Expectations

Contributors must:
- avoid logging sensitive plaintext
- avoid storing plaintext message content server-side unless explicitly approved
- avoid inventing fake cryptographic behavior
- keep authentication and authorization explicit
- keep privileged actions narrow and auditable
- state server visibility and trust implications when changing security-sensitive flows

Do not add unsafe backdoors, bypasses, or convenience shortcuts.

## Documentation Expectations

Add or update docs when work changes:
- architecture
- threat boundaries
- persistence ownership
- API contracts
- operational expectations

Structural architecture changes require a new ADR under `docs/adr/`.

Security-sensitive features must reference threat-model notes under `docs/threat-model/` or equivalent documented reasoning.
