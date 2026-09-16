//! Role: raw.
//! Position: `terrain/dem` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::formats::archives::codec::BinaryError;
use crate::formats::containers::header::ContainerHeader;
use crate::formats::containers::header::HEADER_BYTES;
use crate::formats::containers::tbde::TbdeHeader;

/// A decoded `dem/elevation.dem`: the header verbatim plus its `width * height` row-major samples.
#[derive(Clone, Debug, PartialEq)]
pub struct RawDem {
    /// Header.
    pub header: TbdeHeader,

    /// Samples.
    pub samples: Vec<u16>,
}

impl RawDem {
    /// Decode a complete `TBDE` buffer.
    pub fn parse(bytes: &[u8]) -> Result<Self, BinaryError> {
        let total = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        let mut sink = RawDemSink::new(total);
        sink.push(bytes)?;
        sink.finish()
    }

    /// Width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.header.width
    }

    /// Height.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.header.height
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.header.width || y >= self.header.height {
            return None;
        }
        (y as usize)
            .checked_mul(self.header.width as usize)?
            .checked_add(x as usize)
    }

    /// The quantised sample at `(x, y)`, or `None` outside the grid.
    #[must_use]
    pub fn sample_u16(&self, x: u32, y: u32) -> Option<u16> {
        self.samples.get(self.index(x, y)?).copied()
    }

    /// Elevation at `(x, y)` in metres — `offset_m + sample * scale_m`, computed on demand.
    #[must_use]
    pub fn metres(&self, x: u32, y: u32) -> Option<f32> {
        Some(self.header.metres(self.sample_u16(x, y)?))
    }

    /// The whole grid as `f32` metres.
    #[must_use]
    pub fn metres_grid(&self) -> Vec<f32> {
        self.samples
            .iter()
            .map(|&s| self.header.metres(s))
            .collect()
    }
}

/// Incremental `TBDE` decoder: feed it the response's chunks, take a [`RawDem`] at the end.
pub struct RawDemSink {
    total_bytes: u64,

    head: [u8; HEADER_BYTES],
    head_len: usize,
    header: Option<TbdeHeader>,

    samples: Vec<u16>,

    filled: usize,

    pending: Option<u8>,

    payload_seen: usize,
}

impl RawDemSink {
    /// A sink for a file of exactly `total_bytes` bytes (`content-length`, or the buffer length).
    #[must_use]
    pub fn new(total_bytes: u64) -> Self {
        Self {
            total_bytes,
            head: [0; HEADER_BYTES],
            head_len: 0,
            header: None,
            samples: Vec::new(),
            filled: 0,
            pending: None,
            payload_seen: 0,
        }
    }

    /// The parsed header, once enough bytes have arrived for it.
    #[must_use]
    pub fn header(&self) -> Option<&TbdeHeader> {
        self.header.as_ref()
    }

    /// Feed the next chunk. Header bytes first, then samples, little-endian.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), BinaryError> {
        let mut rest = chunk;
        if self.header.is_none() {
            let want = HEADER_BYTES - self.head_len;
            let take = want.min(rest.len());
            self.head[self.head_len..self.head_len + take].copy_from_slice(&rest[..take]);
            self.head_len += take;
            rest = &rest[take..];
            if self.head_len < HEADER_BYTES {
                return Ok(());
            }
            self.begin_payload()?;
        }
        self.decode_payload(rest)
    }

    fn begin_payload(&mut self) -> Result<(), BinaryError> {
        let (header, _) = TbdeHeader::read(&self.head)?;

        let declared = header
            .payload_bytes()
            .and_then(|p| p.checked_add(HEADER_BYTES))
            .and_then(|f| u64::try_from(f).ok())
            .ok_or(BinaryError::LengthMismatch {
                what: "TBDE",
                expected: usize::MAX,
                actual: 0,
            })?;
        if declared != self.total_bytes {
            return Err(BinaryError::LengthMismatch {
                what: "TBDE",
                expected: usize::try_from(declared).unwrap_or(usize::MAX),
                actual: usize::try_from(self.total_bytes).unwrap_or(usize::MAX),
            });
        }

        let count = header.sample_count().ok_or(BinaryError::LengthMismatch {
            what: "TBDE",
            expected: usize::MAX,
            actual: 0,
        })?;
        self.samples = vec![0_u16; count];
        self.header = Some(header);
        Ok(())
    }

    fn decode_payload(&mut self, bytes: &[u8]) -> Result<(), BinaryError> {
        self.payload_seen = self.payload_seen.saturating_add(bytes.len());
        let mut rest = bytes;
        if let Some(lo) = self.pending.take() {
            let Some((&hi, tail)) = rest.split_first() else {
                self.pending = Some(lo);
                return Ok(());
            };
            self.store(u16::from_le_bytes([lo, hi]))?;
            rest = tail;
        }
        let pairs = rest.len() / 2;

        if cfg!(target_endian = "little")
            && pairs > 0
            && let Ok(words) = bytemuck::try_cast_slice::<u8, u16>(&rest[..pairs * 2])
        {
            let end = self.filled.saturating_add(words.len());
            if end > self.samples.len() {
                return Err(self.overrun());
            }
            self.samples[self.filled..end].copy_from_slice(words);
            self.filled = end;
        } else {
            for pair in rest[..pairs * 2].chunks_exact(2) {
                self.store(u16::from_le_bytes([pair[0], pair[1]]))?;
            }
        }
        if rest.len() % 2 == 1 {
            self.pending = Some(rest[rest.len() - 1]);
        }
        Ok(())
    }

    fn store(&mut self, sample: u16) -> Result<(), BinaryError> {
        let Some(slot) = self.samples.get_mut(self.filled) else {
            return Err(self.overrun());
        };
        *slot = sample;
        self.filled += 1;
        Ok(())
    }

    fn overrun(&self) -> BinaryError {
        BinaryError::LengthMismatch {
            what: "TBDE",
            expected: self.samples.len() * size_of::<u16>(),
            actual: self.payload_seen,
        }
    }

    /// Finish: the header must have arrived and the payload must be exactly what it declared.
    pub fn finish(self) -> Result<RawDem, BinaryError> {
        let Some(header) = self.header else {
            return Err(BinaryError::Truncated {
                what: "TBDE",
                expected: HEADER_BYTES,
                actual: self.head_len,
            });
        };
        if self.filled != self.samples.len() || self.pending.is_some() {
            return Err(BinaryError::LengthMismatch {
                what: "TBDE",
                expected: self.samples.len() * size_of::<u16>(),
                actual: self.payload_seen,
            });
        }
        Ok(RawDem {
            header,
            samples: self.samples,
        })
    }
}

/// Frame a header and its samples into the `TBDE` bytes an emitter writes.
#[must_use]
pub fn to_bytes(header: &TbdeHeader, samples: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_BYTES + size_of_val(samples));
    out.extend_from_slice(&header.to_header_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[cfg(test)]
#[path = "tests/raw_tests.rs"]
mod tests;
