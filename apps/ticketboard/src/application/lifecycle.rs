use super::*;
impl TicketboardApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        arg: Option<PathBuf>,
        cwd: Option<PathBuf>,
    ) -> Self {
        let mut app = Self {
            repo_root: None,
            state: State::NoRepo { note: None },
            tab: Tab::default(),
            load_rx: None,
            pick_rx: None,
            cargo: subproc::resolve_cargo(),
            check: CheckModel::default(),
            check_log: BoundedLog::new(subproc::LOG_CAP),
            check_handle: None,
            show_output: false,
            git_chip: GitChip::default(),
            git_lines: Vec::new(),
            git_handle: None,
            git_flight: Coalescer::default(),
            git_expanded: false,
            watch: None,
            watch_rx: None,
            watch_error: None,
            debounce: Debouncer::new(watch::DEBOUNCE_MS),
            epoch: Instant::now(),
            verb: CommandExecutionState::new(),
            dialog: None,
            toasts: Vec::new(),
            advanced_status: None,
            viewer: ViewerState::Closed,
            viewer_rx: None,
            md_cache: CommonMarkCache::default(),
            viewer_w: parse_viewer_width(cc.storage.and_then(|s| s.get_string(VIEWER_W_KEY))),
        };
        match discovery::resolve_repo_root(arg, cwd.as_deref()) {
            Some(root) if discovery::has_tickets_dir(&root) => {
                app.adopt_root(root, &cc.egui_ctx);
            }
            Some(root) => {
                app.state = State::NoRepo {
                    note: Some(format!(
                        "{} has no {}/ directory",
                        root.display(),
                        ticket_engine::repository::TICKETS_DIR
                    )),
                };
            }
            None => {
                // Fall back to the persisted picker choice — revalidated on load.
                let saved = cc
                    .storage
                    .and_then(|s| s.get_string(REPO_ROOT_KEY))
                    .map(PathBuf::from);
                match saved {
                    Some(root) if discovery::has_tickets_dir(&root) => {
                        app.adopt_root(root, &cc.egui_ctx);
                    }
                    Some(root) => {
                        app.state = State::NoRepo {
                            note: Some(format!(
                                "saved path {} no longer contains {}/",
                                root.display(),
                                ticket_engine::repository::TICKETS_DIR
                            )),
                        };
                    }
                    None => {}
                }
            }
        }
        app
    }

    /// A validated repo root becomes active: arm the watch, load the corpus, and
    /// run the launch strict check.
    pub(super) fn adopt_root(&mut self, root: PathBuf, ctx: &egui::Context) {
        self.repo_root = Some(root);
        self.arm_watch(ctx);
        self.start_load(ctx, false);
        self.trigger_check(ctx);
    }

    /// Load the corpus and supplementary datasets on a worker thread. In-place reloads
    /// keep the current board or refusal visible until the replacement result arrives.
    pub(super) fn start_load(&mut self, ctx: &egui::Context, in_place: bool) {
        if let Some(root) = self.repo_root.clone() {
            let repaint_ctx = ctx.clone();
            self.load_rx = Some(background_loading::spawn_load(root, move || {
                repaint_ctx.request_repaint()
            }));
            if !in_place {
                self.state = State::Loading;
            }
        }
    }

    // ---- strict check (trust banner) ----

    /// Run the native folder picker on a worker thread because the picker blocks its caller.
    pub(super) fn start_pick(&mut self, ctx: &egui::Context) {
        if self.pick_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let repaint_ctx = ctx.clone();
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Pick a repo root containing .ai/tickets/")
                .pick_folder();
            let _ = tx.send(picked);
            repaint_ctx.request_repaint();
        });
        self.pick_rx = Some(rx);
    }

    /// open a repo-relative markdown document in the viewer pane —
    /// state → Loading, the read spawned on a worker thread (replacing any
    /// in-flight read; its late result is dropped by `ViewerState::land`).
    /// The escape fence and every failure mode live in `viewer::load_doc`.
    pub(super) fn open_doc(&mut self, rel: String, ctx: &egui::Context) {
        let Some(root) = self.repo_root.clone() else {
            // Unreachable from the detail panel (it only renders over an
            // adopted root), but the machine stays total: an explicit note.
            self.viewer = ViewerState::Fallback {
                path: rel,
                text: String::new(),
                note: "no repo root resolved".to_owned(),
            };
            return;
        };
        self.viewer.open(&rel);
        let repaint_ctx = ctx.clone();
        self.viewer_rx = Some(viewer::spawn_read(root, rel, move || {
            repaint_ctx.request_repaint()
        }));
    }
}
