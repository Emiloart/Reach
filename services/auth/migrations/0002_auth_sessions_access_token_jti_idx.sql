CREATE UNIQUE INDEX IF NOT EXISTS sessions_access_token_jti_uidx
    ON auth.sessions (access_token_jti);
