//! Planning.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Method {
    GitSubject,
    IdInterpolation,
}

pub(super) enum ShippedAction {
    Leave,
    Set(String),
    /// The stray's date value leaves the field and no SHA exists — absent + marked.
    ClearMarkedAbsent,
}

pub(super) struct Plan {
    pub(super) id: String,
    pub(super) set_created: Option<String>,
    pub(super) set_completed: Option<String>,
    pub(super) shipped: ShippedAction,
    pub(super) mark: Vec<&'static str>,
    pub(super) note: Vec<String>,
}

/// What one backfill pass did — the printable evidence.
#[derive(Debug, Default)]
pub struct BackfillReport {
    pub shipped_total: usize,
    pub a_git_subject: usize,
    pub b_id_interpolation: usize,
    pub c_already_complete: usize,
    pub created_git: usize,
    pub created_interp: usize,
    pub completed_git: usize,
    pub completed_interp: usize,
    pub completed_stray: usize,
    pub shipped_sha_filled: usize,
    pub shipped_absent_marked: usize,
    /// Preformatted before/after line per date-shaped stray.
    pub strays: Vec<String>,
    /// Non-SHA non-date shipped_at values left untouched (out of mandate).
    pub odd_shipped_untouched: Vec<String>,
    /// Tickets whose final created_at > completed_at (reported, never coerced).
    pub inverted: Vec<String>,
    /// Ids written back — empty means "nothing to do".
    pub changed: Vec<String>,
}

