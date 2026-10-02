-- Identity locks serialize ownership changes with gameplay attribution.
CREATE TABLE arma_identity_serialization (
    arma_id text PRIMARY KEY CHECK (arma_id <> '' AND arma_id = btrim(arma_id))
);

ALTER TABLE identity_link_codes ADD COLUMN cancelled_at timestamptz;
ALTER TABLE identity_link_codes ADD COLUMN cancellation_reason text;

UPDATE identity_link_codes SET cancelled_at = clock_timestamp(), cancellation_reason = 'expired'
WHERE consumed_at IS NULL AND expires_at <= clock_timestamp();

WITH ranked AS (
    SELECT code, row_number() OVER (PARTITION BY discord_id ORDER BY created_at DESC, code DESC) AS position
    FROM identity_link_codes WHERE consumed_at IS NULL AND cancelled_at IS NULL
)
UPDATE identity_link_codes c SET cancelled_at = clock_timestamp(), cancellation_reason = 'superseded'
FROM ranked r WHERE c.code = r.code AND r.position > 1;

CREATE UNIQUE INDEX identity_link_codes_one_pending_per_account
    ON identity_link_codes(discord_id) WHERE consumed_at IS NULL AND cancelled_at IS NULL;

ALTER TABLE identity_link_codes ADD CONSTRAINT identity_link_code_terminal_state
    CHECK (consumed_at IS NULL OR cancelled_at IS NULL);
ALTER TABLE identity_link_codes ADD CONSTRAINT identity_link_code_cancellation_reason
    CHECK ((cancelled_at IS NULL) = (cancellation_reason IS NULL));
