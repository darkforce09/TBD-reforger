//! Seatless place holders must stay seatable: every holder must be matchable to a distinct free
//! seat that its policy admits (Hall's condition for a bipartite graph).
//!
//! Holders are offered to the matching in priority order and augmenting paths only reassign
//! seats among holders already matched. The holders that a matching of the earlier holders cannot
//! absorb are therefore exactly those that must yield: the matched set is the priority-greedy
//! basis of the transversal matroid, so no earlier holder is ever displaced by a later one.

/// `eligible[holder][seat]` states whether that holder's policy admits that free seat.
/// Rows are in priority order; every row has the same length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatEligibility {
    seat_count: usize,
    rows: Vec<Vec<bool>>,
}

impl SeatEligibility {
    pub fn new(seat_count: usize) -> Self {
        Self {
            seat_count,
            rows: Vec::new(),
        }
    }

    pub fn seat_count(&self) -> usize {
        self.seat_count
    }

    pub fn holder_count(&self) -> usize {
        self.rows.len()
    }

    /// Append the next holder in priority order.
    pub fn push_holder(&mut self, admits: impl Fn(usize) -> bool) {
        self.rows.push((0..self.seat_count).map(admits).collect());
    }

    pub fn admits(&self, holder: usize, seat: usize) -> bool {
        self.rows[holder][seat]
    }

    /// Holder → seat assignment. `None` marks a holder no matching of higher-priority holders
    /// can absorb.
    pub fn priority_matching(&self) -> Vec<Option<usize>> {
        let mut seat_holder: Vec<Option<usize>> = vec![None; self.seat_count];
        for holder in 0..self.rows.len() {
            let mut visited = vec![false; self.seat_count];
            self.augment(holder, &mut visited, &mut seat_holder);
        }
        let mut assignment = vec![None; self.rows.len()];
        for (seat, holder) in seat_holder.into_iter().enumerate() {
            if let Some(holder) = holder {
                assignment[holder] = Some(seat);
            }
        }
        assignment
    }

    /// Every holder can be seated simultaneously.
    pub fn all_holders_seatable(&self) -> bool {
        self.priority_matching().iter().all(Option::is_some)
    }

    /// Holders that must yield, in priority order, for the rest to remain seatable.
    pub fn unseatable_holders(&self) -> Vec<usize> {
        self.priority_matching()
            .iter()
            .enumerate()
            .filter_map(|(holder, seat)| seat.is_none().then_some(holder))
            .collect()
    }

    /// The holders stay seatable after `seat` is taken by someone who is not a holder.
    pub fn seatable_without(&self, seat: usize) -> bool {
        let mut reduced = Self::new(self.seat_count);
        for row in &self.rows {
            reduced.rows.push(
                row.iter()
                    .enumerate()
                    .map(|(index, admitted)| *admitted && index != seat)
                    .collect(),
            );
        }
        reduced.all_holders_seatable()
    }

    /// Kuhn's augmenting path search; iteration order keeps the result deterministic.
    fn augment(
        &self,
        holder: usize,
        visited: &mut [bool],
        seat_holder: &mut [Option<usize>],
    ) -> bool {
        for seat in 0..self.seat_count {
            if !self.rows[holder][seat] || visited[seat] {
                continue;
            }
            visited[seat] = true;
            let free = match seat_holder[seat] {
                None => true,
                Some(current) => self.augment(current, visited, seat_holder),
            };
            if free {
                seat_holder[seat] = Some(holder);
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
#[path = "tests/seat_matching.rs"]
mod tests;
