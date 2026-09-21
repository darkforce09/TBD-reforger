use super::*;
use crate::testutil::Scratch;
use std::fs;

fn mk_repo(root: &Path) {
    fs::create_dir_all(root.join(ticket_engine::repository::TICKETS_DIR)).unwrap();
}

#[test]
fn arg_wins_over_ancestor() {
    let s = Scratch::new("arg-wins");
    let repo_a = s.path().join("a");
    mk_repo(&repo_a);
    let repo_b = s.path().join("b");
    mk_repo(&repo_b);
    let cwd = repo_b.join("deep/inside");
    fs::create_dir_all(&cwd).unwrap();
    // cwd sits inside repo_b, but the explicit arg (repo_a) wins.
    let got = resolve_repo_root(Some(repo_a.clone()), Some(&cwd));
    assert_eq!(got, Some(repo_a));
}

#[test]
fn arg_wins_even_when_invalid() {
    let s = Scratch::new("arg-invalid");
    let repo = s.path().join("repo");
    mk_repo(&repo);
    let bogus = s.path().join("not-a-repo");
    fs::create_dir_all(&bogus).unwrap();
    let cwd = repo.join("sub");
    fs::create_dir_all(&cwd).unwrap();
    // No silent fallback to the valid ancestor: the caller refuses instead.
    let got = resolve_repo_root(Some(bogus.clone()), Some(&cwd));
    assert_eq!(got, Some(bogus.clone()));
    assert!(!has_tickets_dir(&bogus));
}

#[test]
fn ancestor_found_from_nested_cwd() {
    let s = Scratch::new("walk-up");
    let repo = s.path().join("repo");
    mk_repo(&repo);
    let cwd = repo.join("apps/ticketboard/src");
    fs::create_dir_all(&cwd).unwrap();
    assert_eq!(resolve_repo_root(None, Some(&cwd)), Some(repo.clone()));
    // The repo root itself also resolves (ancestors() includes self).
    assert_eq!(walk_up_for_tickets(&repo), Some(repo));
}

#[test]
fn nothing_found_is_none() {
    let s = Scratch::new("none");
    let cwd = s.path().join("plain/dir");
    fs::create_dir_all(&cwd).unwrap();
    assert_eq!(resolve_repo_root(None, Some(&cwd)), None);
    assert_eq!(resolve_repo_root(None, None), None);
}

#[test]
fn positional_arg_skips_flags() {
    let args = vec!["--verbose".to_string(), "/some/repo".to_string()];
    assert_eq!(positional_arg(args), Some(PathBuf::from("/some/repo")));
    assert_eq!(positional_arg(Vec::new()), None);
}
