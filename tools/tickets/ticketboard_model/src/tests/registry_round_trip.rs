//! Byte-exact round trips of the committed registry through the ticketboard's own readers.
//!
//! Every `.ai/tickets/T-*.toml` loaded by the board's corpus loader renders back through
//! `ticket_model`'s canonical encoder to the bytes on disk, and the committed wave lock read by
//! the board's lock reader renders back through `ticket_wave_lock` to the bytes on disk, so the
//! board's readers drop no field the writers keep. The committed estimate files all load without
//! an error row through the board's estimate loader.

use std::fs;

use crate::execution_metrics::estimated::load_raw;
use crate::ticket_registry::services::corpus_loading::load_corpus;
use crate::wave_plan::services::lock_file::{LockState, load_lock};
use ticket_model::TicketId;

fn checkout_root() -> std::path::PathBuf {
    repository_root::find_repository_root().expect("the tests run inside the checkout")
}

#[test]
fn every_registry_ticket_read_by_the_board_renders_back_byte_identical() {
    let root = checkout_root();
    let corpus = load_corpus(&root).expect("the committed registry loads");
    assert!(
        corpus.tickets.len() > 800,
        "the scan must see the committed registry (saw {} tickets)",
        corpus.tickets.len()
    );
    for loaded in &corpus.tickets {
        let disk = fs::read_to_string(&loaded.path).expect("read the ticket file");
        let rendered = ticket_model::render_ticket_toml(&loaded.ticket).expect("render");
        assert!(
            rendered == disk,
            "{}: the board's read renders back differently",
            loaded.path.display()
        );
    }
    println!("{} ticket files byte-identical", corpus.tickets.len());
}

#[test]
fn the_wave_lock_read_by_the_board_renders_back_byte_identical() {
    let root = checkout_root();
    let disk = fs::read_to_string(ticket_wave_lock::lock_path(&root)).expect("read wave.lock");
    let LockState::Loaded(lock) = load_lock(&root) else {
        panic!("the committed wave lock must load");
    };
    let ids = |list: &[String]| list.iter().map(TicketId::new).collect::<Vec<_>>();
    let by_id = |map: &std::collections::BTreeMap<String, Vec<String>>| {
        map.iter()
            .map(|(id, paths)| (TicketId::new(id.as_str()), paths.clone()))
            .collect()
    };
    let writer_shape = ticket_wave_lock::WaveLock {
        version: lock.version,
        max_concurrent: lock.max_concurrent,
        wave_base: lock.wave_base,
        pack_last: ids(&lock.pack_last),
        waves: lock
            .waves
            .iter()
            .map(|wave| ticket_wave_lock::LockWave {
                n: wave.n,
                tickets: ids(&wave.tickets),
            })
            .collect(),
        emptied: Vec::new(),
        owns: by_id(&lock.owns),
        depends_on: by_id(&lock.depends_on),
    };
    let rendered = ticket_wave_lock::render(&writer_shape).expect("render");
    assert!(
        rendered == disk,
        "the board's read of wave.lock renders back differently"
    );
}

#[test]
fn every_committed_estimate_file_loads_through_the_board_without_an_error_row() {
    let root = checkout_root();
    let files = fs::read_dir(root.join(repository_layout::ESTIMATES_DIR))
        .expect("read the estimates folder")
        .count();
    let raw = load_raw(&root);
    assert!(
        raw.errors.is_empty(),
        "estimate error rows: {:?}",
        raw.errors
    );
    assert_eq!(raw.records.len(), files, "every estimate file loads");
}
