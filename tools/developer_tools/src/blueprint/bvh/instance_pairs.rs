//! One-sided-collision face pairing. Enfusion collision faces register only when marched into
//! from their front side, so a wall is seen as one entry face in the `+` run and one in the `-`
//! run. Pairing reconstructs solid intervals from the two lists.

use super::params::Params;
use super::types::SolidInterval;

/// Consuming two-pointer pairing (the `segments` default): each closing face pairs AT MOST once,
/// so a distant closing face cannot bridge across a doorway gap that already consumed it — the
/// live non-consuming rule let one far face close several forward faces, a suspected phantom-wall
/// contributor. Unmatched faces on either side become one-sided slivers (live semantics).
pub fn pair_consuming(fwd: &[f64], closing_sorted: &[f64], p: &Params) -> Vec<SolidInterval> {
    let mut used = vec![false; closing_sorted.len()];
    let mut out = Vec::new();

    for &a in fwd {
        let mut best: Option<usize> = None;
        for (i, &b) in closing_sorted.iter().enumerate() {
            if used[i] || b <= a - p.pair_behind_m {
                continue;
            }
            if b - a > p.max_pair_m {
                break;
            }
            best = Some(i);
            break;
        }
        match best {
            Some(i) => {
                used[i] = true;
                let b = closing_sorted[i];
                out.push(SolidInterval {
                    a: a.min(b),
                    b: b.max(a),
                    one_sided: false,
                });
            }
            None => out.push(SolidInterval {
                a,
                b: a + p.sliver_m,
                one_sided: true,
            }),
        }
    }
    for (i, &b) in closing_sorted.iter().enumerate() {
        if !used[i] {
            out.push(SolidInterval {
                a: b - p.sliver_m,
                b,
                one_sided: true,
            });
        }
    }

    out.sort_by(|x, y| x.a.total_cmp(&y.a));
    merge_overlaps(out)
}

fn merge_overlaps(sorted: Vec<SolidInterval>) -> Vec<SolidInterval> {
    let mut out: Vec<SolidInterval> = Vec::with_capacity(sorted.len());
    for iv in sorted {
        match out.last_mut() {
            Some(last) if iv.a <= last.b + 1e-9 => {
                last.b = last.b.max(iv.b);
                last.one_sided = last.one_sided && iv.one_sided;
            }
            _ => out.push(iv),
        }
    }
    out
}

/// Normalize a "-" run (descending) into an ascending closing-face list for the pairers.
pub fn ascending(neg_run: &[f64]) -> Vec<f64> {
    let mut v = neg_run.to_vec();
    v.sort_by(f64::total_cmp);
    v
}

#[cfg(test)]
#[path = "../tests/pair/tests.rs"]
mod tests;
