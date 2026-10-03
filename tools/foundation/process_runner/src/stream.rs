//! Draining a child's pipes for the whole of its life.
//!
//! **Role:** the threads that drain a child's pipes, whole or line by line.
//! **Position:** private to the crate; `crate::runner` and `crate::run_modes` start them before
//! they wait.
//! **Signals & state:** one reader thread per pipe, joined by the runner or ended by EOF.
//! **Invariants:** every pipe is read to EOF for the child's whole life; text decoding is lossy,
//! and the byte drain decodes nothing.
//!
//! A pipe buffer is about 64 KiB. A parent that reads stdout to the end before touching stderr
//! deadlocks the moment the child fills the stderr buffer: the child blocks writing, the parent
//! blocks reading, and neither moves again. Every capture here therefore hands each pipe to a
//! thread that reads it until EOF, started before the parent waits on the child.
//!
//! Decoding is lossy on purpose. A stray non-UTF-8 byte in a compiler diagnostic or a game log
//! must not leave a gate unable to run — that is exactly the "did not run, reported as a result"
//! shape this crate exists to prevent.

use std::io::{BufRead, BufReader, PipeReader, Read};
use std::process::Child;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

/// The two threads draining a child's separate stdout and stderr pipes.
pub(super) struct SeparateDrains {
    stdout: JoinHandle<Vec<u8>>,
    stderr: JoinHandle<Vec<u8>>,
}

impl SeparateDrains {
    /// Take both pipes off `child` and start reading each on its own thread.
    pub(super) fn start(child: &mut Child) -> SeparateDrains {
        let mut out_pipe = child.stdout.take();
        let mut err_pipe = child.stderr.take();
        SeparateDrains {
            stdout: std::thread::spawn(move || drain(&mut out_pipe)),
            stderr: std::thread::spawn(move || drain(&mut err_pipe)),
        }
    }

    /// Wait for both threads and hand back `(stdout, stderr)`.
    ///
    /// A panicked reader yields an empty string rather than propagating: the child's status is
    /// the verdict, and losing captured text must not turn into losing the exit code.
    pub(super) fn join(self) -> (String, String) {
        let (stdout, stderr) = self.join_bytes();
        (
            String::from_utf8_lossy(&stdout).into_owned(),
            String::from_utf8_lossy(&stderr).into_owned(),
        )
    }

    /// Wait for both threads and hand back `(stdout, stderr)` as the bytes the child wrote.
    pub(super) fn join_bytes(self) -> (Vec<u8>, Vec<u8>) {
        let stdout = self.stdout.join().unwrap_or_default();
        let stderr = self.stderr.join().unwrap_or_default();
        (stdout, stderr)
    }
}

/// Start one thread per pipe of `child` that sends each line, decoded lossily and without its
/// `\n` or `\r\n`, to `lines`; each thread ends at its pipe's EOF or when the receiver is gone.
pub(super) fn start_line_drains(child: &mut Child, lines: &Sender<String>) {
    if let Some(pipe) = child.stdout.take() {
        let lines = lines.clone();
        std::thread::spawn(move || send_lines(pipe, &lines));
    }
    if let Some(pipe) = child.stderr.take() {
        let lines = lines.clone();
        std::thread::spawn(move || send_lines(pipe, &lines));
    }
}

fn send_lines(pipe: impl Read, lines: &Sender<String>) {
    let mut reader = BufReader::new(pipe);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                if buf.last() == Some(&b'\n') {
                    buf.pop();
                    if buf.last() == Some(&b'\r') {
                        buf.pop();
                    }
                }
                let line = String::from_utf8_lossy(&buf).into_owned();
                if lines.send(line).is_err() {
                    return;
                }
            }
        }
    }
}

/// Start the single thread reading a shared pipe that carries stdout and stderr interleaved.
pub(super) fn start_merged_drain(mut reader: PipeReader) -> JoinHandle<String> {
    std::thread::spawn(move || read_to_string_lossy(&mut reader))
}

fn drain(pipe: &mut Option<impl Read>) -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(p) = pipe.as_mut() {
        let _ = p.read_to_end(&mut buf);
    }
    buf
}

/// Read to EOF, replacing invalid UTF-8 instead of refusing the bytes.
pub(super) fn read_to_string_lossy(source: &mut impl Read) -> String {
    let mut buf = Vec::new();
    let _ = source.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}
