//! Sequence numbers of command packets.
//!
//! The number is one byte: the client starts at 0 and counts up, and after 255 the count wraps
//! to 0 and the numbers are reused. A command keeps its number for every retransmission, so a
//! late or repeated answer to an earlier command never completes a later one while fewer than
//! 256 commands separate them.

#[derive(Debug, Default)]
pub(super) struct CommandSequence {
    next: u8,
}

impl CommandSequence {
    /// The number for the next command.
    pub(super) fn allocate(&mut self) -> u8 {
        let sequence = self.next;
        self.next = sequence.wrapping_add(1);
        sequence
    }
}

#[cfg(test)]
#[path = "tests/command_sequence.rs"]
mod tests;
