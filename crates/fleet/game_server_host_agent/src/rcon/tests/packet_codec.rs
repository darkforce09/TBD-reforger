use super::*;

/// Bitwise CRC32 (IEEE, reflected, polynomial 0xEDB88320): a reference independent of the
/// table-driven implementation the codec uses.
fn reference_crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// A server datagram built by hand, checksummed with the reference CRC.
fn server_datagram(checked: &[u8]) -> Vec<u8> {
    let mut datagram = b"BE".to_vec();
    datagram.extend_from_slice(&reference_crc32(checked).to_le_bytes());
    datagram.extend_from_slice(checked);
    datagram
}

#[test]
fn the_reference_crc_matches_the_standard_check_value() {
    assert_eq!(reference_crc32(b"123456789"), 0xCBF4_3926);
}

#[test]
fn a_login_packet_carries_the_password_after_the_login_type() {
    let packet = login_packet(b"range-master");
    let mut expected_checked = vec![0xFF, 0x00];
    expected_checked.extend_from_slice(b"range-master");
    assert_eq!(&packet[..2], b"BE");
    assert_eq!(&packet[6..], expected_checked.as_slice());
    assert_eq!(
        packet[2..6],
        reference_crc32(&expected_checked).to_le_bytes(),
        "little-endian CRC32 of every byte from 0xFF onward"
    );
}

#[test]
fn a_command_packet_carries_its_sequence_number_before_the_command() {
    let packet = command_packet(0xFE, b"#players");
    assert_eq!(&packet[6..9], &[0xFF, 0x01, 0xFE]);
    assert_eq!(&packet[9..], b"#players");
    assert_eq!(packet[2..6], reference_crc32(&packet[6..]).to_le_bytes());
}

#[test]
fn an_empty_command_packet_is_the_two_byte_keep_alive() {
    let packet = command_packet(7, b"");
    assert_eq!(&packet[6..], &[0xFF, 0x01, 7]);
}

#[test]
fn a_server_message_is_acknowledged_with_its_sequence_number() {
    let packet = server_message_acknowledgement(42);
    assert_eq!(&packet[6..], &[0xFF, 0x02, 42]);
    assert_eq!(packet[2..6], reference_crc32(&packet[6..]).to_le_bytes());
}

#[test]
fn login_responses_decode_to_accepted_or_refused() {
    assert_eq!(
        decode_server_packet(&server_datagram(&[0xFF, 0x00, 0x01])),
        Ok(ServerPacket::LoginResponse { accepted: true })
    );
    assert_eq!(
        decode_server_packet(&server_datagram(&[0xFF, 0x00, 0x00])),
        Ok(ServerPacket::LoginResponse { accepted: false })
    );
    assert_eq!(
        decode_server_packet(&server_datagram(&[0xFF, 0x00, 0x01, 0x01])),
        Err(DecodeError::Malformed("login response"))
    );
}

#[test]
fn a_single_packet_command_response_decodes_whole() {
    let mut checked = vec![0xFF, 0x01, 9];
    checked.extend_from_slice(b"Players on server:");
    assert_eq!(
        decode_server_packet(&server_datagram(&checked)),
        Ok(ServerPacket::CommandResponse {
            sequence: 9,
            body: ResponseBody::Whole(b"Players on server:".to_vec()),
        })
    );
    assert_eq!(
        decode_server_packet(&server_datagram(&[0xFF, 0x01, 3])),
        Ok(ServerPacket::CommandResponse {
            sequence: 3,
            body: ResponseBody::Whole(Vec::new()),
        }),
        "the answer to a keep-alive is empty"
    );
}

#[test]
fn a_multi_packet_response_header_is_decoded() {
    let datagram = server_datagram(&[0xFF, 0x01, 4, 0x00, 3, 2, b'o', b'k']);
    assert_eq!(
        decode_server_packet(&datagram),
        Ok(ServerPacket::CommandResponse {
            sequence: 4,
            body: ResponseBody::Fragment {
                total: 3,
                index: 2,
                part: b"ok".to_vec(),
            },
        })
    );
}

#[test]
fn inconsistent_multi_packet_headers_are_malformed() {
    for checked in [
        vec![0xFF, 0x01, 4, 0x00, 0, 0],
        vec![0xFF, 0x01, 4, 0x00, 2, 2],
        vec![0xFF, 0x01, 4, 0x00, 2],
    ] {
        assert_eq!(
            decode_server_packet(&server_datagram(&checked)),
            Err(DecodeError::Malformed("multi-packet response header")),
            "{checked:?}"
        );
    }
}

#[test]
fn a_server_message_decodes_with_its_sequence_number() {
    let mut checked = vec![0xFF, 0x02, 17];
    checked.extend_from_slice(b"Player #3 connected");
    assert_eq!(
        decode_server_packet(&server_datagram(&checked)),
        Ok(ServerPacket::ServerMessage {
            sequence: 17,
            message: b"Player #3 connected".to_vec(),
        })
    );
}

#[test]
fn corrupted_packets_are_rejected() {
    let valid = server_datagram(&[0xFF, 0x01, 1, b'o', b'k']);
    for position in 6..valid.len() {
        let mut corrupted = valid.clone();
        corrupted[position] ^= 0x20;
        let decoded = decode_server_packet(&corrupted);
        if position == 6 {
            assert_eq!(decoded, Err(DecodeError::NotBattlEye));
        } else {
            assert_eq!(
                decoded,
                Err(DecodeError::ChecksumMismatch),
                "byte {position}"
            );
        }
    }
    let mut wrong_checksum = valid.clone();
    wrong_checksum[3] ^= 0x01;
    assert_eq!(
        decode_server_packet(&wrong_checksum),
        Err(DecodeError::ChecksumMismatch)
    );
}

#[test]
fn foreign_and_truncated_datagrams_are_rejected() {
    assert_eq!(
        decode_server_packet(b"BE\0\0\0\0\xFF"),
        Err(DecodeError::Truncated)
    );
    let mut foreign = server_datagram(&[0xFF, 0x01, 1]);
    foreign[0] = b'X';
    assert_eq!(
        decode_server_packet(&foreign),
        Err(DecodeError::NotBattlEye)
    );
    assert_eq!(
        decode_server_packet(&server_datagram(&[0xFF, 0x07, 1])),
        Err(DecodeError::UnknownPacketType(0x07))
    );
    assert_eq!(
        decode_server_packet(&server_datagram(&[0xFF, 0x01])),
        Err(DecodeError::Malformed("command response"))
    );
}
