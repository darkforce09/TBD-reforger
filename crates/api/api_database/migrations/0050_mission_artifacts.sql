-- A mission artifact is the compiled mod document of one mission version together with every
-- input that determined it. It is written once and never changed: reviews, approvals,
-- deployments, game runtimes and roster derivation all refer to the same bytes.
CREATE TABLE public.mission_artifacts (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_id uuid NOT NULL REFERENCES public.missions(id) ON DELETE RESTRICT,
    mission_version_id uuid NOT NULL REFERENCES public.mission_versions(id) ON DELETE RESTRICT,
    version_payload_sha256 text NOT NULL CHECK (version_payload_sha256 ~ '^[0-9a-f]{64}$'),
    metadata jsonb NOT NULL CHECK (jsonb_typeof(metadata) = 'object'),
    metadata_sha256 text NOT NULL CHECK (metadata_sha256 ~ '^[0-9a-f]{64}$'),
    catalog_sha256 text NOT NULL CHECK (catalog_sha256 ~ '^[0-9a-f]{64}$'),
    modpack_id uuid REFERENCES public.modpacks(id) ON DELETE RESTRICT,
    modpack_version text,
    compiler_version text NOT NULL CHECK (length(btrim(compiler_version)) > 0),
    schema_version text NOT NULL CHECK (length(btrim(schema_version)) > 0),
    terrain text NOT NULL CHECK (length(btrim(terrain)) > 0),
    document bytea NOT NULL,
    document_sha256 text NOT NULL CHECK (document_sha256 ~ '^[0-9a-f]{64}$'),
    document_bytes integer NOT NULL CHECK (document_bytes BETWEEN 1 AND 8388608),
    diagnostics jsonb NOT NULL DEFAULT '[]'::jsonb CHECK (jsonb_typeof(diagnostics) = 'array'),
    artifact_digest text NOT NULL UNIQUE CHECK (artifact_digest ~ '^[0-9a-f]{64}$'),
    created_by text NOT NULL REFERENCES public.users(discord_id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK (octet_length(document) = document_bytes),
    CHECK ((modpack_id IS NULL) = (modpack_version IS NULL))
);
CREATE INDEX mission_artifacts_by_mission ON public.mission_artifacts (mission_id, created_at DESC);
CREATE INDEX mission_artifacts_by_version ON public.mission_artifacts (mission_version_id);

CREATE FUNCTION public.refuse_mission_artifact_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'mission artifacts are immutable' USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER mission_artifacts_are_immutable
    BEFORE UPDATE OR DELETE ON public.mission_artifacts
    FOR EACH ROW EXECUTE FUNCTION public.refuse_mission_artifact_change();
