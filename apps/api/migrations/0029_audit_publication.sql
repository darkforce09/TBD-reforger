-- Audit row identifiers identify facts; publication sequences order durable deliveries.
-- Pending entries are visible to publishers only after their audit transaction commits.
CREATE TABLE public.audit_publication_pending (
    audit_id bigint PRIMARY KEY REFERENCES public.audit_logs(id) ON DELETE CASCADE
);

-- Publishers lock this singleton before reading pending entries and retain the lock through
-- commit. A later sequence therefore cannot become visible before an earlier sequence.
CREATE TABLE public.audit_publication_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    last_sequence bigint NOT NULL DEFAULT 0 CHECK (last_sequence >= 0)
);

INSERT INTO public.audit_publication_state (singleton, last_sequence) VALUES (true, 0);

CREATE TABLE public.audit_publications (
    sequence bigint PRIMARY KEY CHECK (sequence > 0),
    audit_id bigint NOT NULL UNIQUE REFERENCES public.audit_logs(id) ON DELETE CASCADE
);

CREATE FUNCTION public.enqueue_audit_publication() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO public.audit_publication_pending (audit_id)
    VALUES (NEW.id)
    ON CONFLICT (audit_id) DO NOTHING;
    RETURN NULL;
END
$$;

CREATE TRIGGER enqueue_audit_publication
    AFTER INSERT ON public.audit_logs
    FOR EACH ROW EXECUTE FUNCTION public.enqueue_audit_publication();

-- Existing history enters the same durable publication path as newly committed audit facts.
INSERT INTO public.audit_publication_pending (audit_id)
SELECT audit.id
FROM public.audit_logs AS audit
WHERE NOT EXISTS (
    SELECT 1 FROM public.audit_publications AS published WHERE published.audit_id = audit.id
)
ON CONFLICT (audit_id) DO NOTHING;
