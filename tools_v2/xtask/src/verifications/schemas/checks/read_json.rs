use super::*;

pub(super) fn read_json(p: &Path) -> Result<Value> {
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

pub(super) fn schema_root(root: &Path) -> PathBuf {
    root.join("packages/tbd-schema")
}

pub(super) fn spec_dir(root: &Path) -> PathBuf {
    root.join("docs/specs/Mission_Creator_Architecture")
}

/// Print a FAIL header + errors and return exit code 1; or the OK line and 0.
pub(super) fn verdict(name: &str, ok_line: &str, errors: &[String]) -> u8 {
    if errors.is_empty() {
        if ok_line.is_empty() {
            println!("{name}: OK");
        } else {
            println!("{name}: OK {ok_line}");
        }
        0
    } else {
        eprintln!("{name}: FAIL ({})", errors.len());
        for e in errors {
            eprintln!("  {e}");
        }
        1
    }
}
