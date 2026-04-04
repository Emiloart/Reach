ALTER TABLE keys.key_bundles
    ADD CONSTRAINT key_bundles_signed_prekey_fk
    FOREIGN KEY (signed_prekey_id)
    REFERENCES keys.signed_prekeys (signed_prekey_id);
