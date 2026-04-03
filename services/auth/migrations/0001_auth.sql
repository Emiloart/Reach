CREATE SCHEMA IF NOT EXISTS auth;

CREATE TABLE IF NOT EXISTS auth.sessions (
    session_id UUID PRIMARY KEY,
    account_id UUID NOT NULL,
    device_id UUID NOT NULL,
    state STRING NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL,
    last_refreshed_at TIMESTAMPTZ NULL,
    access_token_jti UUID NOT NULL
);

CREATE INDEX IF NOT EXISTS sessions_account_device_state_idx
    ON auth.sessions (account_id, device_id, state);

CREATE INDEX IF NOT EXISTS sessions_expires_at_idx
    ON auth.sessions (expires_at);

CREATE TABLE IF NOT EXISTS auth.refresh_token_families (
    family_id UUID PRIMARY KEY,
    session_id UUID NOT NULL UNIQUE REFERENCES auth.sessions (session_id),
    current_token_hash BYTES NOT NULL,
    previous_token_hash BYTES NULL,
    rotation_counter INT8 NOT NULL,
    compromised_at TIMESTAMPTZ NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS refresh_token_families_expires_at_idx
    ON auth.refresh_token_families (expires_at);

CREATE INDEX IF NOT EXISTS refresh_token_families_compromised_at_idx
    ON auth.refresh_token_families (compromised_at)
    WHERE compromised_at IS NOT NULL;
