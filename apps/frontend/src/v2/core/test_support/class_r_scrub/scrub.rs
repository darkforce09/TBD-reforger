//! The removal passes themselves, and the four entry points the source pins call.
//!
//! **Role:** cuts the test module, dead `cfg` items, blocks whose condition cannot be proved to
//! run, and everything after an unconditional jump — then hands back what is left.
//! **Position:** the top of the scrub pipeline and the module's public surface.
//! **Signals & state:** none. [`Scrub`] holds two buffers for the duration of one call.
//! **Invariants:** removals blank in both buffers at once, so a later pass cannot see what an
//! earlier one removed and brace balance is preserved. An undecided condition is removed rather
//! than kept: a wrongly removed block costs a loud red pin, a wrongly kept one costs silence over
//! code the build never runs, and those two are not symmetric.

use super::cfg::{call_args, cfg_eval};
use super::consts::{constant_shaped, constants};
use super::expr::eval_bool;
use super::lexer::{balanced, blank, kw_at, mask};

/// Two parallel buffers over one file: the masked copy structure is read from, and the working
/// copy the caller ends up searching.
struct Scrub {
    scan: Vec<char>,
    out: Vec<char>,
}

impl Scrub {
    fn kill(&mut self, range: std::ops::Range<usize>) {
        for k in range {
            if k < self.scan.len() {
                self.scan[k] = blank(self.scan[k]);
                self.out[k] = blank(self.out[k]);
            }
        }
    }

    fn cut_test_module(&mut self) {
        let needle: Vec<char> = "#[cfg(test)]".chars().collect();
        if let Some(at) = find_from(&self.scan, &needle, 0) {
            self.kill(at..self.scan.len());
        }
    }

    fn kill_dead_cfg_items(&mut self) {
        let n = self.scan.len();
        let mut i = 0usize;
        while i < n {
            if self.scan[i] != '#' {
                i += 1;
                continue;
            }
            let mut j = i + 1;
            if self.scan.get(j) == Some(&'!') {
                j += 1;
            }
            if self.scan.get(j) != Some(&'[') {
                i += 1;
                continue;
            }
            let Some(close) = balanced(&self.scan, j, '[', ']') else {
                i += 1;
                continue;
            };
            let inner: String = self.scan[j + 1..close].iter().collect();
            if let Some(pred) = call_args(&inner, "cfg") {
                if cfg_eval(&pred) == Some(false) {
                    let end = item_end_after(&self.scan, close + 1);
                    self.kill(i..end);
                    i = end;
                    continue;
                }
            }
            i = close + 1;
        }
    }

    fn kill_const_false_blocks(&mut self) {
        let consts = constants(&self.scan);
        let n = self.scan.len();
        let mut i = 0usize;
        while i < n {
            let klen = if kw_at(&self.scan, i, "if") {
                2
            } else if kw_at(&self.scan, i, "while") {
                5
            } else {
                i += 1;
                continue;
            };
            let mut j = i + klen;
            let mut d = 0i32;
            let mut stop = None;
            while j < n {
                match self.scan[j] {
                    '(' | '[' => d += 1,
                    ')' | ']' => d -= 1,
                    '{' if d <= 0 => {
                        stop = Some((j, false));
                        break;
                    }
                    '=' if d <= 0 && self.scan.get(j + 1) == Some(&'>') => {
                        stop = Some((j, true));
                        break;
                    }
                    ';' if d <= 0 => break,
                    _ => {}
                }
                j += 1;
            }
            let Some((at, arrow)) = stop else {
                i += klen;
                continue;
            };
            let cond: String = self.scan[i + klen..at].iter().collect();
            let dead = match eval_bool(&cond, &consts) {
                Some(b) => !b,
                // Unknown never means "live" — see the doc comment on this function.
                None => constant_shaped(&cond, &consts),
            };
            if dead {
                let end = if arrow {
                    arm_end(&self.scan, at + 2)
                } else {
                    balanced(&self.scan, at, '{', '}')
                        .map(|e| e + 1)
                        .unwrap_or(n)
                };
                self.kill(i..end);
                i = end;
            } else {
                i += klen;
            }
        }
    }

