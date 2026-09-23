use super::*;
impl TicketboardApp {
    pub(super) fn apply(&mut self, actions: Vec<Action>, ctx: &egui::Context) {
        for action in actions {
            match action {
                Action::Reload => self.start_load(ctx, false),
                Action::Recheck => self.trigger_check(ctx),
                Action::CancelCheck => {
                    if let Some(handle) = &self.check_handle {
                        handle.kill();
                    }
                }
                Action::ToggleOutput => self.show_output = !self.show_output,
                Action::ToggleGitList => self.git_expanded = !self.git_expanded,
                Action::PickFolder => self.start_pick(ctx),
                Action::OpenPath(path) => open_path(&path),
                Action::OpenDoc(rel) => self.open_doc(rel, ctx),
                Action::CloseViewer => {
                    // Back touches ONLY the viewer — the board selection stays,
                    // so the viewer COLUMN collapses and the detail column
                    // keeps rendering the same ticket.
                    self.viewer.close();
                    self.viewer_rx = None;
                }
                Action::SetTab(tab) => self.tab = tab,
                Action::CopyText(text) => ctx.copy_text(text),
                Action::ToggleLegacyExpand => {
                    if let State::Board(b) = &mut self.state {
                        b.legacy_expanded = !b.legacy_expanded;
                    }
                }
                Action::Select(index) => {
                    if let State::Board(b) = &mut self.state {
                        if b.selected != Some(index) {
                            // A different ticket's quarantine starts collapsed.
                            b.legacy_expanded = false;
                        }
                        b.selected = Some(index);
                        if b.compare == Some(index) {
                            b.compare = None;
                        }
                    }
                }
                Action::SelectId(id) => {
                    if let State::Board(b) = &mut self.state
                        && let Some(&index) = b.board.id_to_index.get(&id)
                    {
                        if b.selected != Some(index) {
                            b.legacy_expanded = false;
                        }
                        b.selected = Some(index);
                        if b.compare == Some(index) {
                            b.compare = None;
                        }
                    }
                }
                Action::Compare(index) => {
                    if let State::Board(b) = &mut self.state {
                        match b.selected {
                            // Nothing selected yet: shift-click behaves like select.
                            None => b.selected = Some(index),
                            Some(sel) if sel == index => {}
                            Some(_) => b.compare = Some(index),
                        }
                    }
                }
                Action::ClearCompare => {
                    if let State::Board(b) = &mut self.state {
                        b.compare = None;
                    }
                }
                Action::ToggleColumn(col) => {
                    if let State::Board(b) = &mut self.state {
                        b.expanded[col] = !b.expanded[col];
                    }
                }
                Action::ToggleNode(index) => {
                    if let State::Board(b) = &mut self.state {
                        b.tree_expanded[index] = !b.tree_expanded[index];
                        b.reflatten();
                    }
                }
                Action::ToggleWave0 => {
                    if let State::Board(b) = &mut self.state {
                        b.wave0_expanded = !b.wave0_expanded;
                    }
                }
                Action::FiltersChanged => {
                    if let State::Board(b) = &mut self.state {
                        b.refilter();
                    }
                }
                Action::SortMetrics(table, key) => {
                    // Re-sort ONCE on click — the paint path never sorts.
                    if let State::Board(b) = &mut self.state
                        && let MetricsState::Loaded(m) = &mut b.metrics
                    {
                        match table {
                            TableKind::Ticket => {
                                b.metrics_sort.ticket = b.metrics_sort.ticket.toggled(key);
                                metrics::sort_rows(&mut m.per_ticket, b.metrics_sort.ticket);
                            }
                            TableKind::Agent => {
                                b.metrics_sort.agent = b.metrics_sort.agent.toggled(key);
                                metrics::sort_rows(&mut m.per_agent, b.metrics_sort.agent);
                            }
                        }
                    }
                }
                Action::SortEstimates(table, key) => {
                    if let State::Board(b) = &mut self.state
                        && let EstimatesState::Loaded(e) = &mut b.estimates
                    {
                        match table {
                            EstimatedTableKind::Class => {
                                b.est_sort.class = b.est_sort.class.toggled(key);
                                estimates::sort_rows(&mut e.per_class, b.est_sort.class);
                            }
                            EstimatedTableKind::Domain => {
                                b.est_sort.domain = b.est_sort.domain.toggled(key);
                                estimates::sort_rows(&mut e.per_domain, b.est_sort.domain);
                            }
                        }
                    }
                }
                Action::CloseDetail => {
                    if let State::Board(b) = &mut self.state {
                        b.selected = None;
                        b.compare = None;
                        b.legacy_expanded = false;
                    }
                }
                Action::OpenAnchorDialog(index) => {
                    if let State::Board(b) = &self.state {
                        self.dialog = Some(
                            crate::ticket_actions::services::dialog_builders::anchor_dialog(
                                &mutate::context(b),
                                index,
                            ),
                        );
                    }
                }
                Action::OpenDialog(dialog) => self.dialog = Some(*dialog),
                Action::Dispatch(req) => {
                    self.dialog = None;
                    self.dispatch_verb(req, ctx);
                }
                Action::ToggleVerbDrawer => self.verb.drawer_open = !self.verb.drawer_open,
            }
        }
    }
}
