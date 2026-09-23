//! The command-citation rule: every `cargo xtask` command a live document cites exists.
//!
//! **Role:** finds every `cargo xtask` in a live document's inline code spans and in each line of
//! its fenced code blocks, whatever their info string (a line ending in `\` continues on the next
//! one), reads the words that follow up to where shell syntax or the surrounding prose ends the
//! command ([`command_words`]), and walks them down xtask's own clap command tree
//! ([`walk_command`]).
//!
//! **Position:** a [`DocumentRule`] of the link-check pipeline in [`super::super::link_check`];
//! it reads the code spans and fenced blocks of the scan ([`super::markdown_scan`]). The tree is
//! [`crate::cli::Cli`]'s, built once per run by [`xtask_command_tree`]; the tests hand the rule a
//! tree of their own.
//!
//! **Signals & state:** the counts of one run.
//!
//! **Invariants:** frozen records are never judged; only `cargo` standing as a word of its own
//! opens a citation, so `hcargo xtask` is none; while the command reached has subcommands, each
//! word that is neither a flag nor a placeholder must name one of them or an alias of one, a flag
//! that takes a value takes the next word with it, and a placeholder ends the walk without a
//! break; the words after a leaf command are its arguments and are never judged; a break names the
//! command path up to and including the first word that names no subcommand.

use clap::{Command, CommandFactory};

use super::judged_documents::DocumentArea;
use super::markdown_scan::CodeBlock;
use super::{BreakRule, DocumentRule, JudgedDocument, RuleContext, RuleFindings};
use crate::cli::Cli;

/// The program word of a citation.
const CARGO: &str = "cargo";

/// The subcommand word that makes a cargo invocation a citation of xtask.
const XTASK: &str = "xtask";

/// What walking a cited command's words down the command tree concluded.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum CitedCommand {
    /// Every word resolved, up to a leaf command or the end of the words.
    Exists,
    /// A placeholder stands where a subcommand belongs, so the rest cannot be judged.
    ReachesPlaceholder,
    /// The command path up to and including the first word that names no subcommand, spelled
    /// `cargo xtask …`.
    Unknown(String),
}

/// xtask's own command tree, built the way clap builds it before parsing, so the subcommands and
/// flags clap adds itself are part of it.
pub(super) fn xtask_command_tree() -> Command {
    let mut tree = Cli::command();
    tree.build();
    tree
}

