-- T-405 — retire the stored `javascript:` payloads T-391 could not reach.
--
-- T-391 shipped `services::text::is_http_url` and enforced it at `upsert_match`, so no NEW
-- non-http(s) value can enter `matches.aar_replay_url` through the API. It could do nothing about
-- rows that were already there, and `frontend/src/deployments.rs` binds that column into an
-- `<a href>` — so until this runs, a pre-guard `javascript:` URL is still one click from executing.
-- T-405 also guards the render sink, which is the belt to this migration's braces: the sink stops
-- a bad value being *rendered*, this stops one being *stored*. Neither makes the other redundant.
--
-- ── QUARANTINE, NOT DELETE ────────────────────────────────────────────────────────────────────
--
-- Every offending value is copied into `url_quarantine` before the column is NULLed. That is what
-- makes it safe for the predicate below to be as strict as it is: the instruction was to be
-- conservative about destroying legitimate data, and the honest way to honour that is to destroy
-- nothing rather than to guess leniently and leave live payloads behind. A false positive here
-- costs one `UPDATE ... FROM url_quarantine` to undo; a false negative costs an XSS.
--
-- The table is deliberately generic (`table_name` / `column_name` / `row_id`). Four more columns
-- share this exact defect and are filed as follow-ups — `announcements.thumbnail_url`,
-- `events.banner_image_url`, `missions.thumbnail_url`, `users.avatar_url` — and each will want to
-- quarantine the same way. One table beats four.
--
-- ── THE PREDICATE, AND WHERE IT DISAGREES WITH THE RUST GUARD ─────────────────────────────────
--
-- `looks_like_http_url` is an APPROXIMATION of `services::text::is_http_url`. It has to be: the
-- Rust guard runs a full WHATWG URL parser, and Postgres has no such thing. The name says
-- "looks like" rather than "is" for that reason. The divergences are enumerated here and PINNED
-- BY A TEST — `apps/website/api/tests/aar_replay_url_backfill.rs` runs this function and the Rust
-- guard over the same shared case table and fails if the disagreement set changes.
--
-- The two are exactly equivalent on the only thing that can execute — THE SCHEME — and on the
-- characters that can make a browser perceive a different scheme than the one stored (ASCII
-- controls, leading/trailing whitespace). They disagree only about the shape of the AUTHORITY,
-- which cannot execute anything. That asymmetry is the design, not an accident:
--
--   SQL KEEPS, RUST REJECTS  (this direction is safe: the value is still an http(s) URL, so it
--                             can only ever be *fetched*; it is also the conservative direction,
--                             since it errs toward leaving data alone)
--     * `http://@`   — userinfo present, host empty. The Rust guard gets `Err(EmptyHost)` from
--     * `https://a@`   the parser. The regex below only asks "is there a character after the
--                       slashes that is not a delimiter", and `@` is such a character. Measured,
--                       not assumed. Also true of any other host the WHATWG parser rejects but
--                       the regex accepts, e.g. `http://[]/` (invalid IPv6) or a host containing
--                       a space. All are `http`; none can run script.
--
--   RUST REJECTS, SQL NEVER SEES
--     * anything containing U+0000. Postgres `text` cannot hold a NUL at all — the server
--       refuses a NUL escape at parse time ("invalid Unicode escape value"), so no such row can
--       exist to be matched. The Rust guard still refuses them, because it sees values before
--       storage. Three cases in the shared table are of this shape; the test skips them for that
--       reason, not because they are uninteresting.
--
--   NOT A DIVERGENCE, BUT WORTH SAYING OUT LOUD
--     * `''` is left alone by the backfill below (see the `<> ''` in the WHERE clause), even
--       though `is_http_url('')` is `false`. Empty is this column's "no replay uploaded yet"
--       sentinel and `upsert_match` preserves it deliberately (`Some("") => Some("")`); it
--       carries no scheme and cannot execute. The backfill matches `upsert_match`'s rule, which
--       is the rule that actually governs this column, rather than the bare predicate's.
--     * `https:///replay.json` is KEPT, and it is not hostless: WHATWG's special-authority-
--       ignore-slashes state eats the third slash, so the host is `replay.json`. Both
--       implementations agree, and both are right. See the shared case table.
--
-- There is no third direction. SQL cannot reject something Rust accepts: Rust accepting implies
-- no leading whitespace and a scheme of `http`/`https`, which implies the string literally starts
-- `http:` or `https:` (any case) — so `^https?:` matches; and it implies a non-empty host, which
-- can only sit after the slash run, so the `[^/\\?#]` character exists. Verified over the whole
-- shared case table by the test named above, not merely argued here.
--
-- ── IDEMPOTENCY ───────────────────────────────────────────────────────────────────────────────
--
-- Safe on a database with zero bad rows (both statements match nothing) and safe to replay by
-- hand: the UPDATE NULLs what it quarantines, so a second pass finds nothing, and the INSERT is
-- `ON CONFLICT DO NOTHING` against a unique key. `CREATE TABLE IF NOT EXISTS` /
-- `CREATE OR REPLACE FUNCTION` are no-ops when already in force. sqlx runs the file in one
-- transaction, so a database either gets all of this or none of it.

