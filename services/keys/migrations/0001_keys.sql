CREATE SCHEMA IF NOT EXISTS keys;

CREATE TABLE IF NOT EXISTS keys.key_bundles (
    bundle_id UUID PRIMARY KEY,
    device_id UUID NOT NULL,
    bundle_version INT8 NOT NULL,
    identity_key_public BYTES NOT NULL,
    identity_key_alg STRING NOT NULL,
    signed_prekey_id UUID NOT NULL,
    published_at TIMESTAMPTZ NOT NULL,
    superseded_at TIMESTAMPTZ NULL,
    is_current BOOL NOT NULL,
    CONSTRAINT key_bundles_device_version_unique UNIQUE (device_id, bundle_version)
);

CREATE UNIQUE INDEX IF NOT EXISTS key_bundles_current_device_idx
    ON keys.key_bundles (device_id)
    WHERE is_current = true;

CREATE TABLE IF NOT EXISTS keys.signed_prekeys (
    signed_prekey_id UUID PRIMARY KEY,
    device_id UUID NOT NULL,
    public_key BYTES NOT NULL,
    signature BYTES NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    superseded_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS signed_prekeys_device_created_at_idx
    ON keys.signed_prekeys (device_id, created_at DESC);

CREATE TABLE IF NOT EXISTS keys.one_time_prekeys (
    prekey_id UUID PRIMARY KEY,
    device_id UUID NOT NULL,
    public_key BYTES NOT NULL,
    state STRING NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    claimed_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS one_time_prekeys_device_state_idx
    ON keys.one_time_prekeys (device_id, state);

CREATE INDEX IF NOT EXISTS one_time_prekeys_device_created_at_idx
    ON keys.one_time_prekeys (device_id, created_at);

