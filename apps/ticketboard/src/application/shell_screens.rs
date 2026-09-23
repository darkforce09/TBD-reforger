use super::*;
pub(super) fn topbar_ui(
    ui: &mut Ui,
    repo_root: Option<&Path>,
    busy: bool,
    tab: Tab,
    board_active: bool,
    verb_busy: bool,
    actions: &mut Vec<Action>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ticketboard").strong());
        ui.separator();
        for (t, label) in TABS {
            if ui.selectable_label(tab == t, label).clicked() {
                actions.push(Action::SetTab(t));
            }
        }
        ui.separator();
        match repo_root {
            Some(root) => ui.monospace(root.display().to_string()),
            None => ui.label(RichText::new("no repo").weak()),
        };
        if ui
            .add_enabled(repo_root.is_some() && !busy, Button::new("Reload"))
            .clicked()
        {
            actions.push(Action::Reload);
        }
        if busy {
            ui.add(Spinner::new().size(14.0));
        }
        // mint a new ticket through the one write path.
        if board_active {
            ui.separator();
            if ui
                .add_enabled(!verb_busy, Button::new("New ticket…"))
                .on_hover_text("cargo xtask ticket add <title> [--summary …]")
                .clicked()
            {
                actions.push(Action::OpenDialog(Box::new(mutate::add_dialog())));
            }
        }
    });
}

// ---- full-window states ----

pub(super) fn norepo_ui(ui: &mut Ui, note: Option<&str>, picking: bool, actions: &mut Vec<Action>) {
    ui.add_space(24.0);
    ui.heading("No ticket registry found");
    ui.add_space(8.0);
    ui.label(format!(
        "Ticketboard needs a repo root containing {}/ — two ways to point it at one:",
        ticket_engine::repository::TICKETS_DIR
    ));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("1.");
        ui.label("Pass the repo root as the positional CLI argument:");
        ui.monospace("ticketboard /path/to/repo");
    });
    ui.horizontal(|ui| {
        ui.label("2.");
        ui.label(format!(
            "Launch from anywhere inside the repo — the app walks up from the current \
             directory looking for {}/.",
            ticket_engine::repository::TICKETS_DIR
        ));
    });
    if let Some(note) = note {
        ui.add_space(8.0);
        ui.label(RichText::new(note).color(ui.visuals().warn_fg_color));
    }
    ui.add_space(16.0);
    if ui
        .add_enabled(!picking, Button::new("Pick repo folder…"))
        .clicked()
    {
        actions.push(Action::PickFolder);
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "The picked folder is remembered (eframe storage in your user config dir) \
             and revalidated on the next launch.",
        )
        .weak()
        .small(),
    );
}

pub(super) fn loading_ui(ui: &mut Ui, repo_root: Option<&Path>) {
    ui.add_space(48.0);
    ui.vertical_centered(|ui| {
        ui.add(Spinner::new().size(28.0));
        ui.add_space(12.0);
        if let Some(root) = repo_root {
            ui.monospace(format!(
                "parsing {}/T-*.toml …",
                root.join(ticket_engine::repository::TICKETS_DIR).display()
            ));
        }
    });
}

/// The trust surface: fail-closed refusal naming the file, verbatim error in
/// monospace, and a recovery path that needs no restart (fix on disk → Reload).
pub(super) fn refusal_ui(ui: &mut Ui, error: &LoadError, actions: &mut Vec<Action>) {
    ui.add_space(24.0);
    ui.heading("Ticket corpus refused to load");
    ui.add_space(8.0);
    ui.label(
        "Fail-closed: nothing renders until every ticket parses — no partial board. \
         Fix the file on disk, then Reload; no restart needed.",
    );
    ui.add_space(16.0);
    ui.label(
        RichText::new(error.file.display().to_string())
            .monospace()
            .size(15.0)
            .strong(),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Reload").clicked() {
            actions.push(Action::Reload);
        }
        if ui.button("Reveal in file manager").clicked() {
            let dir = error.file.parent().unwrap_or(&error.file).to_path_buf();
            actions.push(Action::OpenPath(dir));
        }
    });
    ui.add_space(12.0);
    ScrollArea::vertical()
        .id_salt("refusal_error")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // The parse error, VERBATIM — never paraphrased.
            ui.label(RichText::new(&error.error).monospace());
        });
}
