use super::*;

/// glibc `en_AU.UTF-8` `strcoll`, over the ASCII these extractions contain.
///
/// MEASURED 2026-08-12 against `sort` on this host, which is how the committed baseline was
/// captured. Four levels, as glibc implements ISO 14651:
///
/// * **L1** alphanumerics only, case-folded — punctuation and space are *ignorable*. This is why
///   `aab` < `a b`: primary `aab` against primary `ab`.
/// * **L2** accents, constant across ASCII, so absent here.
/// * **L3** case per surviving character, lowercase first (`ab` < `aB` < `Ab` < `AB`).
/// * **L4** the ignored characters as (position, code point), compared **position first** —
///   `a}bc` < `ab c` even though `}` > space. glibc declares this level `position`, so a string
///   that runs out of ignorables sorts **last**: `a b c` < `a bc` < `ab c` < `abc`.
///
/// The trailing byte compare is GNU `sort`'s last-resort `strcmp`. Unreachable for distinct inputs
/// — L1+L3+L4 together reconstruct the string — and kept so a tie cannot become nondeterminism.
pub(super) fn collate_cmp(a: &str, b: &str) -> Ordering {
    fn primary(s: &str) -> Vec<u8> {
        let keep = s.bytes().filter(u8::is_ascii_alphanumeric);
        keep.map(|c| c.to_ascii_lowercase()).collect()
    }
    fn case(s: &str) -> Vec<u8> {
        let keep = s.bytes().filter(u8::is_ascii_alphanumeric);
        keep.map(|c| u8::from(c.is_ascii_uppercase())).collect()
    }
    fn ignorable(s: &str) -> Vec<(usize, u8)> {
        let all = s.bytes().enumerate();
        all.filter(|(_, c)| !c.is_ascii_alphanumeric()).collect()
    }
    let ord = primary(a)
        .cmp(&primary(b))
        .then_with(|| case(a).cmp(&case(b)));
    if ord != Ordering::Equal {
        return ord;
    }
    let (ia, ib) = (ignorable(a), ignorable(b));
    let (mut x, mut y) = (ia.iter(), ib.iter());
    loop {
        match (x.next(), y.next()) {
            (None, None) => return a.as_bytes().cmp(b.as_bytes()),
            // Exhausted sorts LAST — the `position` level, not a prefix comparison.
            (None, Some(_)) => return Ordering::Greater,
            (Some(_), None) => return Ordering::Less,
            (Some(p), Some(q)) if p != q => return p.cmp(q),
            _ => {}
        }
    }
}
