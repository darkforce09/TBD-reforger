//! Unit tests for the API freshness probe of `preflight/execution.rs`: a commit under any API
//! source folder, the API crates included, is newer code the running API must not predate.

use super::*;
use std::process::Command;

/// A throwaway repository under the temp folder, removed on drop.
struct Repository(PathBuf);

impl Repository {
    fn new(name: &str) -> Repository {
        let root = std::env::temp_dir().join(format!(
            "preflight-api-freshness-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture root");
        let repository = Repository(root);
        repository.git(&["init", "--quiet", "--initial-branch=main"], 0);
        repository
    }

    fn git(&self, args: &[&str], epoch: i64) {
        let date = format!("@{epoch} +0000");
        let status = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&self.0)
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?}");
    }

    /// Commit one file at `relative`, dated `epoch`.
    fn commit(&self, relative: &str, epoch: i64) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        fs::write(&path, format!("{epoch}\n")).expect("write");
        self.git(&["add", "--", relative], epoch);
        self.git(&["commit", "--quiet", "-m", relative], epoch);
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_commit_under_an_api_crate_is_newer_api_code() {
    let repository = Repository::new("crates");
    repository.commit("apps/api/src/lib.rs", 1_000_000_000);
    repository.commit("crates/api/api_state/src/lib.rs", 2_000_000_000);
    repository.commit("documentation/notes.md", 3_000_000_000);
    assert_eq!(newest_api_commit_epoch(&repository.0), 2_000_000_000);
}

#[test]
fn a_commit_under_the_app_still_counts() {
    let repository = Repository::new("app");
    repository.commit("crates/api/api_state/src/lib.rs", 1_000_000_000);
    repository.commit("apps/api/src/lib.rs", 2_000_000_000);
    assert_eq!(newest_api_commit_epoch(&repository.0), 2_000_000_000);
}

#[test]
fn no_answer_from_git_is_zero() {
    let missing = std::env::temp_dir().join(format!(
        "preflight-api-freshness-{}-missing",
        std::process::id()
    ));
    assert_eq!(newest_api_commit_epoch(&missing), 0);
}
