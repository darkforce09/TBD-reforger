//! Role: tbdd class r scrub.
//! Position: `formats/density/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

fn mask(src: &[char]) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(src.len());
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut i = 0usize;
    while i < src.len() {
        if src[i] == '/' && src.get(i + 1) == Some(&'/') {
            while i < src.len() && src[i] != '\n' {
                out.push(blank(src[i]));
                i += 1;
            }
            continue;
        }
        if src[i] == '/' && src.get(i + 1) == Some(&'*') {
            let mut depth = 0usize;
            while i < src.len() {
                if src[i] == '/' && src.get(i + 1) == Some(&'*') {
                    depth += 1;
                    out.extend_from_slice(&[' ', ' ']);
                    i += 2;
                } else if src[i] == '*' && src.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    out.extend_from_slice(&[' ', ' ']);
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(src[i]));
                    i += 1;
                }
            }
            continue;
        }
        if let Some(end) = literal_end(src, i) {
            for c in &src[i..end] {
                out.push(blank(*c));
            }
            i = end;
            continue;
        }
        out.push(src[i]);
        i += 1;
    }
    assert_eq!(
        out.len(),
        src.len(),
        "T-935.5: the scrubber mask lost alignment with the source — nothing built on it can \
             be trusted, so this is a hard failure rather than a silent skip"
    );
    out
}

fn literal_end(src: &[char], i: usize) -> Option<usize> {
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    if src[i] == 'r' && (i == 0 || !ident(src[i - 1])) {
        let mut j = i + 1;
        let mut hashes = 0usize;
        while src.get(j) == Some(&'#') {
            hashes += 1;
            j += 1;
        }
        if src.get(j) != Some(&'"') {
            return None;
        }
        j += 1;
        while j < src.len() {
            if src[j] == '"' && src[j + 1..].iter().take(hashes).all(|c| *c == '#') {
                return Some(j + 1 + hashes);
            }
            j += 1;
        }
        return Some(src.len());
    }
    if src[i] == '"' {
        let mut j = i + 1;
        while j < src.len() {
            match src[j] {
                '\\' => j += 2,
                '"' => return Some(j + 1),
                _ => j += 1,
            }
        }
        return Some(src.len());
    }
    if src[i] == '\'' {
        let is_char = src.get(i + 1) == Some(&'\\') || src.get(i + 2) == Some(&'\'');
        if !is_char {
            return None;
        }
        let mut j = i + 1;
        while j < src.len() {
            match src[j] {
                '\\' => j += 2,
                '\'' => return Some(j + 1),
                _ => j += 1,
            }
        }
        return Some(src.len());
    }
    None
}

/// The shipped half of `src`: comments and literals blanked, every `#[cfg(test)]` item cut. Length and line numbers are preserved throughout (everything removed becomes spaces).
pub(crate) fn live_source(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let masked = mask(&chars);
    let mut out = masked.clone();
    let needle: Vec<char> = "#[cfg(test)]".chars().collect();
    let mut i = 0usize;
    while i + needle.len() <= masked.len() {
        if masked[i..i + needle.len()] != needle[..] {
            i += 1;
            continue;
        }
        let mut brackets = 0usize;
        let mut parentheses = 0usize;
        let Some(open) = (i + needle.len()..masked.len()).find(|&k| {
            match masked[k] {
                '[' => brackets += 1,
                ']' => brackets = brackets.saturating_sub(1),
                '(' => parentheses += 1,
                ')' => parentheses = parentheses.saturating_sub(1),
                '{' | ';' if brackets == 0 && parentheses == 0 => return true,
                _ => {}
            }
            false
        }) else {
            break;
        };
        if masked[open] == ';' {
            for slot in &mut out[i..=open] {
                if *slot != '\n' {
                    *slot = ' ';
                }
            }
            i = open + 1;
            continue;
        }
        let mut depth = 0usize;
        let mut close = masked.len();
        for (k, ch) in masked.iter().enumerate().skip(open) {
            if *ch == '{' {
                depth += 1;
            } else if *ch == '}' {
                depth -= 1;
                if depth == 0 {
                    close = k;
                    break;
                }
            }
        }
        let last = close.min(out.len() - 1);
        for slot in &mut out[i..=last] {
            if *slot != '\n' {
                *slot = ' ';
            }
        }
        i = close + 1;
    }
    out.into_iter().collect()
}

#[test]
fn out_of_line_test_modules_do_not_hide_following_production() {
    let src = "pub fn before() {}\n#[cfg(test)]\n#[path = \"tests/reference.rs\"]\nmod reference;\npub fn after() {}\n#[cfg(test)]\nfn helper(a: [u8; 3]) { let _ = a; }";
    let live = live_source(src);
    assert_eq!(live.chars().count(), src.chars().count());
    assert!(live.contains("pub fn before() {}"));
    assert!(live.contains("pub fn after() {}"));
    assert!(!live.contains("mod reference"));
    assert!(!live.contains("fn helper"));
}
