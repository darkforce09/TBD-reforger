-- Credentials without an authenticated issuance provenance require a new sign-in.
-- A migrated refresh row uses its own UUID as the parent session identity.
UPDATE authentication_sessions s SET revoked_at = COALESCE(s.revoked_at, now())
WHERE EXISTS (SELECT 1 FROM refresh_tokens r WHERE r.id = s.id AND r.session_id = s.id);
UPDATE refresh_tokens r SET revoked_at = COALESCE(r.revoked_at, now())
FROM authentication_sessions s WHERE r.session_id = s.id AND s.revoked_at IS NOT NULL;
