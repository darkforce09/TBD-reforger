use super::*;
impl TicketboardApp {
    pub(super) fn toast(&mut self, text: String, error: bool) {
        self.toasts.push(Toast::new(text, error));
    }

    /// Dispatch a verb request from the UI: re-hash the CAS guard NOW; a
    /// mismatch refuses the dispatch (no subprocess) and reloads. Otherwise the
    /// request enters the single-flight queue — spawned immediately when idle,
    /// parked FIFO when a verb is already running.
    pub(super) fn dispatch_verb(&mut self, req: verbs::TicketCommand, ctx: &egui::Context) {
        if !verbs::cas_ok(req.guard.as_ref()) {
            self.toast(
                format!("{} — file changed on disk — reloading", req.display),
                true,
            );
            self.start_load(ctx, true);
            return;
        }
        match self.verb.queue.submit(req) {
            Some(start) => self.spawn_verb(start, ctx),
            None => {
                let pending = self.verb.queue.pending_len();
                self.toast(
                    format!("queued behind the running verb ({pending} pending)"),
                    false,
                );
            }
        }
    }

    /// Spawn `cargo <alias-expanded verb argv>` at the repo root through the
    /// subproc helper. Sets the watch-suppression flag and opens the
    /// drawer so the streamed log is visible while the verb runs.
    pub(super) fn spawn_verb(&mut self, req: verbs::TicketCommand, ctx: &egui::Context) {
        let Some(root) = self.repo_root.clone() else {
            // No repo root — the queue slot must not stay claimed forever.
            let _ = self.verb.queue.finish(false);
            return;
        };
        self.verb.log.clear();
        self.verb.last = None;
        self.verb.dropped_note = None;
        self.verb.drawer_open = true;
        self.set_verb_in_flight(true);
        let args: Vec<&str> = req.args.iter().map(String::as_str).collect();
        let repaint_ctx = ctx.clone();
        self.verb.handle = Some(subproc::spawn_streaming(
            &self.cargo,
            &args,
            &root,
            move || repaint_ctx.request_repaint(),
        ));
    }

    /// Drain the in-flight verb stream. On exit — success or refusal — ALWAYS
    /// reload corpus+lock and refresh the git chip. Success shows the stdout
    /// tail as a toast; a nonzero exit keeps the drawer open with the FULL
    /// verbatim output + exit code, drops the pending queue (never auto-retry),
    /// triggers ONE strict re-check (the banner goes red on wave-stale
    /// state), and — when the log carries the wave-stale signature — shows the
    /// recovery command as text. The app never runs `wave repack` itself.
    pub(super) fn poll_verb(&mut self, ctx: &egui::Context) {
        let mut term: Option<(Option<i32>, Option<String>)> = None;
        if let Some(handle) = &self.verb.handle {
            while let Ok(event) = handle.rx.try_recv() {
                match event {
                    ProcessEvent::Line(line) => self.verb.log.push(line),
                    ProcessEvent::Exited { code } => term = Some((code, None)),
                    ProcessEvent::SpawnFailed(error) => term = Some((None, Some(error))),
                }
            }
        }
        let Some((code, spawn_error)) = term else {
            return;
        };
        self.verb.handle = None;
        let display = self
            .verb
            .queue
            .running_display()
            .unwrap_or("verb")
            .to_owned();
        let success = code == Some(0) && spawn_error.is_none();
        // Signal-killed (the mid-verb SIGKILL between save and repack) counts as
        // the wave-stale hazard even though the dead process printed nothing.
        let killed = code.is_none() && spawn_error.is_none();
        let hint = !success
            && verbs::recovery_hint_applies(killed, self.verb.log.iter().map(String::as_str));
        self.verb.last = Some(CommandOutcome {
            display: display.clone(),
            code,
            at: trust::utc_hms(epoch_secs()),
            spawn_error,
            hint,
        });
        // ALWAYS — even (especially) after a refusal: the verb may have exited
        // between save and sync/repack, and the board must show the disk truth.
        self.start_load(ctx, true);
        self.trigger_git(ctx);
        if success {
            let tail = verbs::success_tail(self.verb.log.iter().map(String::as_str))
                .unwrap_or_else(|| format!("exit 0 — {display}"));
            self.toast(tail, false);
            self.verb.drawer_open = false;
        } else {
            self.verb.drawer_open = true;
            // One deliberate strict re-check (coalesced) — the banner is
            // how mid-verb-crash wave-stale state surfaces as check-red.
            self.trigger_check(ctx);
        }
        let mut fin = self.verb.queue.finish(success);
        if fin.dropped > 0 {
            self.verb.dropped_note = Some(format!(
                "{} pending verb request(s) dropped after this failure — nothing auto-retries",
                fin.dropped
            ));
        }
        while let Some(next) = fin.next {
            if verbs::cas_ok(next.guard.as_ref()) {
                self.spawn_verb(next, ctx);
                break;
            }
            self.toast(
                format!("{} — file changed on disk — reloading", next.display),
                true,
            );
            fin = self.verb.queue.finish(true);
        }
        if !self.verb.queue.busy() {
            self.set_verb_in_flight(false);
        }
    }
}
