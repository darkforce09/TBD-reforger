use super::*;
// ---- single-flight queue ----

/// Result of [`TicketCommandQueue::finish`].
#[derive(Debug, PartialEq, Eq)]
pub struct Finish {
    /// Pending requests dropped because the finished verb failed — surfaced in
    /// the drawer; nothing auto-retries.
    pub dropped: usize,
    /// The next FIFO request, already marked running — the caller must spawn it
    /// or report it back through another `finish` (CAS refusal).
    pub next: Option<TicketCommand>,
}

/// One verb subprocess at a time; extra requests wait FIFO, each keeping the
/// fingerprint captured when its affordance was used. A failure drops the whole
/// pending tail — cascading writes onto a refusal helps nobody, and the app
/// never auto-retries.
#[derive(Debug, Default)]
pub struct TicketCommandQueue {
    running: Option<TicketCommand>,
    pending: VecDeque<TicketCommand>,
}

impl TicketCommandQueue {
    /// Submit a request. `Some` ⇒ the queue was idle — spawn it NOW; `None` ⇒
    /// parked FIFO behind the in-flight verb.
    #[must_use]
    pub fn submit(&mut self, req: TicketCommand) -> Option<TicketCommand> {
        if self.running.is_some() {
            self.pending.push_back(req);
            None
        } else {
            self.running = Some(req.clone());
            Some(req)
        }
    }

    /// The in-flight verb finished (or a popped-but-CAS-refused request was
    /// abandoned — report that as `success = true`: the refusal is not a verb
    /// failure). Failure clears the pending tail; success pops the next request
    /// and marks it running.
    #[must_use]
    pub fn finish(&mut self, success: bool) -> Finish {
        self.running = None;
        if !success {
            let dropped = self.pending.len();
            self.pending.clear();
            return Finish {
                dropped,
                next: None,
            };
        }
        let next = self.pending.pop_front();
        if let Some(n) = &next {
            self.running = Some(n.clone());
        }
        Finish { dropped: 0, next }
    }

    /// The app's in-flight flag (drives `set_verb_in_flight` and every disabled
    /// affordance).
    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// The literal command line of the running verb (drawer header).
    pub fn running_display(&self) -> Option<&str> {
        self.running.as_ref().map(|r| r.display.as_str())
    }
}
