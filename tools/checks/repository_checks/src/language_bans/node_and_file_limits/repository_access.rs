//! The file-length gate's rendering over the library scan.
//!
//! **Role:** runs [`repository_laws::file_length::scan_file_lengths`] and
//! prints its verdict as `verify file-length` output.
//! **Position:** the bodies behind the parent module's re-exports.
//! **Signals & state:** none; each entry reads the checkout and prints.
//! **Invariants:** a scan that could not run exits 2 and a scan that read no file exits 1, so
//! neither reads as a pass; each over-long file prints one `SIZE-3:` line on stderr and the
//! summary prints on stdout.

use super::*;
use repository_laws::file_length::scan_file_lengths;

/// The body of `cargo xtask verify file-length` over the current checkout.
pub fn verify_file_length() -> Result<u8> {
    Ok(verify_file_length_in(
        &repository_layout::find_repository_root()?,
    ))
}

/// The file-length gate over the checkout at `root`: 0 clean, 1 a file over its ceiling or an
/// empty walk, 2 did not run.
pub(super) fn verify_file_length_in(root: &Path) -> u8 {
    let scan = match scan_file_lengths(root) {
        Ok(scan) => scan,
        Err(cause) => return refuse_file_length(cause),
    };
    if scan.files.is_empty() {
        println!("FAIL: file-length walked 0 source files — refusing a vacuous pass.");
        return 1;
    }
    for violation in &scan.violations {
        eprintln!("{}", violation.rendered());
    }
    println!("{}", scan.summary());
    u8::from(!scan.violations.is_empty())
}

fn refuse_file_length(cause: NotRun) -> u8 {
    let v = Verdict::did_not_run(
        "file-length could not scan the source trees",
        Kind::Ban,
        cause,
    );
    println!("{v}");
    println!("file-length: FAIL (did not run)");
    2
}
