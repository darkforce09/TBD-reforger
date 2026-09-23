-- A saved mission version is the authored input that artifacts compile from and that reviewers
-- open in the read-only review workspace, so its content never changes after insert. A version
-- an artifact refers to cannot be deleted either: the artifact's foreign key restricts it.
CREATE FUNCTION public.refuse_mission_version_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'mission versions are immutable' USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER mission_versions_are_immutable
    BEFORE UPDATE ON public.mission_versions
    FOR EACH ROW EXECUTE FUNCTION public.refuse_mission_version_change();
