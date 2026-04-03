CREATE SCHEMA IF NOT EXISTS identity;

CREATE TABLE IF NOT EXISTS identity.accounts (
    account_id UUID PRIMARY KEY,
    state STRING NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    deletion_requested_at TIMESTAMPTZ NULL,
    purge_after TIMESTAMPTZ NULL,
    suspension_code STRING NULL
);

CREATE INDEX IF NOT EXISTS accounts_state_idx
    ON identity.accounts (state);

CREATE INDEX IF NOT EXISTS accounts_purge_after_idx
    ON identity.accounts (purge_after)
    WHERE purge_after IS NOT NULL;

CREATE TABLE IF NOT EXISTS identity.devices (
    device_id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES identity.accounts (account_id),
    device_number INT4 NOT NULL,
    platform STRING NOT NULL,
    device_label STRING NULL,
    app_version STRING NOT NULL,
    status STRING NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL,
    attestation_type STRING NULL,
    attestation_summary BYTES NULL,
    CONSTRAINT devices_account_device_number_unique UNIQUE (account_id, device_number)
);

CREATE INDEX IF NOT EXISTS devices_account_status_idx
    ON identity.devices (account_id, status);

