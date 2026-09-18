use super::*;

/// T-946 — a wave that emptied correctly still has rows for oracle 2 to read.
///
/// Before this, `wave_plan_tickets_at` looked only at `[[waves]]`, so a wave whose every
/// ticket had shipped — the repack having frozen its set as a pending `[[emptied]]` entry —
/// produced an EMPTY list, and oracle 2 reported "has NO rows for wave N … it cannot
/// corroborate this boundary". Every gate then demanded `TBD_GATE_BASE_CONFIRM` about a
/// ledger that was in fact recording exactly the wave being closed.
#[test]
fn the_plan_speaks_for_a_wave_that_has_already_emptied() {
    let toml = concat!(
        "version = 1\n",
        "max_concurrent = 3\n",
        "wave_base = 235\n",
        "pack_last = []\n",
        "[owns]\n",
        "[depends_on]\n",
        "[[waves]]\n",
        "n = 0\n",
        "tickets = [\"T-1\"]\n",
        "[[waves]]\n",
        "n = 237\n",
        "tickets = [\"T-9\"]\n",
        "[[emptied]]\n",
        "n = 236\n",
        "tickets = [\"T-300\", \"T-935.1\", \"T-277\"]\n",
    );
    let lock = ticket_engine::wave_lock::parse(toml).expect("parse");
    println!(
        "── open 237 = {:?} · open 236 = {:?} · pending = {:?}",
        lock.tickets_in_wave(237),
        lock.tickets_in_wave(236),
        lock.emptied
            .iter()
            .map(|e| (e.n, e.tickets.clone()))
            .collect::<Vec<_>>()
    );
    // The open-wave view alone is silent about 236 — that is the bug, kept visible here.
    assert!(lock.tickets_in_wave(236).is_empty());
    // The pending record is not silent, and it names the wave that closed.
    let pending: Vec<String> = lock
        .emptied
        .iter()
        .find(|e| e.n == 236)
        .map(|e| e.tickets.clone())
        .unwrap_or_default();
    assert_eq!(pending, vec!["T-300", "T-935.1", "T-277"]);
    // And an open wave still wins for its own label.
    assert_eq!(lock.tickets_in_wave(237), vec!["T-9".to_string()]);
}

#[test]
fn the_four_accepted_suffixes_and_nothing_else() {
    // T-613. Every widening admits a class of English continuation, and each of those is a
    // plausible thing a hurried operator writes about a wave that DID NOT close.
    assert!(wave_close_subject_ok("wave 76 CLOSED"));
    assert!(wave_close_subject_ok("wave 76 CLOSED: five slices"));
    assert!(wave_close_subject_ok("wave 76 CLOSED — five slices"));
    assert!(wave_close_subject_ok("wave 76 CLOSED - five slices"));

    // The exact forgery wave 77's verifier used.
    assert!(!wave_close_subject_ok(
        "wave 76 CLOSED? reopened — reverting T-608 pending re-gate"
    ));
    assert!(!wave_close_subject_ok("wave 76 CLOSED, partially"));
    assert!(!wave_close_subject_ok("wave 76 CLOSED (partially)"));
    assert!(!wave_close_subject_ok("wave 76 CLOSEDish"));
    // The number is validated as digits, so `wave 7x CLOSED` cannot become a boundary either.
    assert!(!wave_close_subject_ok("wave 7x CLOSED"));
    assert!(!wave_close_subject_ok("wave  76 CLOSED"));
    assert!(!wave_close_subject_ok("Wave 76 CLOSED"));
    assert!(!wave_close_subject_ok("wave 76 closed"));
    assert!(!wave_close_subject_ok("re: wave 76 CLOSED"));
}

#[test]
fn the_prefilter_and_the_authority_agree_on_the_delimiters() {
    // The ERE is handed to git, but it must not be able to select a subject the authority
    // rejects for a delimiter reason — that is the T-613 hole in reverse.
    let re = regex::Regex::new(WAVE_CLOSE_MARKER_RE).unwrap();
    for s in [
        "wave 76 CLOSED",
        "wave 76 CLOSED: x",
        "wave 76 CLOSED — x",
        "wave 76 CLOSED - x",
    ] {
        assert!(re.is_match(s), "{s}");
        assert!(wave_close_subject_ok(s), "{s}");
    }
    for s in ["wave 76 CLOSED? x", "wave 76 CLOSEDish"] {
        assert!(!re.is_match(s), "{s}");
        assert!(!wave_close_subject_ok(s), "{s}");
    }
}
