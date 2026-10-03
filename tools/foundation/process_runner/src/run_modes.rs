//! The runs that do not capture a child's output as text.
//!
//! **Role:** the [`crate::Run`] methods for a child that shares this terminal
//! ([`crate::Run::terminal`]), exchanges bytes ([`crate::Run::binary_output`]), writes to files
//! ([`crate::Run::output_to_files`]), outlives this process ([`crate::Run::spawn_detached`]),
//! streams lines to a caller that may kill it ([`crate::Run::stream_lines`]) or replaces this
//! process ([`crate::Run::replace_process`]).
//! **Position:** private to the crate, over the spawn, stdin, isolation, kill and reaping helpers
//! of `crate::runner` and the drains of `crate::stream`; [`BinaryOutput`] and [`StreamingChild`]
//! are re-exported at the crate root.
//! **Signals & state:** none here; each mode owns its child as its own module describes.
//! **Invariants:** a signal is [`verification_core::NotRun::Signalled`], never an exit code, in
//! every mode that reaps its child; every mode but the terminal and the process replacement puts
//! the child in its own session, so a deadline or a kill reaches its whole tree.

mod binary_pipes;
mod detached;
mod file_output;
mod line_stream;
mod process_replacement;
mod terminal;

pub use binary_pipes::BinaryOutput;
pub use line_stream::StreamingChild;
