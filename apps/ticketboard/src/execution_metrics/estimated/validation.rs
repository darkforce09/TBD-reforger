use super::*;
// ---- validation (the validate_estimate mirror) ----

/// Schema `generated_at` pattern `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$` —
/// second precision, no fraction — restated without a regex engine.
pub(super) fn valid_stamp_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
            .iter()
            .all(|&i| b[i].is_ascii_digit())
}

/// The WIDENED cohort key as display text: present `k=v` parts joined with
/// `" · "`; the empty key renders its documented meaning, never a bare void.
pub fn cohort_key_str(key: &CohortKey) -> String {
    let parts: Vec<String> = [
        ("class", &key.class),
        ("domain", &key.domain),
        ("layer", &key.layer),
    ]
    .iter()
    .filter_map(|(name, v)| v.as_ref().map(|v| format!("{name}={v}")))
    .collect();
    if parts.is_empty() {
        "{} (all diff_loc tickets)".to_owned()
    } else {
        parts.join(" · ")
    }
}

/// Semantic mirror of `ticket_engine::metrics::estimates::validate_estimate` plus
/// the schema patterns jsonschema enforces there. Returns the typed per-source
/// inputs. Deliberately NOT mirrored: `factor == TOKENS_PER_LOC` (each file
/// carries the factor it used and the board renders it; the doc-pin is check's
/// governance rule, not a file-shape rule).
pub(super) fn validate_file(rec: &EstimateFile) -> Result<Source, String> {
    if !valid_ticket_id(&rec.id) {
        return Err(format!(
            "id {:?} does not match the schema pattern ^T-[0-9]+([.][0-9]+)*$",
            rec.id
        ));
    }
    if !valid_stamp_shape(&rec.generated_at) {
        return Err(format!(
            "generated_at {:?} does not match the schema pattern \
             ^[0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}T[0-9]{{2}}:[0-9]{{2}}:[0-9]{{2}}Z$",
            rec.generated_at
        ));
    }
    ticket_engine::validate_rfc3339_utc("generated_at", &rec.generated_at)?;
    if rec.factor == 0 {
        return Err("factor must be >= 1".to_owned());
    }
    match rec.source.as_str() {
        "diff_loc" => {
            let loc = rec
                .loc_changed
                .ok_or("source diff_loc requires loc_changed")?;
            let shas = rec
                .derived_from_shas
                .as_ref()
                .ok_or("source diff_loc requires derived_from_shas")?;
            if shas.is_empty() {
                return Err("derived_from_shas must name at least one subject SHA".to_owned());
            }
            for s in shas {
                if !valid_git_sha(s) {
                    return Err(format!(
                        "derived_from_shas entry {s:?} is not 7-40 lowercase hex"
                    ));
                }
            }
            if rec.cohort.is_some() || rec.cohort_size.is_some() {
                return Err("source diff_loc carries no cohort fields".to_owned());
            }
            let expect = loc
                .checked_mul(rec.factor)
                .ok_or("loc_changed x factor overflow")?;
            if rec.tokens_estimated != expect {
                return Err(format!(
                    "tokens_estimated ({}) != loc_changed ({loc}) x factor ({}) = {expect}",
                    rec.tokens_estimated, rec.factor
                ));
            }
            Ok(Source::DiffLoc {
                loc_changed: loc,
                shas: shas.len(),
            })
        }
        "cohort_median" => {
            let size = rec
                .cohort_size
                .ok_or("source cohort_median requires cohort_size")?;
            if size == 0 {
                return Err("cohort_size must be >= 1".to_owned());
            }
            let key = rec
                .cohort
                .as_ref()
                .ok_or("source cohort_median requires the cohort key")?;
            if rec.loc_changed.is_some() || rec.derived_from_shas.is_some() {
                return Err("source cohort_median carries no diff_loc fields".to_owned());
            }
            Ok(Source::CohortMedian {
                key: cohort_key_str(key),
                size,
            })
        }
        other => Err(format!("unknown source {other:?} (diff_loc|cohort_median)")),
    }
}
