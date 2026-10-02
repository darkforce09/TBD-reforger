//! What one occurrence of a path spelling in a file is: a repository-root path, a relative path, a
//! segment behind another path, a URL, or no path at all.
//!
//! **Role:** the shared byte-level vocabulary of the path passes: which bytes a path segment holds,
//! where a path token starts and ends, and [`classify_occurrence`], which tells the repository-root
//! spelling pass and the retired-spelling verification apart the same way.
//!
//! **Position:** used by [`super::root_spellings`], [`super::relative_references`] and
//! [`crate::commands::refactor::relocate::retired_spellings`].
//!
//! **Signals & state:** none; pure functions over a source string and byte offsets.
//!
//! **Invariants:** an occurrence is a path only on segment boundaries: the byte before it is no
//! segment byte, and the byte after it is no name byte (a `.` counts as a boundary only when no
//! name byte follows it, so `from.md` is another name but `from.` ends a sentence); a token led by
//! `//` after a `:` is a URL and never a repository path.

/// Whether `byte` may appear inside one path segment.
pub(crate) fn is_segment_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'~' | b'@' | b'+')
}

/// Whether `byte` may appear inside a name: letters, digits, `_` and `-`.
pub(crate) fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

/// Where an occurrence of a path spelling sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PathOccurrence {
    /// Not on segment boundaries: part of a longer name.
    NotAPath,
    /// Inside a URL.
    Url,
    /// A repository-root spelling, bare or after a leading `/`.
    RepositoryRoot,
    /// In a token led by `./` or `../`, or holding a `..` segment: the relative-reference pass
    /// owns it.
    Relative {
        /// Where the relative token starts.
        token_start: usize,
    },
    /// Behind other path segments, such as `elsewhere/from`.
    Embedded {
        /// Where the whole token starts.
        token_start: usize,
    },
}

/// Classify the occurrence of a spelling at `start..end` in `source`.
pub(crate) fn classify_occurrence(source: &str, start: usize, end: usize) -> PathOccurrence {
    let bytes = source.as_bytes();
    if !ends_on_boundary(bytes, end) {
        return PathOccurrence::NotAPath;
    }
    if start == 0 {
        return PathOccurrence::RepositoryRoot;
    }
    let before = bytes[start - 1];
    if is_segment_byte(before) {
        return PathOccurrence::NotAPath;
    }
    if before != b'/' {
        return PathOccurrence::RepositoryRoot;
    }
    let token_start = token_start(bytes, start - 1);
    let prefix = &source[token_start..start];
    if prefix.starts_with("//") {
        return PathOccurrence::Url;
    }
    if prefix == "/" {
        return PathOccurrence::RepositoryRoot;
    }
    let first_segment = prefix
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("");
    if first_segment == "." || first_segment == ".." || prefix.split('/').any(|s| s == "..") {
        return PathOccurrence::Relative { token_start };
    }
    PathOccurrence::Embedded { token_start }
}

/// Whether the spelling ending at `end` ends on a segment boundary.
pub(crate) fn ends_on_boundary(bytes: &[u8], end: usize) -> bool {
    match bytes.get(end) {
        None => true,
        Some(b'.') => !bytes.get(end + 1).copied().is_some_and(is_name_byte),
        Some(byte) => !is_name_byte(*byte),
    }
}

/// The first byte of the path token holding byte `at`: the scan runs back over segment bytes and
/// `/` separators.
pub(crate) fn token_start(bytes: &[u8], at: usize) -> usize {
    let mut index = at;
    while index > 0 && (is_segment_byte(bytes[index - 1]) || bytes[index - 1] == b'/') {
        index -= 1;
    }
    index
}

/// One past the last byte of the path token starting at `at`, trailing dots (sentence ends)
/// excluded.
pub(crate) fn token_end(bytes: &[u8], at: usize) -> usize {
    let mut index = at;
    while index < bytes.len() && (is_segment_byte(bytes[index]) || bytes[index] == b'/') {
        index += 1;
    }
    while index > at && bytes[index - 1] == b'.' {
        index -= 1;
    }
    index
}

/// Every start offset of `needle` in `haystack`, overlapping matches included.
pub(crate) fn match_starts(haystack: &str, needle: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut from = 0;
    while let Some(found) = haystack[from..].find(needle) {
        starts.push(from + found);
        from += found + 1;
        while !haystack.is_char_boundary(from) {
            from += 1;
        }
    }
    starts
}
