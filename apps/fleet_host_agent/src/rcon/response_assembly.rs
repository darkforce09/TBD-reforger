//! Reassembly of command responses the server split into several packets.
//!
//! Every part names the number of parts and its own zero-based index, and UDP may deliver the
//! parts in any order, more than once, or not at all. A part is kept the first time its index
//! arrives; the response is complete when every index has arrived. The parts are joined in
//! index order as bytes, before any text decoding, so a character split across two parts
//! survives.

use super::packet_codec::ResponseBody;

#[derive(Debug, Default)]
pub(super) struct ResponseAssembly {
    parts: Vec<Option<Vec<u8>>>,
}

impl ResponseAssembly {
    /// Takes one answer to the command and returns the whole response once it is complete.
    /// A part whose number of parts differs from the first part's is ignored.
    pub(super) fn accept(&mut self, body: ResponseBody) -> Option<Vec<u8>> {
        let (total, index, part) = match body {
            ResponseBody::Whole(response) => return Some(response),
            ResponseBody::Fragment { total, index, part } => {
                (usize::from(total), usize::from(index), part)
            }
        };
        if self.parts.is_empty() {
            self.parts = vec![None; total];
        }
        if self.parts.len() != total {
            return None;
        }
        let slot = self.parts.get_mut(index)?;
        if slot.is_none() {
            *slot = Some(part);
        }
        self.parts.iter().all(Option::is_some).then(|| {
            self.parts
                .iter()
                .flatten()
                .flat_map(|part| part.iter().copied())
                .collect()
        })
    }
}

#[cfg(test)]
#[path = "tests/response_assembly.rs"]
mod tests;
