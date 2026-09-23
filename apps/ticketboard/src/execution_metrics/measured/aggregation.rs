use super::*;
#[derive(Default)]
pub(super) struct Accumulator {
    runs: u64,
    tokens: u64,
    elapsed: u64,
    finished_runs: u64,
    unfinished: u64,
    /// `(instant, verbatim stamp)` — compared by INSTANT (string order breaks
    /// on fractional seconds), displayed verbatim.
    min_started: Option<(i128, String)>,
    max_finished: Option<(i128, String)>,
}

impl Accumulator {
    fn add(&mut self, run: &LoadedRun) {
        self.runs += 1;
        self.tokens += run.receipt.tokens_consumed.total;
        match run.elapsed {
            Some(secs) => {
                self.elapsed += secs;
                self.finished_runs += 1;
            }
            None => self.unfinished += 1,
        }
        if self
            .min_started
            .as_ref()
            .is_none_or(|(ns, _)| run.started_ns < *ns)
        {
            self.min_started = Some((run.started_ns, run.receipt.started.clone()));
        }
        if let (Some(fin_ns), Some(fin)) = (run.finished_ns, run.receipt.finished.as_ref())
            && self
                .max_finished
                .as_ref()
                .is_none_or(|(ns, _)| fin_ns > *ns)
        {
            self.max_finished = Some((fin_ns, fin.clone()));
        }
    }

    fn into_row(self, key: String) -> MeasuredRow {
        MeasuredRow {
            runs_str: self.runs.to_string(),
            tokens_str: format_tokens(self.tokens),
            elapsed_str: if self.finished_runs == 0 {
                "—".to_owned()
            } else {
                format_elapsed(self.elapsed)
            },
            unfinished_str: self.unfinished.to_string(),
            key,
            runs: self.runs,
            tokens: self.tokens,
            elapsed: self.elapsed,
            finished_runs: self.finished_runs,
            unfinished: self.unfinished,
            min_started: self.min_started.map(|(_, s)| s).unwrap_or_default(),
            max_finished: self.max_finished.map(|(_, s)| s),
        }
    }
}

pub(super) fn rows_of(map: BTreeMap<String, Accumulator>) -> Vec<MeasuredRow> {
    let mut rows: Vec<MeasuredRow> = map
        .into_iter()
        .map(|(key, acc)| acc.into_row(key))
        .collect();
    sort_rows(&mut rows, Sort::default());
    rows
}

pub(super) fn build_model(runs: &[LoadedRun], errors: Vec<ErrorRow>) -> MetricsModel {
    let mut by_ticket: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut by_agent: BTreeMap<String, Accumulator> = BTreeMap::new();
    let mut all = Accumulator::default();
    for run in runs {
        by_ticket
            .entry(run.receipt.id.clone())
            .or_default()
            .add(run);
        by_agent
            .entry(run.receipt.agent.clone())
            .or_default()
            .add(run);
        all.add(run);
    }
    let (tickets, agents) = (by_ticket.len(), by_agent.len());
    let strip = if all.runs == 0 {
        format!(
            "no valid receipts — {} malformed file(s) listed below",
            errors.len()
        )
    } else {
        let elapsed_part = if all.finished_runs == 0 {
            "no finished runs — elapsed unknown".to_owned()
        } else {
            format!(
                "elapsed Σ {} over {} finished",
                format_elapsed(all.elapsed),
                all.finished_runs
            )
        };
        format!(
            "{} run(s) · {} tokens · {elapsed_part} · in flight / unfinished: {} · \
             {tickets} ticket(s) · {agents} agent(s)",
            all.runs,
            format_tokens(all.tokens),
            all.unfinished
        )
    };
    MetricsModel {
        per_ticket: rows_of(by_ticket),
        per_agent: rows_of(by_agent),
        errors,
        grand: Grand {
            runs: all.runs,
            tokens: all.tokens,
            elapsed: all.elapsed,
            finished_runs: all.finished_runs,
            unfinished: all.unfinished,
            tickets,
            agents,
            strip,
        },
    }
}

pub fn sort_rows(rows: &mut [MeasuredRow], sort: Sort) {
    rows.sort_by(|a, b| {
        let ord = match sort.key {
            SortKey::Tokens => a.tokens.cmp(&b.tokens),
            SortKey::Runs => a.runs.cmp(&b.runs),
            SortKey::Elapsed => a.elapsed.cmp(&b.elapsed),
        };
        let ord = if sort.desc { ord.reverse() } else { ord };
        // Deterministic tie-break: key name, ascending, regardless of direction.
        ord.then_with(|| a.key.cmp(&b.key))
    });
}
