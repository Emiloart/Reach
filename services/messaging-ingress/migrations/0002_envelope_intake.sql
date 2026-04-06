DROP TABLE IF EXISTS messaging_ingress.idempotency_keys;
DROP TABLE IF EXISTS messaging_ingress.message_intake;
DROP TABLE IF EXISTS messaging_ingress.direct_conversations;

CREATE TABLE IF NOT EXISTS messaging_ingress.envelope_replay_records (
    envelope_id UUID NOT NULL,
    sender_account_id UUID NOT NULL,
    sender_device_id UUID NOT NULL,
    replay_nonce BYTES NOT NULL,
    reserved_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT envelope_replay_records_pkey PRIMARY KEY (envelope_id),
    CONSTRAINT envelope_replay_records_sender_device_replay_nonce_key UNIQUE (
        sender_account_id,
        sender_device_id,
        replay_nonce
    )
);

CREATE TABLE IF NOT EXISTS messaging_ingress.accepted_envelopes (
    envelope_id UUID NOT NULL,
    sender_account_id UUID NOT NULL,
    sender_device_id UUID NOT NULL,
    recipient_account_id UUID NOT NULL,
    recipient_device_id UUID NOT NULL,
    encrypted_payload BYTES NOT NULL,
    content_type STRING NOT NULL,
    client_timestamp TIMESTAMPTZ NOT NULL,
    replay_nonce BYTES NOT NULL,
    payload_version STRING NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL,
    recipient_bundle_id UUID NOT NULL,
    recipient_signed_prekey_id UUID NOT NULL,
    claimed_one_time_prekey_id UUID NULL,
    prekey_resolution_mode STRING NOT NULL,
    CONSTRAINT accepted_envelopes_pkey PRIMARY KEY (envelope_id),
    CONSTRAINT accepted_envelopes_envelope_id_fk
        FOREIGN KEY (envelope_id)
        REFERENCES messaging_ingress.envelope_replay_records (envelope_id)
);
