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
mod tests {
    use super::*;

    fn p() -> Params {
        Params::default()
    }

    #[test]
    fn clean_wall_pairs() {
        let iv = pair_consuming(&[3.40], &[3.55], &p());
        assert_eq!(iv.len(), 1);
        assert!((iv[0].len() - 0.15).abs() < 1e-9);
        assert!(!iv[0].one_sided);
    }

    #[test]
    fn two_walls_with_doorway_do_not_bridge() {
        // fwd faces at 1.0 and 4.0; closing at 1.15 and 4.15; doorway between.
        let iv = pair_consuming(&[1.0, 4.0], &[1.15, 4.15], &p());
        assert_eq!(iv.len(), 2);
        assert!(iv.iter().all(|i| i.len() < 0.2));
    }

    #[test]
    fn consumed_closing_face_cannot_double_pair() {
        // Two forward faces 0.10 apart, ONE closing face: first consumes it, second slivers.
        let iv = pair_consuming(&[1.0, 1.10], &[1.15], &p());
        assert_eq!(iv.len(), 1, "sliver overlaps the pair and merges: {iv:?}");
        assert!((iv[0].a - 1.0).abs() < 1e-9);
        assert!((iv[0].b - 1.19).abs() < 1e-6);
    }

    #[test]
    fn one_sided_forward_and_backward() {
        let iv = pair_consuming(&[2.0], &[9.0], &p());
        assert_eq!(iv.len(), 2);
        assert!(iv[0].one_sided && iv[1].one_sided);
        assert!((iv[0].a - 2.0).abs() < 1e-9);
        assert!((iv[1].b - 9.0).abs() < 1e-9);
    }
}
