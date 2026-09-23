-- Session ownership and revocation are authoritative for access and refresh credentials.
CREATE TABLE authentication_sessions (
    id uuid PRIMARY KEY,
    discord_id text NOT NULL REFERENCES users(discord_id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    development_role user_role,
    UNIQUE (id, discord_id)
);
CREATE INDEX authentication_sessions_account ON authentication_sessions(discord_id);
ALTER TABLE refresh_tokens ADD COLUMN session_id uuid;
INSERT INTO authentication_sessions (id, discord_id, created_at, expires_at, revoked_at)
SELECT r.id, r.discord_id, COALESCE(r.created_at, now()), r.expires_at,
       CASE WHEN u.is_banned OR u.deleted_at IS NOT NULL THEN now() ELSE r.revoked_at END
FROM refresh_tokens r JOIN users u USING (discord_id);
UPDATE refresh_tokens SET session_id = id;
ALTER TABLE refresh_tokens ALTER COLUMN session_id SET NOT NULL;
ALTER TABLE refresh_tokens ADD CONSTRAINT refresh_token_session_owner
    FOREIGN KEY (session_id, discord_id) REFERENCES authentication_sessions(id, discord_id)
    ON DELETE CASCADE;
CREATE INDEX refresh_tokens_session ON refresh_tokens(session_id);

-- Account mutation already holds the first lock in the account -> session -> token order.
CREATE FUNCTION revoke_unavailable_account_sessions() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.is_banned OR NEW.deleted_at IS NOT NULL THEN
        UPDATE authentication_sessions SET revoked_at = COALESCE(revoked_at, now())
        WHERE discord_id = NEW.discord_id;
        UPDATE refresh_tokens SET revoked_at = COALESCE(revoked_at, now())
        WHERE discord_id = NEW.discord_id;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER revoke_unavailable_account_sessions
AFTER UPDATE OF is_banned, deleted_at ON users
FOR EACH ROW EXECUTE FUNCTION revoke_unavailable_account_sessions();
