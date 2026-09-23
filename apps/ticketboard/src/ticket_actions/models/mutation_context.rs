use super::*;
// ---- read-only context every mutation affordance needs ----

#[derive(Clone, Copy)]
pub struct MutationContext<'a> {
    pub repo_root: Option<&'a Path>,
    /// A verb subprocess is in flight — every dispatch affordance disables.
    pub busy: bool,
}

/// Ticket data required by command affordances; excludes all application and rendering state.
pub(crate) struct TicketActionContext<'a> {
    pub corpus: &'a Corpus,
    pub id_to_index: &'a HashMap<String, usize>,
}
