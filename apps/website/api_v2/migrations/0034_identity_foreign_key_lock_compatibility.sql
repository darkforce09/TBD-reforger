-- Arma ownership is unique when present; only discord_id is a referenced account key.
-- A partial index preserves NULL/non-NULL uniqueness semantics while letting Arma changes
-- use NO KEY UPDATE, compatible with foreign-key KEY SHARE checks during registration.
DROP INDEX idx_users_arma_id;
CREATE UNIQUE INDEX idx_users_arma_id ON users(arma_id) WHERE arma_id IS NOT NULL;
