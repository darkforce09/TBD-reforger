use super::*;
impl eframe::App for TicketboardApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);

        let mut actions: Vec<Action> = Vec::new();
        let busy = self.load_rx.is_some();
        let tab = self.tab;
        // while a verb subprocess is in flight, every mutation
        // affordance (menus, strip, dialog Run, drag, New ticket) is disabled.
        let verb_busy = self.verb.queue.busy();
        let board_active = matches!(self.state, State::Board(_));

        // Trust banner ABOVE the tabs — persistent on every tab and every state
        // once a repo is active.
        if self.repo_root.is_some() {
            Panel::top(Id::new("trustbanner")).show(ui, |ui| {
                self.trust_banner_ui(ui, &mut actions);
            });
        }
        Panel::top(Id::new("topbar")).show(ui, |ui| {
            topbar_ui(
                ui,
                self.repo_root.as_deref(),
                busy,
                tab,
                board_active,
                verb_busy,
                &mut actions,
            );
        });
        if board_active {
            Panel::top(Id::new("filterbar")).show(ui, |ui| {
                if let State::Board(b) = &mut self.state
                    && filter_bar_ui(
                        ui,
                        &mut b.filters,
                        &b.filter_index.executors,
                        &b.facet_options,
                    )
                {
                    actions.push(Action::FiltersChanged);
                }
            });
        }
        if let State::Board(b) = &self.state {
            Panel::bottom(Id::new("footer")).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&b.footer);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        mutate::verb_chip_ui(ui, &self.verb, &mut actions);
                    });
                });
            });
            // Verb drawer ABOVE the footer: streamed log while a verb runs;
            // stays open with the full verbatim output on a nonzero exit.
            if self.verb.drawer_open {
                Panel::bottom(Id::new("verb_drawer")).show(ui, |ui| {
                    mutate::drawer_ui(ui, &self.verb, &mut actions);
                });
            }
            // The right region contains the detail column and a viewer column to its right.
            // Closing the viewer leaves selection intact. The viewer is added first so it
            // occupies the right edge; narrow windows show the viewer alone.
            let pane = right_pane(
                self.viewer.is_open(),
                b.selected,
                ctx.content_rect().width(),
            );
            if matches!(pane, RightPane::Viewer | RightPane::Both(_)) {
                let shown = Panel::right(Id::new("viewer"))
                    .resizable(true)
                    .default_size(self.viewer_w)
                    .size_range(VIEWER_W_MIN..=VIEWER_W_MAX)
                    .show(ui, |ui| {
                        viewer_pane_ui(
                            ui,
                            &self.viewer,
                            &mut self.md_cache,
                            self.repo_root.as_deref(),
                            &mut actions,
                        );
                    });
                // The dragged width lands here every frame and persists via
                // `save` (eframe Storage) — clamped, so the stored preference
                // always round-trips through `parse_viewer_width` unchanged.
                self.viewer_w = shown
                    .response
                    .rect
                    .width()
                    .clamp(VIEWER_W_MIN, VIEWER_W_MAX);
            }
            if let RightPane::Detail(selected) | RightPane::Both(selected) = pane {
                Panel::right(Id::new("detail"))
                    .resizable(true)
                    .default_size(DETAIL_W)
                    .show(ui, |ui| {
                        detail_ui(
                            ui,
                            MutationContext {
                                repo_root: self.repo_root.as_deref(),
                                busy: verb_busy,
                            },
                            b,
                            selected,
                            &mut self.advanced_status,
                            &mut actions,
                        );
                    });
            }
        }
        egui::CentralPanel::default().show(ui, |ui| match &self.state {
            State::NoRepo { note } => {
                norepo_ui(ui, note.as_deref(), self.pick_rx.is_some(), &mut actions);
            }
            State::Loading => loading_ui(ui, self.repo_root.as_deref()),
            State::Refused(e) => refusal_ui(ui, e, &mut actions),
            State::Board(b) => {
                let mctx = MutationContext {
                    repo_root: self.repo_root.as_deref(),
                    busy: verb_busy,
                };
                match tab {
                    Tab::Board => board_ui(ui, b, mctx, &mut actions),
                    Tab::Waves => waves_ui(ui, b, &mut actions),
                    Tab::Tree => tree_ui(ui, b, &mut actions),
                    Tab::Metrics => metrics_ui(ui, b, &mut actions),
                }
            }
        });

        // dialog pass — one modal above everything, closed on Cancel /
        // Esc / backdrop / dispatch (dialogs only exist over a loaded board).
        if let Some(dialog) = &mut self.dialog {
            let mut keep = false;
            if let State::Board(b) = &self.state {
                let mctx = MutationContext {
                    repo_root: self.repo_root.as_deref(),
                    busy: verb_busy,
                };
                keep = !mutate::dialog_ui(&ctx, b, mctx, dialog, &mut actions);
            }
            if !keep {
                self.dialog = None;
            }
        }
        mutate::toasts_ui(&ctx, &mut self.toasts);

        self.apply(actions, &ctx);
    }

    /// Persist the user preferences: the active repo root (revalidated on the
    /// next launch) and the viewer-column width (— revalidated +
    /// clamped through `parse_viewer_width`). This is the app's ONLY direct
    /// write surface, and it goes to eframe Storage in the user config dir —
    /// never the repo (mutations shell the xtask verbs; the app
    /// writes no ticket bytes itself).
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Some(root) = &self.repo_root {
            storage.set_string(REPO_ROOT_KEY, root.display().to_string());
        }
        storage.set_string(VIEWER_W_KEY, self.viewer_w.to_string());
    }
}
