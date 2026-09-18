use super::*;

#[test]
fn prairielearn_case_insensitive() {
    assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/sam/PrairieLearn/x").is_err());
    assert!(refuse_prairielearn("TBD_SSH_HOST", "sam@PRAIRIELEARN.local").is_err());
    assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/sam/tbd/repo").is_ok());
}

#[test]
fn remote_prefix_rejects_escape_and_outside() {
    assert!(require_tbd_remote_prefix("/home/sam/tbd/../elsewhere").is_err());
    assert!(require_tbd_remote_prefix("/tmp/not-tbd-at-all").is_err());
    assert!(require_tbd_remote_prefix("/home/sam/tbd").is_ok());
    assert!(require_tbd_remote_prefix("/home/sam/tbd/repo///").is_ok());
}

#[test]
fn usage_mentions_dry_run() {
    assert!(USAGE.contains("--dry-run"));
    assert!(USAGE.contains("TBD_REMOTE_DIR"));
}