CREATE TABLE IF NOT EXISTS public.url_quarantine (
    id             bigserial PRIMARY KEY,
    table_name     text NOT NULL,
    column_name    text NOT NULL,
    row_id         uuid NOT NULL,
    original_value text NOT NULL,
    reason         text NOT NULL,
    ticket         text NOT NULL,
    quarantined_at timestamptz NOT NULL DEFAULT now()
);

-- One quarantine record per (table, column, row). A second quarantine of the same row keeps the
-- FIRST captured value, which is the one that predates anything this platform wrote — the value
-- an incident review would actually want.
CREATE UNIQUE INDEX IF NOT EXISTS idx_url_quarantine_row
    ON public.url_quarantine USING btree (table_name, column_name, row_id);

-- The SQL half of `services::text::is_http_url`. Kept as a FUNCTION rather than inlined into the
-- UPDATE for two reasons: the backfills for the other four columns will reuse it verbatim instead
-- of copy-pasting a regex, and a function is a thing a test can call directly — which is how the
-- divergence list above stays honest instead of aspirational.
CREATE OR REPLACE FUNCTION public.looks_like_http_url(candidate text)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
RETURNS NULL ON NULL INPUT
AS $$
    SELECT
        -- No ASCII control characters. Browsers DELETE tab/CR/LF from anywhere inside a URL
        -- attribute, so `java<TAB>script:` resolves as `javascript:` while satisfying any test
        -- applied to the raw bytes. The range starts at \x01 rather than \x00 because a NUL
        -- cannot be present in a `text` value in the first place (see the divergence notes).
        candidate !~ '[\x01-\x1F\x7F]'
        -- No leading or trailing whitespace, for the same reason: browsers strip it before
        -- parsing, so the stored form and the resolved form must not be able to disagree. The
        -- character set is the full Unicode `White_Space` property — exactly what Rust's
        -- `str::trim()` strips, all 25 codepoints, verified one by one against `chr(n)`. It is
        -- NOT `[[:space:]]`, which is locale-dependent and would silently cover less.
        -- Note U+0085 (NEL) and U+00A0 (NBSP) are here but NOT in the control range above: Rust
        -- catches them via `is_whitespace`, not `is_ascii_control`, and so must this.
        -- Deliberately absent, in both implementations: U+200B ZWSP, U+FEFF BOM and U+00AD soft
        -- hyphen are not `White_Space`, so neither side trims them — they are rejected instead by
        -- the anchor below, because a scheme cannot contain them.
        AND candidate = btrim(
                candidate,
                E'\t\n\u000B\f\r \u0085\u00A0\u1680'
                || E'\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200A'
                || E'\u2028\u2029\u202F\u205F\u3000'
            )
        -- The scheme allowlist plus a non-empty authority, in one anchored match:
        --   `^https?:`   an allowlist of exactly two, ASCII-case-insensitively (`~*`), because a
        --                `javascript:` denylist enumerates an open set and loses to the first
        --                spelling nobody wrote down. This is what refuses `ftp://evil.com/x` and
        --                `javascript://evil.com/%0aalert(1)` — the latter being a real payload
        --                that DOES parse with host `evil.com`, so the host check alone would wave
        --                it through.
        --   `[/\\]*`     any run of slashes and backslashes. WHATWG's special-authority states
        --                consume all of them, so `http:example.com`, `http:/example.com`,
        --                `http://example.com`, `http:///example.com` and `http:/\example.com` all
        --                name the same URL. Being stricter here would NULL rows the Rust guard
        --                accepts — the one direction this migration must never take.
        --   `[^/\\?#]`   at least one character of authority. This is what refuses `http://`,
        --                `https://`, `http://?q` and `http://#f`.
        AND candidate ~* '^https?:[/\\]*[^/\\?#]'
$$;

COMMENT ON FUNCTION public.looks_like_http_url(text) IS
    'T-405. SQL approximation of services::text::is_http_url. Exact on the scheme and on the '
    'characters that can disguise it; looser on authority shape (accepts http://@, which the '
    'WHATWG parser rejects as an empty host). Never stricter than the Rust guard. Divergences '
    'are pinned by apps/website/api/tests/aar_replay_url_backfill.rs.';

INSERT INTO public.url_quarantine
    (table_name, column_name, row_id, original_value, reason, ticket)
SELECT 'matches', 'aar_replay_url', m.id, m.aar_replay_url,
       'scheme is not http/https, or the value carries control characters or edge whitespace',
       'T-405'
FROM public.matches m
WHERE m.aar_replay_url IS NOT NULL
  AND m.aar_replay_url <> ''
  AND NOT public.looks_like_http_url(m.aar_replay_url)
ON CONFLICT (table_name, column_name, row_id) DO NOTHING;

UPDATE public.matches
   SET aar_replay_url = NULL
 WHERE aar_replay_url IS NOT NULL
   AND aar_replay_url <> ''
   AND NOT public.looks_like_http_url(aar_replay_url);
