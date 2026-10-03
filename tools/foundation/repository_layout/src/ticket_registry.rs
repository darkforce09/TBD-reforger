//! The ticket registry's files.
//!
//! **Role:** the repository-relative paths of the registry folder and the schemas, vocabulary,
//! wave lock, queue, receipts and estimates beside the ticket files.
//! **Position:** `ticket_engine` reads and writes them; `xtask` and `ticketboard` read the same
//! files, so all three resolve them here.
//! **Signals & state:** none; constants.
//! **Invariants:** every path lies under [`TICKETS_DIR`].

/// The registry itself: one `T-<id>.toml` per ticket, parents and children alike, beside the
/// schemas and receipts that describe them.
pub const TICKETS_DIR: &str = ".ai/tickets";

/// Draft 2020-12 schema every ticket file is validated against by `ticket check`.
pub const SCHEMA: &str = ".ai/tickets/schema.json";

/// The four-level domain → layer → component → surface word list a ticket's `[scope]` block is
/// resolved against at every corpus load.
pub const SCOPE_VOCAB: &str = ".ai/tickets/scope-vocab.toml";

/// The corpus facts no ticket file states: the ids that must never be minted, and which ticket
/// implements an editor gap row the ticket itself does not claim.
pub const CORPUS_PINS: &str = ".ai/tickets/corpus-pins.toml";

/// The wave plan, compiled from the ticket files by `cargo xtask wave repack`, its one writer.
pub const WAVE_LOCK: &str = ".ai/tickets/wave.lock";

/// The dispatch queue `ticket sync` regenerates: batch size, concurrency, worktree base, and the
/// ready tickets in order.
pub const QUEUE_JSON: &str = ".ai/tickets/queue.json";

/// Run receipts, one `<ticket id>/` subtree each. Outside the ticket files and the wave lock, so
/// parallel lands touch disjoint subtrees and never a shared file.
pub const METRICS_DIR: &str = ".ai/tickets/metrics";

/// The committed schema every run receipt must satisfy.
pub const METRICS_SCHEMA: &str = ".ai/tickets/metrics.schema.json";

/// Token estimates, one file per ticket, outside [`METRICS_DIR`] because an estimate is written
/// before the work and a receipt after it.
pub const ESTIMATES_DIR: &str = ".ai/tickets/estimates";

/// The committed schema every estimate file must satisfy.
pub const ESTIMATES_SCHEMA: &str = ".ai/tickets/estimates.schema.json";
