//! Interpolation.

use super::*;
use anyhow::Context;

/// A dated anchor: earliest/latest known instants for one ticket, from measured
/// on-disk stamps first, else method-1 subject dates. Interpolated values never
/// enter this map.
#[derive(Debug, Clone, Copy)]
pub(super) struct Anchor {
    pub(super) earliest: OffsetDateTime,
    pub(super) latest: OffsetDateTime,
}

pub(super) fn build_anchors(
    corpus: &Corpus,
    subjects: &BTreeMap<String, Vec<SubjectCommit>>,
) -> Result<BTreeMap<String, Anchor>> {
    let mut anchors = BTreeMap::new();
    for (id, t) in &corpus.tickets {
        let (created, completed) = stamps_of(t);
        let pair = match (created, completed) {
            (Some(c), Some(d)) => Some((c, d)),
            (Some(c), None) => Some((c, c)),
            (None, Some(d)) => Some((d, d)),
            (None, None) => subjects
                .get(id)
                .and_then(|s| Some((s.first()?.date_utc.as_str(), s.last()?.date_utc.as_str()))),
        };
        if let Some((lo, hi)) = pair {
            anchors.insert(
                id.clone(),
                Anchor {
                    earliest: parse_utc(lo).with_context(|| format!("{id} anchor"))?,
                    latest: parse_utc(hi).with_context(|| format!("{id} anchor"))?,
                },
            );
        }
    }
    Ok(anchors)
}

/// Method-2 result: the date pair plus the human derivation for `estimate_note`.
pub(super) struct Interp {
    pub(super) created: String,
    pub(super) completed: String,
    pub(super) desc: String,
}

pub(super) fn strip_last_segment(id: &str) -> Option<String> {
    id.rsplit_once('.').map(|(head, _)| head.to_string())
}

/// Resolve id_interpolation dates for `id` (which has no subjects). See the module
/// header for THE RULE. `parent_fields` maps every corpus id to its `parent` key
/// (works only; programs interpolate on the numeric tier directly).
pub(super) fn method2_dates(
    id: &str,
    parent_fields: &BTreeMap<String, Option<String>>,
    anchors: &BTreeMap<String, Anchor>,
    parent_tier: &[(u64, String)],
) -> Result<Interp> {
    let mut cur = id.to_string();
    let mut walked: Vec<String> = Vec::new();
    loop {
        if let Some(a) = anchors.get(&cur) {
            // First hop: the ticket's own partial on-disk stamps. Later hops: the
            // nearest dated ancestor.
            let desc = if walked.is_empty() {
                "from this ticket's own partial stamps (day precision)".to_string()
            } else {
                format!("from parent {cur} (day precision)")
            };
            return Ok(Interp {
                created: day_floor(a.earliest),
                completed: day_floor(a.latest),
                desc,
            });
        }
        if is_parent_id(&cur) {
            let n = parent_numeric_id(&cur).expect("parent-shaped id has a numeral");
            let below = parent_tier.iter().rev().find(|(m, _)| *m < n);
            let above = parent_tier.iter().find(|(m, _)| *m > n);
            let via = if walked.is_empty() {
                String::new()
            } else {
                format!("via parent {cur} ")
            };
            // One-sided derivations honestly say so — no "midpoint" where none
            // was computed.
            let (day, how) = match (below, above) {
                (Some((_, lo)), Some((_, hi))) => {
                    let a = anchors[lo].latest;
                    let b = anchors[hi].earliest;
                    (
                        day_floor(a + (b - a) / 2),
                        format!("between {lo} and {hi} (midpoint, day precision)"),
                    )
                }
                (Some((_, lo)), None) => (
                    day_floor(anchors[lo].latest),
                    format!("from nearest dated neighbor {lo} (one-sided, day precision)"),
                ),
                (None, Some((_, hi))) => (
                    day_floor(anchors[hi].earliest),
                    format!("from nearest dated neighbor {hi} (one-sided, day precision)"),
                ),
                (None, None) => bail!("{id}: no dated ticket exists to interpolate against"),
            };
            return Ok(Interp {
                created: day.clone(),
                completed: day,
                desc: format!("id-interpolated {via}{how}"),
            });
        }
        let parent = parent_fields
            .get(&cur)
            .cloned()
            .flatten()
            .or_else(|| strip_last_segment(&cur))
            .with_context(|| format!("{id}: dotted id {cur} has no derivable parent"))?;
        if parent == cur || walked.contains(&parent) {
            bail!("{id}: parent chain cycles at {parent}");
        }
        walked.push(parent.clone());
        cur = parent;
    }
}
