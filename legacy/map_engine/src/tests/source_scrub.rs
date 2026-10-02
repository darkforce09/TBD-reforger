//! Role: reduce Rust source to the text a build actually compiles.
//! Position: `tests` in the map engine.
//! Signals & state: none; a pure function of the text handed to it.
//! Invariants: comments and string literals are blanked CHARACTER FOR CHARACTER, so line and column
//! positions survive and a token found in the result is a token the compiler sees. A guard that
//! asserts "this file calls X" therefore cannot be satisfied by a mention of X in prose.

pub fn strip_rust_lexical_noise(src: &str) -> String {
    let s: Vec<char> = src.chars().collect();
    let n = s.len();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;

    fn blank(out: &mut String, c: char) {
        out.push(if c == '\n' { '\n' } else { ' ' });
    }
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';

    while i < n {
        let c = s[i];

        if c == '/' && i + 1 < n && s[i + 1] == '/' {
            while i < n && s[i] != '\n' {
                blank(&mut out, s[i]);
                i += 1;
            }
            continue;
        }

        if c == '/' && i + 1 < n && s[i + 1] == '*' {
            let mut depth = 0usize;
            while i < n {
                if s[i] == '/' && i + 1 < n && s[i + 1] == '*' {
                    depth += 1;
                    out.push_str("  ");
                    i += 2;
                } else if s[i] == '*' && i + 1 < n && s[i + 1] == '/' {
                    depth -= 1;
                    out.push_str("  ");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    blank(&mut out, s[i]);
                    i += 1;
                }
            }
            continue;
        }

        {
            let mut j = i;
            if s[j] == 'b' {
                j += 1;
            }
            let mut hashes = 0usize;
            if j < n && s[j] == 'r' {
                let mut k = j + 1;
                while k < n && s[k] == '#' {
                    hashes += 1;
                    k += 1;
                }
                if k < n && s[k] == '"' && (i == 0 || !is_ident(s[i - 1])) {
                    while i <= k {
                        blank(&mut out, s[i]);
                        i += 1;
                    }
                    while i < n {
                        let closes =
                            s[i] == '"' && (1..=hashes).all(|h| i + h < n && s[i + h] == '#');
                        if closes {
                            for _ in 0..=hashes {
                                blank(&mut out, s[i]);
                                i += 1;
                            }
                            break;
                        }
                        blank(&mut out, s[i]);
                        i += 1;
                    }
                    continue;
                }
            }
        }

        {
            let j = if s[i] == 'b' { i + 1 } else { i };
            if j < n && s[j] == '"' && (i == 0 || !is_ident(s[i - 1])) {
                while i <= j {
                    blank(&mut out, s[i]);
                    i += 1;
                }
                while i < n {
                    if s[i] == '\\' {
                        blank(&mut out, s[i]);
                        if i + 1 < n {
                            blank(&mut out, s[i + 1]);
                        }
                        i += 2;
                        continue;
                    }
                    let end = s[i] == '"';
                    blank(&mut out, s[i]);
                    i += 1;
                    if end {
                        break;
                    }
                }
                continue;
            }
        }

        {
            let q = if c == 'b' && i + 1 < n && s[i + 1] == '\'' && (i == 0 || !is_ident(s[i - 1]))
            {
                Some(i + 1)
            } else if c == '\'' {
                Some(i)
            } else {
                None
            };
            if let Some(q) = q {
                let end = if q + 1 < n && s[q + 1] == '\\' {
                    (q + 3..n).find(|&k| s[k] == '\'')
                } else if q + 2 < n && s[q + 2] == '\'' {
                    Some(q + 2)
                } else {
                    None
                };
                if let Some(end) = end {
                    while i <= end {
                        blank(&mut out, s[i]);
                        i += 1;
                    }
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}
