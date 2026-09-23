use super::*;
impl TicketboardApp {
    /// Monotonic milliseconds since application startup, used by the debouncer.
    pub(super) fn now_ms(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Suppress strict-check reruns during a mutation and one trailing debounce window.
    /// Reloads continue so the UI can show command writes without starting a check storm.
    pub fn set_verb_in_flight(&mut self, in_flight: bool) {
        let now = self.now_ms();
        self.debounce.set_suppressed(in_flight, now);
    }

    /// Request a strict check. Triggers during a running check coalesce into one follow-up.
    pub(super) fn trigger_check(&mut self, ctx: &egui::Context) {
        if self.repo_root.is_none() {
            return;
        }
        if self.check.coalescer.trigger() {
            self.start_check(ctx);
        }
    }

    /// Spawn `cargo xtask ticket check --strict` (alias-expanded argv) at the repo
    /// root — the app never re-implements check, it invokes it.
    pub(super) fn start_check(&mut self, ctx: &egui::Context) {
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        self.check.on_start();
        self.check_log.clear();
        let repaint_ctx = ctx.clone();
        self.check_handle = Some(subproc::spawn_streaming(
            &self.cargo,
            &trust::CHECK_ARGS,
            &root,
            move || repaint_ctx.request_repaint(),
        ));
    }

    // ---- git-dirty chip ----

    /// Refresh the git chip (runs after every reload and every check exit).
    /// Same single-flight coalescing as the check.
    pub(super) fn trigger_git(&mut self, ctx: &egui::Context) {
        if self.repo_root.is_none() {
            return;
        }
        if self.git_flight.trigger() {
            self.start_git(ctx);
        }
    }

    pub(super) fn start_git(&mut self, ctx: &egui::Context) {
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        self.git_lines.clear();
        let repaint_ctx = ctx.clone();
        self.git_handle = Some(subproc::spawn_streaming(
            "git",
            &gitstatus::GIT_ARGS,
            &root,
            move || repaint_ctx.request_repaint(),
        ));
    }

    // ---- file watch ----

    /// Arm the notify watches for the active root. Only the `.ai/tickets` watch is
    /// load-bearing; its failure is surfaced in the banner, never a crash.
    pub(super) fn arm_watch(&mut self, ctx: &egui::Context) {
        self.watch = None;
        self.watch_rx = None;
        self.watch_error = None;
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        let repaint_ctx = ctx.clone();
        match watch::spawn(&root, tx, move || repaint_ctx.request_repaint()) {
            Ok(handle) => {
                self.watch = Some(handle);
                self.watch_rx = Some(rx);
            }
            Err(e) => self.watch_error = Some(e),
        }
    }

    /// Drain available worker results without blocking, then poll check, Git, watch,
    /// and command events in that order.
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.load_rx
            && let Ok(bundle) = rx.try_recv()
        {
            self.load_rx = None;
            let previous = match &self.state {
                State::Board(board) => Some(board.as_ref()),
                _ => None,
            };
            self.state = match WorkspaceState::reload(bundle, previous) {
                Ok(board) => State::Board(Box::new(board)),
                Err(error) => State::Refused(error),
            };
            // Registry files may have changed under git too — refresh the chip.
            self.trigger_git(ctx);
        }
        if let Some(rx) = &self.pick_rx
            && let Ok(picked) = rx.try_recv()
        {
            self.pick_rx = None;
            if let Some(root) = picked {
                if discovery::has_tickets_dir(&root) {
                    self.adopt_root(root, ctx);
                } else {
                    self.state = State::NoRepo {
                        note: Some(format!(
                            "picked folder {} has no {}/ directory",
                            root.display(),
                            ticket_engine::repository::TICKETS_DIR
                        )),
                    };
                }
            }
        }
        // land the viewer's worker read (stale paths dropped inside).
        if let Some(rx) = &self.viewer_rx
            && let Ok(doc) = rx.try_recv()
        {
            self.viewer_rx = None;
            self.viewer.land(doc);
        }
        self.poll_check(ctx);
        self.poll_git(ctx);
        self.poll_watch(ctx);
        self.poll_verb(ctx);
    }

    pub(super) fn poll_check(&mut self, ctx: &egui::Context) {
        let mut finished = false;
        if let Some(handle) = &self.check_handle {
            while let Ok(event) = handle.rx.try_recv() {
                match event {
                    ProcessEvent::Line(line) => {
                        self.check.on_line(&line);
                        self.check_log.push(line);
                    }
                    ProcessEvent::Exited { code } => {
                        self.check.on_exit(code, trust::utc_hms(epoch_secs()));
                        if code != Some(0) {
                            // Red auto-opens the verbatim pane — the errors are
                            // the point, not a number.
                            self.show_output = true;
                        }
                        finished = true;
                    }
                    ProcessEvent::SpawnFailed(error) => {
                        self.check
                            .on_spawn_failed(error, trust::utc_hms(epoch_secs()));
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.check_handle = None;
            self.trigger_git(ctx);
            if self.check.coalescer.finished() {
                self.start_check(ctx);
            }
        }
    }

    /// Drain the git-status stream into the chip.
    pub(super) fn poll_git(&mut self, ctx: &egui::Context) {
        let mut finished = false;
        if let Some(handle) = &self.git_handle {
            while let Ok(event) = handle.rx.try_recv() {
                match event {
                    ProcessEvent::Line(line) => self.git_lines.push(line),
                    ProcessEvent::Exited { code } => {
                        self.git_chip = gitstatus::chip_from_exit(
                            code,
                            self.git_lines.iter().map(String::as_str),
                        );
                        finished = true;
                    }
                    ProcessEvent::SpawnFailed(error) => {
                        // Git absent — the chip says so; never a crash.
                        self.git_chip = GitChip::Unavailable(error);
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.git_lines.clear();
            self.git_handle = None;
            if self.git_flight.finished() {
                self.start_git(ctx);
            }
        }
    }

    /// Feed raw watch events into the debouncer; a fire reloads corpus+lock in
    /// place and (unless suppressed) re-runs the check through the coalescer.
    pub(super) fn poll_watch(&mut self, ctx: &egui::Context) {
        let now = self.now_ms();
        if let Some(rx) = &self.watch_rx {
            while rx.try_recv().is_ok() {
                self.debounce.on_event(now);
            }
        }
        if let Some(fire) = self.debounce.poll(now) {
            self.start_load(ctx, true);
            if fire.run_check {
                self.trigger_check(ctx);
            }
        }
        // The fire needs a frame after the quiet window even when the user is
        // idle — schedule the wakeup.
        if let Some(due) = self.debounce.due_in(now) {
            ctx.request_repaint_after(Duration::from_millis(due + 5));
        }
    }
}
