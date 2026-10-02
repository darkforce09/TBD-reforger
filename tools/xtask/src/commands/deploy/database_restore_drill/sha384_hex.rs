use super::*;

pub(super) fn sha384_hex(path: &Path) -> String {
    let output = Command::new("sha384sum").arg(path).output();
    match output {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.split_whitespace().next().unwrap_or("").to_string()
        }
        _ => String::new(),
    }
}

pub(super) fn psql_scalar(db: &str, sql: &str) -> String {
    let user = db_user();
    match ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.to_string(),
        ],
    ) {
        Ok((_rc, stdout, _stderr)) => stdout.trim().to_string(),
        Err(_) => String::new(),
    }
}

pub(super) fn psql_query(db: &str, sql: &str) -> String {
    // Same as scalar but preserve newlines (ORDER BY version rows).
    let user = db_user();
    match ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            db.to_string(),
            "-tAc".into(),
            sql.to_string(),
        ],
    ) {
        Ok((_rc, stdout, _stderr)) => stdout,
        Err(_) => String::new(),
    }
}

pub(super) fn drop_scratch_db(scratch: &str) -> Result<()> {
    let user = db_user();
    let sql = format!("DROP DATABASE IF EXISTS \"{scratch}\" WITH (FORCE);");
    let _ = ct_capture(
        false,
        &[
            "psql".into(),
            "-U".into(),
            user,
            "-d".into(),
            "postgres".into(),
            "-qc".into(),
            sql,
        ],
    )?;
    Ok(())
}
