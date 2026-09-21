-- Refresh the stored descriptions of `looks_like_http_url` (0010) and `fk_orphans` (0018) so the
-- catalog comments name the Rust guard and the tests as they stand. Comments only: no data or
-- structure changes, and safe to replay.

COMMENT ON FUNCTION public.looks_like_http_url(text) IS
    'SQL approximation of core::text::http_url_guard::is_http_url. Exact on the scheme and on the '
    'characters that can disguise it; looser on authority shape (accepts http://@, which the '
    'WHATWG parser rejects as an empty host). Never stricter than the Rust guard. Divergences '
    'are pinned by apps/website/api_v2/tests/aar_replay_url_backfill.rs.';

COMMENT ON TABLE public.fk_orphans IS
    'Rows removed or de-pointed so the schema''s foreign keys could be applied (0018, 0019). '
    'row_data is the complete tuple; restore with jsonb_populate_record(NULL::<table>, row_data).';
