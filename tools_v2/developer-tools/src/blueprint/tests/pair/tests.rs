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
