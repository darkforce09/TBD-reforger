//! Locating the `game.scenarioId` string in a JSON document without re-serialising it.
//!
//! The scanner walks the tokens of a document that is already known to be valid JSON: the root
//! object's members, the member named `game` (which must be an object), and that object's
//! member named `scenarioId` (which must be a string). Member names are decoded before they are
//! compared, so an escaped spelling of a name is found too. A name that appears twice in the
//! same object is refused, since readers of the file could disagree on which member counts.

use std::ops::Range;

/// Why the document has no single `game.scenarioId` string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScenarioIdAbsence {
    RootNotAnObject,
    NoGame,
    DuplicateGame,
    GameNotAnObject,
    NoScenarioId,
    DuplicateScenarioId,
    ScenarioIdNotAString,
    Malformed,
}

impl ScenarioIdAbsence {
    pub(super) fn describe(self) -> &'static str {
        match self {
            Self::RootNotAnObject => "the document is not a JSON object",
            Self::NoGame => "the document has no game member",
            Self::DuplicateGame => "the document names game twice",
            Self::GameNotAnObject => "game is not an object",
            Self::NoScenarioId => "game has no scenarioId member",
            Self::DuplicateScenarioId => "game names scenarioId twice",
            Self::ScenarioIdNotAString => "game.scenarioId is not a string",
            Self::Malformed => "the document is not well-formed JSON",
        }
    }
}

/// Why an object could not yield the position of one member's value.
enum MemberSearch {
    NotAnObject,
    Duplicate,
    Malformed,
}

/// The byte range of the `game.scenarioId` string token, quotes included.
pub(super) fn scenario_id_span(document: &str) -> Result<Range<usize>, ScenarioIdAbsence> {
    let mut scanner = Scanner {
        text: document,
        position: 0,
    };
    let game = scanner
        .member_value_position("game")
        .map_err(|search| match search {
            MemberSearch::NotAnObject => ScenarioIdAbsence::RootNotAnObject,
            MemberSearch::Duplicate => ScenarioIdAbsence::DuplicateGame,
            MemberSearch::Malformed => ScenarioIdAbsence::Malformed,
        })?
        .ok_or(ScenarioIdAbsence::NoGame)?;
    scanner.position = game;
    let scenario_id = scanner
        .member_value_position("scenarioId")
        .map_err(|search| match search {
            MemberSearch::NotAnObject => ScenarioIdAbsence::GameNotAnObject,
            MemberSearch::Duplicate => ScenarioIdAbsence::DuplicateScenarioId,
            MemberSearch::Malformed => ScenarioIdAbsence::Malformed,
        })?
        .ok_or(ScenarioIdAbsence::NoScenarioId)?;
    scanner.position = scenario_id;
    if scanner.peek() != Some(b'"') {
        return Err(ScenarioIdAbsence::ScenarioIdNotAString);
    }
    scanner.string_token().ok_or(ScenarioIdAbsence::Malformed)
}

struct Scanner<'a> {
    text: &'a str,
    position: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    /// Scans the whole object at the cursor and returns where the value of the member named
    /// `wanted` starts, if it has one. The cursor ends after the object.
    fn member_value_position(&mut self, wanted: &str) -> Result<Option<usize>, MemberSearch> {
        self.skip_whitespace();
        if self.peek() != Some(b'{') {
            return Err(MemberSearch::NotAnObject);
        }
        self.position += 1;
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(None);
        }
        let mut found = None;
        loop {
            self.skip_whitespace();
            let name = self.string_token().ok_or(MemberSearch::Malformed)?;
            let name: String =
                serde_json::from_str(&self.text[name]).map_err(|_| MemberSearch::Malformed)?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(MemberSearch::Malformed);
            }
            self.position += 1;
            self.skip_whitespace();
            if name == wanted {
                if found.is_some() {
                    return Err(MemberSearch::Duplicate);
                }
                found = Some(self.position);
            }
            self.skip_value().ok_or(MemberSearch::Malformed)?;
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    return Ok(found);
                }
                _ => return Err(MemberSearch::Malformed),
            }
        }
    }

    /// The range of the string token at the cursor, quotes included; the cursor ends after it.
    fn string_token(&mut self) -> Option<Range<usize>> {
        let start = self.position;
        if self.peek() != Some(b'"') {
            return None;
        }
        self.position += 1;
        loop {
            match self.peek()? {
                b'"' => {
                    self.position += 1;
                    return Some(start..self.position);
                }
                b'\\' => self.position += 2,
                _ => self.position += 1,
            }
        }
    }

    /// Steps over the value at the cursor: a string, an object or array with everything inside
    /// it, or a number or literal.
    fn skip_value(&mut self) -> Option<()> {
        match self.peek()? {
            b'"' => self.string_token().map(drop),
            b'{' | b'[' => {
                let mut depth = 0usize;
                loop {
                    match self.peek()? {
                        b'"' => {
                            self.string_token()?;
                        }
                        b'{' | b'[' => {
                            depth += 1;
                            self.position += 1;
                        }
                        b'}' | b']' => {
                            depth -= 1;
                            self.position += 1;
                            if depth == 0 {
                                return Some(());
                            }
                        }
                        _ => self.position += 1,
                    }
                }
            }
            _ => {
                while !matches!(
                    self.peek(),
                    None | Some(b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
                ) {
                    self.position += 1;
                }
                Some(())
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/scenario_id_location.rs"]
mod tests;
