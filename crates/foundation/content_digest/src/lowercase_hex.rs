//! Lowercase hexadecimal spelling of digest bytes.
//!
//! **Role:** turns a digest's bytes into the text every public function of the crate returns.
//! **Position:** called by [`crate::hex_digests`] and [`crate::sha256_hasher`]; private.
//! **Signals & state:** none; one pure function.
//! **Invariants:** two lowercase hex characters per byte, in byte order, no separator.

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Lowercase hex of `bytes`, two characters per byte.
pub(crate) fn lowercase_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}