    fn kill_after_unconditional_jump(&mut self) {
        let n = self.scan.len();
        let mut i = 0usize;
        while i < n {
            let Some(kw) = ["break", "continue", "return"]
                .iter()
                .find(|k| kw_at(&self.scan, i, k))
                .copied()
            else {
                i += 1;
                continue;
            };
            let mut j = i + kw.len();
            while j < n && self.scan[j].is_whitespace() {
                j += 1;
            }
            if self.scan.get(j) != Some(&';') {
                i += kw.len();
                continue;
            }
            j += 1;
            let from = j;
            let mut depth = 0i32;
            while j < n {
                match self.scan[j] {
                    '{' => depth += 1,
                    '}' if depth == 0 => break,
                    '}' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            self.kill(from..j);
            i = j;
        }
    }
}

/// Index of the first occurrence of `needle` in `hay` at or after `from`.
fn find_from(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

/// End (exclusive) of the item an attribute annotates: its balanced `{…}` body, or its `;`.
///
/// Depth-tracked, so the `;` inside `[u8; 3]` is not mistaken for the item terminator.
pub(super) fn item_end_after(scan: &[char], from: usize) -> usize {
    let n = scan.len();
    let mut i = from;
    let mut d = 0i32;
    while i < n {
        match scan[i] {
            '(' | '[' => d += 1,
            ')' | ']' => d -= 1,
            ';' if d <= 0 => return i + 1,
            '{' if d <= 0 => {
                return balanced(scan, i, '{', '}').map(|e| e + 1).unwrap_or(n);
            }
            _ => {}
        }
        i += 1;
    }
    n
}

/// End (exclusive) of the `match` arm body starting at `from`, which is just past the `=>`.
fn arm_end(scan: &[char], from: usize) -> usize {
    let n = scan.len();
    let mut i = from;
    while i < n && scan[i].is_whitespace() {
        i += 1;
    }
    if scan.get(i) == Some(&'{') {
        let end = balanced(scan, i, '{', '}').map(|e| e + 1).unwrap_or(n);
        // an optional trailing comma belongs to the arm
        let mut k = end;
        while k < n && scan[k].is_whitespace() {
            k += 1;
        }
        return if scan.get(k) == Some(&',') {
            k + 1
        } else {
            end
        };
    }
    let mut d = 0i32;
    while i < n {
        match scan[i] {
            '(' | '[' | '{' => d += 1,
            ')' | ']' => d -= 1,
            '}' if d == 0 => return i,
            '}' => d -= 1,
            ',' if d <= 0 => return i + 1,
            _ => {}
        }
        i += 1;
    }
    n
}

/// Run every removal pass over `src` and return what is left. `keep_literals` decides whether
/// string and character literals survive into the result.
fn scrub(src: &str, keep_literals: bool) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut s = Scrub {
        scan: mask(&chars, true),
        out: mask(&chars, !keep_literals),
    };
    s.cut_test_module();
    s.kill_dead_cfg_items();
    s.kill_const_false_blocks();
    s.kill_after_unconditional_jump();
    s.out.into_iter().collect()
}

/// The production half of `src`, with comments and unreachable constructs removed and **string
/// literals kept**.
///
/// A route path, a test id or a line of user-visible copy is code that ships, and pinning one is
/// not the same mistake as pinning a comment.
pub(crate) fn live_source(src: &str) -> String {
    scrub(src, true)
}

/// The same as [`live_source`], with string and character literals blanked as well.
///
/// Use it for assertions that mean "this is a **call**, not a mention", where a needle sitting
/// inside a literal is precisely the decoy.
pub(crate) fn live_code(src: &str) -> String {
    scrub(src, false)
}

/// Locate the **only** item matching `marker` and return `(start, body_open_offset, body_end)`.
///
/// # Panics
///
/// On zero matches — a rename must be new information rather than a silent pass — and on two or
/// more. A second definition of the same name is how a pin gets fed a pristine copy while the real
/// item is cut, and no textual search can tell which one ships, so ambiguity is an error rather
/// than a coin flip.
fn split_only<'a>(src: &'a str, marker: &str) -> (usize, usize, usize) {
    let hits = src.matches(marker).count();
    assert_eq!(
        hits, 1,
        "expected exactly one `{marker}` in the live source, found {hits}. \
         0 means it was renamed or deleted; 2+ means a shadow definition — either way this pin \
         cannot examine code it cannot unambiguously find, so it fails rather than guesses."
    );
    let at = src.find(marker).expect("counted above");
    let tail = &src[at + marker.len()..];
    let open = tail
        .find('{')
        .unwrap_or_else(|| panic!("`{marker}` has no body"));
    let bytes = tail.as_bytes();
    let mut depth = 1usize;
    let mut i = open + 1;
    while i < tail.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    assert_eq!(depth, 0, "`{marker}` body is unbalanced");
    (at + marker.len(), open, i)
}

/// The whole of the **only** item matching `marker`: signature and balanced body.
///
/// Use this when the assertion is about the item's shape — a parameter type, a return type — and
/// not only about what it calls.
pub(crate) fn only_item<'a>(src: &'a str, marker: &str) -> &'a str {
    let (base, _open, end) = split_only(src, marker);
    &src[base - marker.len()..base + end]
}

/// The balanced `{…}` body of the **only** item matching `marker`.
pub(crate) fn only_body<'a>(src: &'a str, marker: &str) -> &'a str {
    let (base, open, end) = split_only(src, marker);
    &src[base + open + 1..base + end - 1]
}
