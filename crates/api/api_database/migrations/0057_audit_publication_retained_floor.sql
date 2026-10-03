-- The retained floor of the audit publication sequence: every sequence at or below
-- `retained_after_sequence` may be missing from `audit_publications`, and every sequence above it
-- up to `last_sequence` is still present. A reader resuming from a cursor below the floor cannot
-- replay the history it missed and restarts from the tail instead.
ALTER TABLE public.audit_publication_state
    ADD COLUMN retained_after_sequence bigint NOT NULL DEFAULT 0
        CHECK (retained_after_sequence >= 0);

-- The floor starts at the highest sequence already missing at or below `last_sequence`, or 0 when
-- none is. That sequence is either `last_sequence` itself or one below a present sequence, so
-- those are the only candidates to test.
UPDATE public.audit_publication_state AS state
SET retained_after_sequence = COALESCE((
    SELECT max(candidate.sequence)
    FROM (
        SELECT state.last_sequence AS sequence
        UNION ALL
        SELECT published.sequence - 1
        FROM public.audit_publications AS published
        WHERE published.sequence <= state.last_sequence
    ) AS candidate
    WHERE candidate.sequence > 0
      AND NOT EXISTS (
          SELECT 1 FROM public.audit_publications AS present
          WHERE present.sequence = candidate.sequence
      )
), 0)
WHERE state.singleton;

-- A deleted publication, directly or through the cascade from its audit row, raises the floor to
-- the highest sequence the statement removed. The floor never falls, and a statement that removes
-- nothing above it leaves the singleton row unlocked.
CREATE FUNCTION public.raise_audit_publication_floor_after_delete() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE public.audit_publication_state AS state
    SET retained_after_sequence = removed.highest_sequence
    FROM (SELECT max(sequence) AS highest_sequence FROM removed_publications) AS removed
    WHERE state.singleton
      AND removed.highest_sequence > state.retained_after_sequence;
    RETURN NULL;
END
$$;

CREATE TRIGGER raise_audit_publication_floor_after_delete
    AFTER DELETE ON public.audit_publications
    REFERENCING OLD TABLE AS removed_publications
    FOR EACH STATEMENT EXECUTE FUNCTION public.raise_audit_publication_floor_after_delete();

-- A truncate removes every published sequence, so the floor becomes `last_sequence`.
CREATE FUNCTION public.raise_audit_publication_floor_after_truncate() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE public.audit_publication_state
    SET retained_after_sequence = last_sequence
    WHERE singleton;
    RETURN NULL;
END
$$;

CREATE TRIGGER raise_audit_publication_floor_after_truncate
    AFTER TRUNCATE ON public.audit_publications
    FOR EACH STATEMENT EXECUTE FUNCTION public.raise_audit_publication_floor_after_truncate();
