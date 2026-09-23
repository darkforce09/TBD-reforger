use super::*;
#[derive(Default)]
pub(super) struct Accumulator {
    tickets: u64,
    tokens: u64,
    diff_loc: u64,
    cohort_median: u64,
}

impl Accumulator {
    fn add(&mut self, v: &ValidEstimate) {
        self.tickets += 1;
        self.tokens += v.tokens_estimated;
        match v.source {
            Source::DiffLoc { .. } => self.diff_loc += 1,
            Source::CohortMedian { .. } => self.cohort_median += 1,
        }
    }

    fn into_row(self, key: String) -> EstimatedRow {
        EstimatedRow {
            tickets_str: self.tickets.to_string(),
            tokens_str: format_tokens(self.tokens),
            diff_loc_str: self.diff_loc.to_string(),
            cohort_str: self.cohort_median.to_string(),
            key,
            tickets: self.tickets,
            tokens: EstimatedTokens(self.tokens),
            diff_loc: self.diff_loc,
            cohort_median: self.cohort_median,
        }
    }
}

/// Class bucket of the ticket an estimate belongs to. Estimates whose ticket is
/// gone (or classless) bucket under an explicit marker — stated, never guessed.
pub(super) fn class_key(ticket: Option<&Ticket>) -> String {
    match ticket {
        None => "(no ticket file)".to_owned(),
        Some(t) => board::class_of(t).unwrap_or("(no class)").to_owned(),
    }
}

/// Domain bucket: the work ticket's scope domain; programs carry no scope.
pub(super) fn domain_key(ticket: Option<&Ticket>) -> String {
    match ticket {
        None => "(no ticket file)".to_owned(),
        Some(Ticket::Work(w)) => w.scope.domain.as_str().to_owned(),
        Some(Ticket::Program(_)) => "(program)".to_owned(),
    }
}

pub(super) fn rows_of(map: BTreeMap<String, Accumulator>) -> Vec<EstimatedRow> {
    let mut rows: Vec<EstimatedRow> = map
        .into_iter()
        .map(|(key, acc)| acc.into_row(key))
        .collect();
    sort_rows(&mut rows, EstimatedSort::default());
    rows
}

/// Join the validated estimate records against the corpus (class/domain of each
/// estimate's ticket) into the panel model. Pure — the raw load happened on the
/// worker thread; this runs at board build.
pub fn build_state(raw: RawEstimates, corpus: &Corpus) -> EstimatesState {
    if !raw.present || (raw.records.is_empty() && raw.errors.is_empty()) {
        return EstimatesState::NoEstimates;
    }
    let by_ticket: HashMap<&str, &Ticket> = corpus
        .tickets
        .iter()
        .map(|t| (t.ticket.id(), &t.ticket))
        .collect();
    let mut by_id = BTreeMap::new();
    let mut by_class: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut by_domain: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut all = Accumulator::default();
    for rec in &raw.records {
        let ticket = by_ticket.get(rec.id.as_str()).copied();
        by_class.entry(class_key(ticket)).or_default().add(rec);
        by_domain.entry(domain_key(ticket)).or_default().add(rec);
        all.add(rec);
        by_id.insert(rec.id.clone(), EstimateDetail::of(rec));
    }
    let (classes, domains) = (by_class.len(), by_domain.len());
    let strip = if all.tickets == 0 {
        format!(
            "no valid estimate files — {} malformed file(s) listed below",
            raw.errors.len()
        )
    } else {
        format!(
            "{} estimate file(s) · {} tokens (estimated) · sources: {} diff_loc / {} \
             cohort_median · {classes} class(es) · {domains} domain(s)",
            all.tickets,
            format_tokens(all.tokens),
            all.diff_loc,
            all.cohort_median
        )
    };
    EstimatesState::Loaded(EstimatesModel {
        by_id,
        per_class: rows_of(by_class),
        per_domain: rows_of(by_domain),
        errors: raw.errors,
        grand: EstimatedTotals {
            files: all.tickets,
            tokens: EstimatedTokens(all.tokens),
            diff_loc: all.diff_loc,
            cohort_median: all.cohort_median,
            classes,
            domains,
            strip,
        },
    })
}

pub fn sort_rows(rows: &mut [EstimatedRow], sort: EstimatedSort) {
    rows.sort_by(|a, b| {
        let ord = match sort.key {
            EstimatedSortKey::Tokens => a.tokens.cmp(&b.tokens),
            EstimatedSortKey::Tickets => a.tickets.cmp(&b.tickets),
            EstimatedSortKey::DiffLoc => a.diff_loc.cmp(&b.diff_loc),
            EstimatedSortKey::CohortMedian => a.cohort_median.cmp(&b.cohort_median),
        };
        let ord = if sort.desc { ord.reverse() } else { ord };
        ord.then_with(|| a.key.cmp(&b.key))
    });
}
