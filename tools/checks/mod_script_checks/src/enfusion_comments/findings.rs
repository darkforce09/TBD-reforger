//! Rule identifiers and the finding every rule reports.
//!
//! **Role:** names the nine rules of the Enfusion in-code documentation card, one identifier per
//! card item, and carries one violation from a rule to the report.
//!
//! **Position:** produced by the rule modules of `enfusion_comments`; printed and counted by the
//! verification entry in `enfusion_comments/mod.rs`.
//!
//! **Signals & state:** none; plain data.
//!
//! **Invariants:** [`RuleId::ALL`] lists every rule exactly once, in card order, so the per-rule
//! counts always print nine rows.

use std::fmt;

/// One rule of the Enfusion in-code documentation card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuleId {
    /// ECM-1: ASCII only, string literals included.
    AsciiOnly,
    /// ECM-2: the `/** ... */` file header comes first and carries every field.
    FileHeader,
    /// ECM-3: a `//!` banner sits directly above every class, enum and method.
    DeclarationBanner,
    /// ECM-4: a trailing `//!<` documents every field and enum member.
    TrailingMemberDoc,
    /// ECM-5: `@authority`, `@rpc` and `@replicated` network tags.
    NetworkAuthority,
    /// ECM-6: `@route` on REST call sites and `@contract` on schema-backed DTOs.
    BoundaryTags,
    /// ECM-7: `[Attribute]` has `desc:` and `[ComponentEditorProps]` has `description:`.
    AttributeDescription,
    /// ECM-8: comments are present tense and context-free.
    ContextFreeProse,
    /// ECM-9: the file name is the primary type, one primary type per file.
    FileNamesPrimaryType,
}

impl RuleId {
    /// Every rule, in card order.
    pub(crate) const ALL: [RuleId; 9] = [
        RuleId::AsciiOnly,
        RuleId::FileHeader,
        RuleId::DeclarationBanner,
        RuleId::TrailingMemberDoc,
        RuleId::NetworkAuthority,
        RuleId::BoundaryTags,
        RuleId::AttributeDescription,
        RuleId::ContextFreeProse,
        RuleId::FileNamesPrimaryType,
    ];

    /// The printed rule id, `ECM-1` through `ECM-9`.
    pub(crate) fn code(self) -> &'static str {
        match self {
            RuleId::AsciiOnly => "ECM-1",
            RuleId::FileHeader => "ECM-2",
            RuleId::DeclarationBanner => "ECM-3",
            RuleId::TrailingMemberDoc => "ECM-4",
            RuleId::NetworkAuthority => "ECM-5",
            RuleId::BoundaryTags => "ECM-6",
            RuleId::AttributeDescription => "ECM-7",
            RuleId::ContextFreeProse => "ECM-8",
            RuleId::FileNamesPrimaryType => "ECM-9",
        }
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// One violation in one script.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Finding {
    /// The 1-based line the violation is reported on.
    pub(crate) line: usize,
    /// The rule the line breaks.
    pub(crate) rule: RuleId,
    /// What is wrong, in one line.
    pub(crate) message: String,
}

impl Finding {
    /// A finding of `rule` on `line`.
    pub(crate) fn new(rule: RuleId, line: usize, message: impl Into<String>) -> Self {
        Finding {
            line,
            rule,
            message: message.into(),
        }
    }
}
