//! BattlEye RCon packet encoding and decoding.
//!
//! Every datagram, in both directions, reads
//! `'B' 'E' | CRC32 (4 bytes, little-endian) | 0xFF | packet type | payload`. The CRC32 (IEEE)
//! covers every byte from the 0xFF onward. Packet types:
//!
//! - `0x00` login: the client sends the password; the server answers `0x01` (accepted) or
//!   `0x00` (refused).
//! - `0x01` command: the client sends a one-byte sequence number and the command text; the
//!   server answers with the same sequence number and the response text. A response that does
//!   not fit one packet is split, and every part then starts with the header
//!   `0x00 | number of parts | zero-based index of this part`. The response text is never
//!   null-terminated, so a leading `0x00` always introduces that header.
//! - `0x02` server message: the server sends a one-byte sequence number and the message text;
//!   the client acknowledges with the same sequence number.

use thiserror::Error;

const MAGIC: [u8; 2] = *b"BE";
const PAYLOAD_MARKER: u8 = 0xFF;
/// Offset of the 0xFF marker, where the checksummed bytes begin.
const CHECKSUM_START: usize = 6;
/// `'B' 'E'`, the checksum and the 0xFF marker.
const HEADER_LENGTH: usize = 7;

const LOGIN: u8 = 0x00;
const COMMAND: u8 = 0x01;
const SERVER_MESSAGE: u8 = 0x02;
const LOGIN_ACCEPTED: u8 = 0x01;
const LOGIN_REFUSED: u8 = 0x00;
const FRAGMENT_HEADER: u8 = 0x00;

/// A packet the server sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ServerPacket {
    LoginResponse { accepted: bool },
    CommandResponse { sequence: u8, body: ResponseBody },
    ServerMessage { sequence: u8, message: Vec<u8> },
}

/// The body of a command response: the whole response, or one part of a split response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ResponseBody {
    Whole(Vec<u8>),
    Fragment { total: u8, index: u8, part: Vec<u8> },
}

/// Why a datagram is not a valid server packet. Invalid datagrams are dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(super) enum DecodeError {
    #[error("shorter than a BattlEye RCon packet")]
    Truncated,
    #[error("not a BattlEye RCon packet")]
    NotBattlEye,
    #[error("CRC32 mismatch")]
    ChecksumMismatch,
    #[error("unknown packet type {0:#04x}")]
    UnknownPacketType(u8),
    #[error("malformed {0}")]
    Malformed(&'static str),
}

pub(super) fn login_packet(password: &[u8]) -> Vec<u8> {
    datagram(LOGIN, &[password])
}

pub(super) fn command_packet(sequence: u8, command: &[u8]) -> Vec<u8> {
    datagram(COMMAND, &[&[sequence], command])
}

pub(super) fn server_message_acknowledgement(sequence: u8) -> Vec<u8> {
    datagram(SERVER_MESSAGE, &[&[sequence]])
}

fn datagram(packet_type: u8, payload: &[&[u8]]) -> Vec<u8> {
    let payload_length: usize = payload.iter().map(|part| part.len()).sum();
    let mut datagram = Vec::with_capacity(HEADER_LENGTH + 1 + payload_length);
    datagram.extend_from_slice(&MAGIC);
    datagram.extend_from_slice(&[0; 4]);
    datagram.push(PAYLOAD_MARKER);
    datagram.push(packet_type);
    for part in payload {
        datagram.extend_from_slice(part);
    }
    let checksum = crc32fast::hash(&datagram[CHECKSUM_START..]);
    datagram[MAGIC.len()..CHECKSUM_START].copy_from_slice(&checksum.to_le_bytes());
    datagram
}

pub(super) fn decode_server_packet(datagram: &[u8]) -> Result<ServerPacket, DecodeError> {
    if datagram.len() <= HEADER_LENGTH {
        return Err(DecodeError::Truncated);
    }
    if datagram[..MAGIC.len()] != MAGIC || datagram[CHECKSUM_START] != PAYLOAD_MARKER {
        return Err(DecodeError::NotBattlEye);
    }
    let declared = u32::from_le_bytes([datagram[2], datagram[3], datagram[4], datagram[5]]);
    if crc32fast::hash(&datagram[CHECKSUM_START..]) != declared {
        return Err(DecodeError::ChecksumMismatch);
    }
    let payload = &datagram[HEADER_LENGTH + 1..];
    match datagram[HEADER_LENGTH] {
        LOGIN => match payload {
            [LOGIN_ACCEPTED] => Ok(ServerPacket::LoginResponse { accepted: true }),
            [LOGIN_REFUSED] => Ok(ServerPacket::LoginResponse { accepted: false }),
            _ => Err(DecodeError::Malformed("login response")),
        },
        COMMAND => {
            let (&sequence, rest) = payload
                .split_first()
                .ok_or(DecodeError::Malformed("command response"))?;
            Ok(ServerPacket::CommandResponse {
                sequence,
                body: response_body(rest)?,
            })
        }
        SERVER_MESSAGE => {
            let (&sequence, message) = payload
                .split_first()
                .ok_or(DecodeError::Malformed("server message"))?;
            Ok(ServerPacket::ServerMessage {
                sequence,
                message: message.to_vec(),
            })
        }
        other => Err(DecodeError::UnknownPacketType(other)),
    }
}

fn response_body(rest: &[u8]) -> Result<ResponseBody, DecodeError> {
    match rest {
        [FRAGMENT_HEADER, total, index, part @ ..] if *total > 0 && index < total => {
            Ok(ResponseBody::Fragment {
                total: *total,
                index: *index,
                part: part.to_vec(),
            })
        }
        [FRAGMENT_HEADER, ..] => Err(DecodeError::Malformed("multi-packet response header")),
        whole => Ok(ResponseBody::Whole(whole.to_vec())),
    }
}

#[cfg(test)]
#[path = "tests/packet_codec.rs"]
mod tests;
