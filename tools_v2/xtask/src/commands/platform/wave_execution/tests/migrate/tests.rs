use super::*;

#[test]
fn comment_stripping_survives_multiline_blocks_and_line_comments() {
    // Comment-only bait must not false-green the claim-body pin.
    let src = "UPDATE public.match_player_stats AS s\n/* SET discord_id = u.discord_id\n   still commented */\n-- AND s.discord_id IS NULL\nSELECT 1;\n";
    let out = strip_sql_comments(src);
    assert!(out.contains("UPDATE public.match_player_stats AS s"));
    assert!(
        !out.contains("SET discord_id = u.discord_id"),
        "block comment leaked: {out}"
    );
    assert!(
        !out.contains("AND s.discord_id IS NULL"),
        "line comment leaked: {out}"
    );
    assert!(out.contains("SELECT 1;"));
}

#[test]
fn a_line_comment_inside_a_block_does_not_end_it() {
    let out = strip_sql_comments("A /* x -- y\nz */ B\n");
    assert!(out.contains('A') && out.contains('B'));
    assert!(!out.contains('z'));
}

#[test]
fn migration_version_and_description_match_the_sed_pipeline() {
    let p = PathBuf::from(
        "apps/website/api_v2/migrations/0016_backfill_pre_t326_linked_match_stats.sql",
    );
    assert_eq!(mig_ver(&p), "16");
    assert_eq!(mig_desc(&p), "backfill pre t326 linked match stats");
}