/// The pure pass over a loaded corpus + mined subject map. Mutates shipped tickets
/// in memory; the caller lands the bytes via [`Corpus::write_back`].
pub fn backfill(
    corpus: &mut Corpus,
    subjects: &BTreeMap<String, Vec<SubjectCommit>>,
) -> Result<BackfillReport> {
    let anchors = build_anchors(corpus, subjects)?;
    let mut parent_tier: Vec<(u64, String)> = anchors
        .keys()
        .filter_map(|id| parent_numeric_id(id).map(|n| (n, id.clone())))
        .collect();
    parent_tier.sort();
    let parent_fields: BTreeMap<String, Option<String>> = corpus
        .tickets
        .iter()
        .map(|(id, t)| {
            let p = match t {
                Ticket::Work(w) => w.parent.clone(),
                Ticket::Program(_) => None,
            };
            (id.clone(), p)
        })
        .collect();

    let mut report = BackfillReport::default();
    let mut plans: Vec<Plan> = Vec::new();

    for (id, t) in &corpus.tickets {
        if t.status().name() != StatusName::Shipped {
            continue;
        }
        report.shipped_total += 1;
        let (created, completed) = stamps_of(t);
        let shipped = shipped_sha_of(t);
        let marked = |f: &str| estimated_of(t).iter().any(|e| e == f);
        let stray = shipped.as_deref().is_some_and(is_date_shaped);
        if let Some(v) = shipped.as_deref()
            && !is_sha_shaped(v)
            && !is_date_shaped(v)
        {
            report.odd_shipped_untouched.push(format!("{id} {v:?}"));
        }
        let needs = stray
            || created.is_none()
            || completed.is_none()
            || (shipped.is_none() && !marked("shipped_at"));
        if !needs {
            report.c_already_complete += 1;
            continue;
        }

        let commits = subjects.get(id).map(Vec::as_slice).unwrap_or(&[]);
        let method = if commits.is_empty() {
            Method::IdInterpolation
        } else {
            Method::GitSubject
        };
        match method {
            Method::GitSubject => report.a_git_subject += 1,
            Method::IdInterpolation => report.b_id_interpolation += 1,
        }

        let mut plan = Plan {
            id: id.clone(),
            set_created: None,
            set_completed: None,
            shipped: ShippedAction::Leave,
            mark: Vec::new(),
            note: Vec::new(),
        };

        // Rule 4 — the date-shaped stray resolves FIRST: its value is real
        // bookkeeping and takes completed_at when that slot is free.
        let mut completed_now = completed.map(str::to_string);
        let mut stray_line = String::new();
        if stray {
            let date = shipped.clone().expect("stray implies value");
            stray_line = format!("{id}: shipped_at {date:?}");
            if completed_now.is_none() {
                let v = format!("{date}T00:00:00Z");
                validate_rfc3339_utc("completed_at", &v).map_err(anyhow::Error::msg)?;
                stray_line.push_str(&format!(" -> completed_at {v:?} (estimated)"));
                plan.set_completed = Some(v);
                plan.mark.push("completed_at");
                plan.note.push(format!(
                    "completed_at from stray date-shaped shipped_at {date}"
                ));
                report.completed_stray += 1;
                completed_now = Some(String::new()); // slot taken; miners must not refill
            } else {
                stray_line.push_str(" dropped (completed_at already present)");
                plan.note.push(format!(
                    "stray date-shaped shipped_at {date} dropped — completed_at already present"
                ));
            }
        }

        // Dates: method 1 mines, method 2 interpolates — absent fields only.
        let mut git_fields: Vec<&str> = Vec::new();
        match method {
            Method::GitSubject => {
                if created.is_none() {
                    plan.set_created = Some(commits[0].date_utc.clone());
                    plan.mark.push("created_at");
                    git_fields.push("created_at");
                    report.created_git += 1;
                }
                if completed_now.is_none() {
                    plan.set_completed = Some(commits.last().expect("nonempty").date_utc.clone());
                    plan.mark.push("completed_at");
                    git_fields.push("completed_at");
                    report.completed_git += 1;
                }
            }
            Method::IdInterpolation => {
                if created.is_none() || completed_now.is_none() {
                    let interp = method2_dates(id, &parent_fields, &anchors, &parent_tier)?;
                    let mut fields: Vec<&str> = Vec::new();
                    if created.is_none() {
                        plan.set_created = Some(interp.created.clone());
                        plan.mark.push("created_at");
                        fields.push("created_at");
                        report.created_interp += 1;
                    }
                    if completed_now.is_none() {
                        plan.set_completed = Some(interp.completed.clone());
                        plan.mark.push("completed_at");
                        fields.push("completed_at");
                        report.completed_interp += 1;
                    }
                    plan.note.push(format!(
                        "no subject commits; {} {}",
                        fields.join("/"),
                        interp.desc
                    ));
                }
            }
        }

        // shipped_at: mined SHA where subjects exist; otherwise ABSENT and marked —
        // a SHA is never invented (module header, the S.6 asymmetry).
        if shipped.is_none() || stray {
            match method {
                Method::GitSubject => {
                    let sha = short_sha(&commits.last().expect("nonempty").sha);
                    if stray {
                        stray_line
                            .push_str(&format!("; shipped_at re-mined -> {sha:?} (git_subject)"));
                    }
                    plan.shipped = ShippedAction::Set(sha);
                    plan.mark.push("shipped_at");
                    git_fields.push("shipped_at");
                    report.shipped_sha_filled += 1;
                }
                Method::IdInterpolation => {
                    if stray {
                        plan.shipped = ShippedAction::ClearMarkedAbsent;
                        stray_line.push_str(
                            "; shipped_at -> absent (no subject commits; marked in estimated[])",
                        );
                    }
                    plan.mark.push("shipped_at");
                    plan.note.push(
                        "shipped_at left absent — no subject commits, a SHA is never invented"
                            .to_string(),
                    );
                    report.shipped_absent_marked += 1;
                }
            }
        }
        if !git_fields.is_empty() {
            plan.note.push(format!(
                "{} git_subject-mined from {} commit subject(s)",
                git_fields.join("/"),
                commits.len()
            ));
        }
        if stray {
            report.strays.push(stray_line);
        }
        plans.push(plan);
    }

    // Apply phase — surgical, validated, dedupe-append markers, compact note.
    for plan in &plans {
        let t = corpus
            .tickets
            .get_mut(&plan.id)
            .expect("planned id is in the corpus");
        apply_plan(t, plan)?;
        let (c, d) = stamps_of(t);
        if let (Some(c), Some(d)) = (c, d)
            && c > d
        {
            report.inverted.push(format!("{} ({c} > {d})", plan.id));
        }
        report.changed.push(plan.id.clone());
    }
    Ok(report)
}
