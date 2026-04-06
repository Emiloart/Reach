CREATE UNIQUE INDEX IF NOT EXISTS signed_prekeys_current_device_idx
    ON keys.signed_prekeys (device_id)
    WHERE superseded_at IS NULL;
