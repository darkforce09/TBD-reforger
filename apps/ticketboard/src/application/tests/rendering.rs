use super::*;
use crate::test_support::{corpus_of, work};
use crate::ticket_actions::services::{commands, dialog_builders};
use crate::wave_plan::services::lock_file::{LockWave, WaveLock};

fn workspace() -> WorkspaceState {
    let corpus = corpus_of(vec![
        work("T-1", "status = \"idea\"", ""),
        work("T-2", "status = \"queued\"\norder = 10", ""),
    ]);
    let lock = WaveLock {
        version: 1,
        max_concurrent: 2,
        wave_base: 0,
        pack_last: vec![],
        waves: vec![LockWave {
            n: 1,
            tickets: vec!["T-2".into()],
        }],
        owns: Default::default(),
        depends_on: Default::default(),
    };
    WorkspaceState::new(
        corpus,
        LockState::Loaded(lock),
        MetricsState::NoReceipts,
        estimates::RawEstimates::default(),
        None,
        ReloadPreferences::default(),
    )
}

fn render(mut paint: impl FnMut(&mut Ui)) -> String {
    let context = egui::Context::default();
    let mut text = String::new();
    // Areas and panels settle their sizes on the first frame.
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1500.0, 950.0),
            )),
            ..Default::default()
        };
        let mut output = context.run_ui(input, |ui| paint(ui));
        output.textures_delta.clear();
        for shape in output.shapes {
            collect_text(&shape.shape, &mut text);
        }
    }
    assert!(!text.is_empty(), "the UI must actually emit text shapes");
    text
}

fn collect_text(shape: &egui::epaint::Shape, text: &mut String) {
    match shape {
        egui::epaint::Shape::Text(shape) => {
            text.push_str(shape.galley.text());
            text.push('\n');
        }
        egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                collect_text(shape, text);
            }
        }
        _ => {}
    }
}

#[test]
fn every_tab_and_ticket_details_render_without_dispatching_commands() {
    let mut board = workspace();
    board.selected = Some(0);
    let mutation = MutationContext {
        repo_root: None,
        busy: false,
    };
    let mut events = Vec::new();
    let text = render(|ui| board_ui(ui, &board, mutation, &mut events));
    assert!(text.contains("idea") && text.contains("T-1"));
    let text = render(|ui| tree_ui(ui, &board, &mut events));
    assert!(text.contains("T-1") && text.contains("T-2"));
    let text = render(|ui| waves_ui(ui, &board, &mut events));
    assert!(text.contains("T-2") && text.contains("Unplanned"));
    let text = render(|ui| metrics_ui(ui, &board, &mut events));
    assert!(text.contains("MEASURED") && text.contains("ESTIMATED"));
    let text = render(|ui| detail_ui(ui, mutation, &board, 0, &mut None, &mut events));
    assert!(text.contains("title of T-1") && text.contains("status"));
    assert!(
        events.is_empty(),
        "passive rendering must not dispatch any action"
    );
}

#[test]
fn mutation_dialog_variants_render_without_executing_their_commands() {
    let board = workspace();
    let context = mutate::context(&board);
    let guard = commands::guard_for(&board.corpus.tickets[0].path);
    let dialogs = [
        dialog_builders::transition_dialog(&context, 0, commands::Transition::Ship, guard.clone()),
        dialog_builders::anchor_dialog(&context, 0),
        dialog_builders::transition_dialog(
            &context,
            1,
            commands::Transition::MarkReady,
            guard.clone(),
        ),
        dialog_builders::add_dialog(),
        dialog_builders::add_child_dialog(&context, 0, guard.clone()),
        dialog_builders::remove_dialog(&context, 0, guard),
    ];
    for mut dialog in dialogs {
        let mut events = Vec::new();
        render(|ui| {
            mutate::dialog_ui(
                ui.ctx(),
                &board,
                MutationContext {
                    repo_root: None,
                    busy: false,
                },
                &mut dialog,
                &mut events,
            );
        });
        assert!(
            events.is_empty(),
            "opening a dialog must never run its command"
        );
    }
}

#[test]
fn refusal_and_document_states_render_their_actual_messages() {
    let mut events = Vec::new();
    let text = render(|ui| norepo_ui(ui, Some("invalid folder"), false, &mut events));
    assert!(text.contains("invalid folder"));
    let error = LoadError {
        file: "broken.toml".into(),
        error: "verbatim parse refusal".into(),
    };
    let text = render(|ui| refusal_ui(ui, &error, &mut events));
    assert!(text.contains("broken.toml") && text.contains("verbatim parse refusal"));
    let mut cache = CommonMarkCache::default();
    for state in [
        ViewerState::Loading {
            path: "docs/plan.md".into(),
        },
        ViewerState::Rendered {
            path: "docs/plan.md".into(),
            text: "# Mission plan".into(),
        },
        ViewerState::Fallback {
            path: "docs/plan.md".into(),
            text: "raw document".into(),
            note: "read refusal".into(),
        },
    ] {
        let text = render(|ui| viewer_pane_ui(ui, &state, &mut cache, None, &mut events));
        assert!(text.contains("docs/plan.md"));
    }
    assert!(events.is_empty());
}