/// The text after each `cargo xtask` in `text`, in order. `cargo` must stand as a word of its
/// own — at the start, or after anything but a letter, a digit, `_`, `-` or `.` — and be followed
/// by blanks and `xtask`, which must end there or before whitespace or shell syntax.
pub(super) fn citations(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find(CARGO) {
        let start = from + offset;
        from = start + CARGO.len();
        let joined_to_word = text[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'));
        let after_cargo = &text[from..];
        let after_blanks = after_cargo.trim_start_matches([' ', '\t']);
        if joined_to_word || after_blanks.len() == after_cargo.len() {
            continue;
        }
        let Some(rest) = after_blanks.strip_prefix(XTASK) else {
            continue;
        };
        if rest
            .chars()
            .next()
            .is_none_or(|c| c.is_whitespace() || ends_command(c))
        {
            found.push(rest);
        }
    }
    found
}

/// Whether `c` is shell syntax that ends a command: a pipe, a list or background operator, a
/// redirection, the end of a command substitution, or a backtick.
fn ends_command(c: char) -> bool {
    matches!(c, '|' | '&' | ';' | '>' | ')' | '`')
}

/// The words of the command that follows a `cargo xtask`, up to where shell syntax or prose ends
/// it: a `#` comment, a pipe, `&&`, `;`, a redirection, the end of a command substitution, the
/// closing quote of a string the citation sits in, or a `,`, `:` or `.` after a word, which no
/// subcommand name ends in and which a citation inside a sentence carries. Quotes around a word are
/// removed; a `>` that closes a `<` in the same word belongs to a placeholder, not a redirection.
pub(super) fn command_words(rest: &str) -> Vec<&str> {
    let mut words = Vec::new();
    for token in rest.split_whitespace() {
        if token.starts_with('#') {
            break;
        }
        let (word, operator) = match shell_operator_at(token) {
            Some(at) => (&token[..at], true),
            None => (token, false),
        };
        let (word, closes_string) = unquote(word);
        let (word, ends_sentence) = without_sentence_punctuation(word);
        let file_descriptor = operator && word.bytes().all(|byte| byte.is_ascii_digit());
        if !word.is_empty() && !file_descriptor {
            words.push(word);
        }
        if operator || closes_string || ends_sentence {
            break;
        }
    }
    words
}

/// `word` without one trailing `,`, `:` or `.`, and whether it carried one; an elision (`...`)
/// and a word of punctuation alone keep theirs.
fn without_sentence_punctuation(word: &str) -> (&str, bool) {
    if word.ends_with("...") {
        return (word, false);
    }
    match word.strip_suffix([',', ':', '.']) {
        Some(stripped) if !stripped.is_empty() => (stripped, true),
        _ => (word, false),
    }
}

/// Where the first shell operator in `token` sits, skipping each `>` that closes a `<`.
fn shell_operator_at(token: &str) -> Option<usize> {
    let mut open_angles = 0usize;
    for (at, c) in token.char_indices() {
        match c {
            '<' => open_angles += 1,
            '>' if open_angles > 0 => open_angles -= 1,
            c if ends_command(c) => return Some(at),
            _ => {}
        }
    }
    None
}

/// `word` without its quotes, and whether it ends with the closing quote of a string opened
/// before it, which ends the command.
fn unquote(word: &str) -> (&str, bool) {
    for quote in ['"', '\''] {
        if let Some(inner) = word.strip_prefix(quote) {
            return (inner.strip_suffix(quote).unwrap_or(inner), false);
        }
        if let Some(inner) = word.strip_suffix(quote) {
            return (inner, true);
        }
    }
    (word, false)
}

/// Walk `words` down the command tree from `root`.
pub(super) fn walk_command(root: &Command, words: &[&str]) -> CitedCommand {
    let mut command = root;
    let mut path: Vec<&str> = Vec::new();
    let mut remaining = words.iter();
    while command.has_subcommands() && !command.is_allow_external_subcommands_set() {
        let Some(&word) = remaining.next() else {
            break;
        };
        if is_flag(word) {
            if flag_takes_value(command, word) {
                remaining.next();
            }
            continue;
        }
        if is_placeholder(word) {
            return CitedCommand::ReachesPlaceholder;
        }
        match command.find_subcommand(word) {
            Some(subcommand) => {
                path.push(word);
                command = subcommand;
            }
            None if command.get_positionals().next().is_some() => break,
            None => {
                path.push(word);
                return CitedCommand::Unknown(format!("{CARGO} {XTASK} {}", path.join(" ")));
            }
        }
    }
    CitedCommand::Exists
}

/// Whether `word` is a flag: `-x`, `--name` or `--name=value`.
fn is_flag(word: &str) -> bool {
    word.len() > 1 && word.starts_with('-')
}

/// Whether `word` is a placeholder for words the citation leaves out: `<…>`, `[…]`, `{…}`, or an
/// ellipsis.
fn is_placeholder(word: &str) -> bool {
    word.contains(['<', '[', '{', '…']) || word.contains("...")
}

/// Whether the flag `word`, written without `=value`, names an option of `command` that takes
/// a value, which is then the next word.
fn flag_takes_value(command: &Command, word: &str) -> bool {
    if word.contains('=') {
        return false;
    }
    let option = match word.strip_prefix("--") {
        Some(long) => command.get_arguments().find(|argument| {
            argument.get_long() == Some(long)
                || argument
                    .get_all_aliases()
                    .is_some_and(|aliases| aliases.contains(&long))
        }),
        None => {
            let mut shorts = word[1..].chars();
            let short = shorts.next();
            if shorts.next().is_some() {
                return false;
            }
            command
                .get_arguments()
                .find(|argument| argument.get_short() == short)
        }
    };
    option.is_some_and(|argument| argument.get_action().takes_values())
}

/// The lines of a fenced block as the shell reads them: a line ending in `\` joined to the next,
/// each with the 1-based document line it starts on.
pub(super) fn shell_lines(block: &CodeBlock) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (offset, text) in block.lines.iter().enumerate() {
        let (start, mut joined) = open
            .take()
            .unwrap_or_else(|| (block.line + 1 + offset, String::new()));
        joined.push_str(text);
        match joined.trim_end().strip_suffix('\\') {
            Some(head) => open = Some((start, format!("{head} "))),
            None => lines.push((start, joined)),
        }
    }
    lines.extend(open);
    lines
}

/// How the rule's cited commands ended.
#[derive(Debug, Default)]
struct CitationCounts {
    existing: usize,
    placeholders: usize,
    unknown: usize,
}

/// The command-citation rule's state for one run.
pub(super) struct CommandCitations<'c> {
    commands: &'c Command,
    counts: CitationCounts,
}

impl<'c> CommandCitations<'c> {
    /// A rule that walks citations down `commands`, a built command tree whose root stands for
    /// `cargo xtask`.
    pub(super) fn new(commands: &'c Command) -> CommandCitations<'c> {
        CommandCitations {
            commands,
            counts: CitationCounts::default(),
        }
    }

    /// Judge every citation in `text`, found at `line` of `document`.
    fn judge_text(&mut self, document: &str, line: usize, text: &str, findings: &mut RuleFindings) {
        for rest in citations(text) {
            match walk_command(self.commands, &command_words(rest)) {
                CitedCommand::Exists => self.counts.existing += 1,
                CitedCommand::ReachesPlaceholder => self.counts.placeholders += 1,
                CitedCommand::Unknown(path) => {
                    self.counts.unknown += 1;
                    findings.broke(
                        document,
                        line,
                        BreakRule::CitedCommandDoesNotExist,
                        format!("`{path}`"),
                    );
                }
            }
        }
    }
}

impl DocumentRule for CommandCitations<'_> {
    fn judges(&self, area: DocumentArea) -> bool {
        !area.is_frozen()
    }

    fn judge(
        &mut self,
        document: &JudgedDocument<'_>,
        _context: &RuleContext<'_>,
        findings: &mut RuleFindings,
    ) {
        for span in &document.scan.code_spans {
            self.judge_text(document.path, span.line, &span.text, findings);
        }
        for block in &document.scan.code_blocks {
            for (line, text) in shell_lines(block) {
                self.judge_text(document.path, line, &text, findings);
            }
        }
    }

    fn totals(&self) -> Vec<String> {
        let counts = &self.counts;
        let judged = counts.existing + counts.placeholders + counts.unknown;
        vec![format!(
            "  command citations: {judged} judged in live documents — {} name an existing \
             command, {} reach a placeholder, {} name no command",
            counts.existing, counts.placeholders, counts.unknown
        )]
    }
}

#[cfg(test)]
#[path = "tests/command_citations.rs"]
mod tests;
