-- A review decides one immutable artifact. Submitting opens a review of the artifact compiled
-- from the current version; approval, conditional approval and rejection each name that
-- artifact, and the decision and its comment commit together. Review comments form one thread
-- per mission, each with its author and the version and artifact it concerns.
ALTER TABLE public.missions
    ADD COLUMN approved_artifact_id uuid REFERENCES public.mission_artifacts(id) ON DELETE RESTRICT;

CREATE TABLE public.mission_reviews (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_id uuid NOT NULL REFERENCES public.missions(id) ON DELETE RESTRICT,
    artifact_id uuid NOT NULL REFERENCES public.mission_artifacts(id) ON DELETE RESTRICT,
    submitted_by text NOT NULL REFERENCES public.users(discord_id),
    submitted_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    state text NOT NULL DEFAULT 'pending' CHECK (state IN (
        'pending', 'approved', 'approved_with_conditions', 'rejected', 'superseded')),
    decided_by text REFERENCES public.users(discord_id),
    decided_at timestamptz,
    CHECK ((state = 'pending') = (decided_at IS NULL)),
    CHECK ((state IN ('approved', 'approved_with_conditions', 'rejected')) = (decided_by IS NOT NULL))
);
CREATE UNIQUE INDEX mission_reviews_one_pending_per_mission
    ON public.mission_reviews (mission_id) WHERE state = 'pending';
CREATE INDEX mission_reviews_by_mission ON public.mission_reviews (mission_id, submitted_at DESC, id);

CREATE TABLE public.mission_review_comments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    mission_id uuid NOT NULL REFERENCES public.missions(id) ON DELETE RESTRICT,
    review_id uuid REFERENCES public.mission_reviews(id) ON DELETE RESTRICT,
    mission_version_id uuid REFERENCES public.mission_versions(id) ON DELETE RESTRICT,
    artifact_id uuid REFERENCES public.mission_artifacts(id) ON DELETE RESTRICT,
    author_id text NOT NULL REFERENCES public.users(discord_id),
    kind text NOT NULL CHECK (kind IN ('comment', 'rejection', 'approval_conditions')),
    body text NOT NULL CHECK (length(btrim(body)) > 0 AND octet_length(body) <= 8000),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK (kind = 'comment' OR review_id IS NOT NULL)
);
CREATE INDEX mission_review_comments_by_mission
    ON public.mission_review_comments (mission_id, created_at, id);
