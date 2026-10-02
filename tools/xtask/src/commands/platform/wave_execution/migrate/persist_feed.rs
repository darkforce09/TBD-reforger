use super::*;

/// Stdin -> psql on `<db>`. Explicit rather than inherited: bash's dynamic scoping would let the
/// helpers read the caller's locals, and a helper whose database depends on who called it is
/// exactly the kind of thing that quietly runs against the wrong one.
pub(super) fn persist_feed(ctx: &Ctx, db: &str, body: &str) -> (String, i32) {
    let mut argv: Vec<String> = Vec::new();
    if ctx.host.bridge {
        argv.push("distrobox-host-exec".into());
    }
    argv.extend(host::v(&[
        "podman",
        "exec",
        "-i",
        "tbd_reforger_db",
        "psql",
        "-U",
        "tbd",
        "-d",
        db,
        "-q",
        "-v",
        "ON_ERROR_STOP=1",
        "-f",
        "-",
    ]));
    super::super::flush();
    let Ok(mut child) = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return (String::new(), 127);
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(body.as_bytes());
    }
    match child.wait_with_output() {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (s, host::status_code(&o.status))
        }
        Err(_) => (String::new(), 127),
    }
}

/// One migration, one transaction — the migration body AND its `_sqlx_migrations` row together, so
/// a migration can never be recorded as applied unless it applied. `rollback` runs the identical
/// transaction and throws it away: used by the slice gate, which must detect without advancing.
pub(super) fn persist_apply_one(ctx: &Ctx, db: &str, f: &Path, finish: &str) -> bool {
    let ver = mig_ver(f);
    let desc = mig_desc(f);
    let sum = sha384(f);
    let src = std::fs::read_to_string(f).unwrap_or_default();
    let mut body = String::from("BEGIN;\n");
    body.push_str(&src);
    body.push('\n');
    body.push_str(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)\n",
    );
    body.push_str(&format!(
        "VALUES ({ver}, '{desc}', true, decode('{sum}','hex'), 0);\n"
    ));
    body.push_str(if finish == "commit" {
        "COMMIT;\n"
    } else {
        "ROLLBACK;\n"
    });
    persist_feed(ctx, db, &body).1 == 0
}

pub(super) fn persist_seed(ctx: &Ctx, db: &str, seed: &str) -> bool {
    let Ok(body) = std::fs::read_to_string(seed) else {
        wprintln!("db_migrate persist: FAIL — seed not found: {seed}");
        return false;
    };
    let (out, rc) = persist_feed(ctx, db, &body);
    if rc != 0 {
        wprintln!(
            "db_migrate persist: FAIL — the committed seed no longer loads into the migrated schema."
        );
        wprintln!(
            "        A migration that makes seeds/content_golden.sql unloadable breaks every fresh"
        );
        wprintln!("        environment, and leaves this persist DB unpopulated for the next wave.");
        let lines: Vec<&str> = out.lines().collect();
        for l in lines.iter().skip(lines.len().saturating_sub(8)) {
            wprintln!("        {l}");
        }
        return false;
    }
    true
}

/// `basename "$1" | sed 's/^0*\([0-9][0-9]*\)_.*/\1/'` — leading zeros stripped, or the basename
/// unchanged when the pattern does not match (sed leaves non-matching lines alone).
pub(super) fn mig_ver(f: &Path) -> String {
    let b = base_name(f);
    let digits: String = b
        .chars()
        .skip_while(|c| *c == '0')
        .take_while(char::is_ascii_digit)
        .collect();
    let consumed = b.chars().take_while(|c| c.is_ascii_digit()).count();
    if consumed > 0 && b.chars().nth(consumed) == Some('_') {
        // `0*` is greedy but `[0-9][0-9]*` needs one digit, so `0016` -> `16` and `000` -> `0`.
        if digits.is_empty() {
            "0".into()
        } else {
            digits
        }
    } else {
        b
    }
}

/// `basename "$1" .sql | sed 's/^[0-9][0-9]*_//; s/_/ /g'`.
pub(super) fn mig_desc(f: &Path) -> String {
    let b = base_name(f);
    let stem = b.strip_suffix(".sql").unwrap_or(&b);
    let n = stem.chars().take_while(char::is_ascii_digit).count();
    let rest = if n > 0 && stem.chars().nth(n) == Some('_') {
        &stem[n + 1..]
    } else {
        stem
    };
    rest.replace('_', " ")
}

pub(super) fn base_name(f: &Path) -> String {
    f.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `sha384sum < "$f" | cut -d' ' -f1`.
///
/// Shelled out rather than reimplemented: the bash asserts the tool is on PATH and FAILS CLOSED
/// when it is not, and that assertion is one of the step's stated anti-vacuity properties. A
/// compiled-in hasher would delete a branch the header promises.
pub(super) fn sha384(f: &Path) -> String {
    let Ok(body) = std::fs::read(f) else {
        return String::new();
    };
    let Ok(mut child) = Command::new("sha384sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    else {
        return String::new();
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(&body);
    }
    let Ok(o) = child.wait_with_output() else {
        return String::new();
    };
    String::from_utf8_lossy(&o.stdout)
        .split(' ')
        .next()
        .unwrap_or("")
        .to_string()
}

pub(super) fn on_path(prog: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()))
        .unwrap_or(false)
}
