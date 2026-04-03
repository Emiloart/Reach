CREATE SCHEMA IF NOT EXISTS messaging_ingress;

CREATE TABLE IF NOT EXISTS messaging_ingress.direct_conversations (
    conversation_id UUID PRIMARY KEY,
    participant_a_account_id UUID NOT NULL,
    participant_b_account_id UUID NOT NULL,
    state STRING NOT NULL,
    default_expire_after_seconds INT4 NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT direct_conversations_participants_unique UNIQUE (
        participant_a_account_id,
        participant_b_account_id
    ),
    CONSTRAINT direct_conversations_participant_order_chk CHECK (
        participant_a_account_id < participant_b_account_id
    )
);

CREATE TABLE IF NOT EXISTS messaging_ingress.message_intake (
    message_id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES messaging_ingress.direct_conversations (conversation_id),
    sender_account_id UUID NOT NULL,
    sender_device_id UUID NOT NULL,
    client_message_id UUID NOT NULL,
    ciphertext_size_bytes INT4 NOT NULL,
    server_received_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS message_intake_conversation_received_at_idx
    ON messaging_ingress.message_intake (conversation_id, server_received_at);

CREATE TABLE IF NOT EXISTS messaging_ingress.idempotency_keys (
    idempotency_key BYTES PRIMARY KEY,
    message_id UUID NOT NULL REFERENCES messaging_ingress.message_intake (message_id),
    created_at TIMESTAMPTZ NOT NULL
);
