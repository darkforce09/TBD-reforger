/// What one read produced — the two terminal states of the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentOutcome {
    Rendered { text: String },
    Fallback { text: String, note: String },
}

/// Worker-thread result: the outcome tagged with the path it answers, so
/// [`super::ViewerState::land`] can drop stale reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDocument {
    pub rel: String,
    pub outcome: DocumentOutcome,
}
