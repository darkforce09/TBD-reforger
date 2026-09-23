use super::*;
// ---- detail-panel models (pure) ----

/// Detail-row rendering of one lifecycle stamp (`created_at` / `completed_at` /
/// `shipped_at`): measured and estimated are DISTINCT states, decided by the
/// ticket's `estimated[]` list. Tooltips carry the `estimate_note` VERBATIM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StampCell {
    /// Value present, not listed in `estimated[]` — no glyph, no tooltip.
    Measured(String),
    /// Value present and listed in `estimated[]` — the [`ESTIMATE_GLYPH`] +
    /// note tooltip.
    Estimated { value: String, tip: String },
    /// Listed in `estimated[]` with NO value on disk (the honest shipped_at
    /// gap: a SHA is never invented) — [`ABSENT_ESTIMATED_MARKER`] + tooltip.
    AbsentEstimated { tip: String },
    /// Absent and unmarked — the plain muted em-dash.
    Absent,
}

/// The glyph predicate: is `name` listed in the ticket's `estimated[]`?
pub fn stamp_estimated(name: &str, estimated: &[String]) -> bool {
    estimated.iter().any(|e| e == name)
}

/// Project one stamp into its render state. `note` is the ticket's
/// `estimate_note`, rendered verbatim as the tooltip (never re-derived); its
/// absence gets the explicit [`NOTE_ABSENT_TIP`].
pub fn stamp_cell(
    name: &str,
    value: Option<&str>,
    estimated: &[String],
    note: Option<&str>,
) -> StampCell {
    let marked = stamp_estimated(name, estimated);
    let tip = || note.map_or_else(|| NOTE_ABSENT_TIP.to_owned(), str::to_owned);
    match (value, marked) {
        (Some(v), false) => StampCell::Measured(v.to_owned()),
        (Some(v), true) => StampCell::Estimated {
            value: v.to_owned(),
            tip: tip(),
        },
        (None, true) => StampCell::AbsentEstimated { tip: tip() },
        (None, false) => StampCell::Absent,
    }
}

/// Precomputed strings of one estimate for the detail panel: value, source,
/// factor and inputs — ALL from the estimate file, never re-derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstimateDetail {
    /// `"252,900"`.
    pub value_str: String,
    /// `"diff_loc"` / `"cohort_median"`.
    pub source: String,
    pub factor: u64,
    /// `"1,686 LOC over 2 sha(s)"` / `"cohort class=feature · n=211"`.
    pub inputs_str: String,
    /// The one-line row: `"252,900 · diff_loc ×150 · 1,686 LOC over 2 sha(s)"`.
    pub row_str: String,
    /// Full tooltip — source, factor, inputs, generated_at.
    pub tip: String,
}

impl EstimateDetail {
    pub(super) fn of(v: &ValidEstimate) -> Self {
        let value_str = format_tokens(v.tokens_estimated);
        let inputs_str = match &v.source {
            Source::DiffLoc { loc_changed, shas } => {
                format!("{} LOC over {shas} sha(s)", format_tokens(*loc_changed))
            }
            Source::CohortMedian { key, size } => format!("cohort {key} · n={size}"),
        };
        let source = v.source.as_str().to_owned();
        let row_str = format!("{value_str} · {source} ×{} · {inputs_str}", v.factor);
        let tip = format!(
            "estimated tokens — source {source} · factor ×{} · {inputs_str} · generated {}",
            v.factor, v.generated_at
        );
        Self {
            value_str,
            source,
            factor: v.factor,
            inputs_str,
            row_str,
            tip,
        }
    }
}

/// Detail-row rendering of the "tokens (estimated)" row. `None` = `"tokens"`
/// not listed in `estimated[]` — NO row at all (measured tokens live on the
/// Metrics tab over receipts; the detail panel never mixes the two).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokensCell {
    /// Marked + a valid estimate file loaded.
    Estimated(EstimateDetail),
    /// Marked but no valid file — an explicit hole, never an invented number.
    MissingFile { tip: String },
}

pub fn tokens_cell(
    id: &str,
    estimated: &[String],
    model: Option<&EstimatesModel>,
) -> Option<TokensCell> {
    if !stamp_estimated("tokens", estimated) {
        return None;
    }
    Some(match model.and_then(|m| m.by_id.get(id)) {
        Some(detail) => TokensCell::Estimated(detail.clone()),
        None => TokensCell::MissingFile {
            tip: format!(
                "\"tokens\" is listed in estimated[] but no valid estimate file loaded at \
                 {}/{id}.json (missing or malformed — see the Metrics tab)",
                ticket_engine::repository::ESTIMATES_DIR
            ),
        },
    })
}
