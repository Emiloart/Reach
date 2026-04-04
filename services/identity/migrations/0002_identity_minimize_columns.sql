ALTER TABLE identity.accounts
    DROP COLUMN IF EXISTS suspension_code;

ALTER TABLE identity.devices
    DROP COLUMN IF EXISTS device_label;

ALTER TABLE identity.devices
    DROP COLUMN IF EXISTS attestation_type;

ALTER TABLE identity.devices
    DROP COLUMN IF EXISTS attestation_summary;
