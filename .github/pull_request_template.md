## Scope of Change

Describe exactly what this PR changes and what it intentionally does not change.

## Services Affected

List the affected services, shared libraries, docs, or infra areas.

## Security Impact

Describe any trust-boundary, authn/authz, secret-handling, key-lifecycle, or attack-surface impact.

## Privacy Impact

Describe any metadata, retention, observability, or server-visibility impact.

## Persistence Impact

Describe any schema, migration, transaction-boundary, ownership, or deletion-semantics impact.

## Validation Run Output

Paste the commands run and their outcome.

Expected baseline:

```text
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
```

## Remaining Gaps

List known limitations, deferred work, and anything not yet validated.

## Completion Check

- [ ] Scope of change is accurately described
- [ ] Services affected are listed
- [ ] Security impact is documented
- [ ] Privacy impact is documented
- [ ] Persistence impact is documented
- [ ] Validation run output is included
- [ ] Remaining gaps are listed

PRs missing this information should not be considered complete.
