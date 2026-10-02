//! The image formats an upload may carry, told by file extension and by leading bytes.
//!
//! **Role:** [`UploadImageExtension`] names the accepted extensions (`jpg`, `jpeg`, `png`, `webp`),
//! reads one from a client file name, and checks that the file's leading bytes are the signature
//! of that extension's format.
//! **Position:** called by the upload handler in [`super`] before anything is stored; the
//! extension it keeps becomes the stored file's extension and so the served URL's.
//! **Signals & state:** none; pure functions.
//! **Invariants:**
//! - The extension is the text after the file name's last `.`, compared without case; any other
//!   extension, or none, is refused.
//! - A file is accepted only when its leading bytes are its extension's signature: JPEG `FF D8 FF`,
//!   PNG `89 50 4E 47 0D 0A 1A 0A`, WebP `RIFF` then four size bytes then `WEBP`. A PNG named
//!   `.jpg`, a text file named `.png` and an empty file are all refused.

/// The JPEG start-of-image marker followed by the first byte of the next marker.
const JPEG_SIGNATURE: &[u8] = &[0xFF, 0xD8, 0xFF];
/// The eight-byte PNG file signature.
const PNG_SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// The RIFF container tag that opens a WebP file.
const RIFF_TAG: &[u8] = b"RIFF";
/// The RIFF form type of a WebP file, after the four-byte chunk size.
const WEBP_FORM: &[u8] = b"WEBP";
/// Where [`WEBP_FORM`] starts: after [`RIFF_TAG`] and the four-byte chunk size.
const WEBP_FORM_OFFSET: usize = 8;

/// An accepted upload extension, as the stored file keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadImageExtension {
    Jpg,
    Jpeg,
    Png,
    Webp,
}

impl UploadImageExtension {
    /// The accepted extension of `file_name`, compared without case, or `None`.
    pub fn from_file_name(file_name: &str) -> Option<Self> {
        let (_, extension) = file_name.rsplit_once('.')?;
        match extension.to_ascii_lowercase().as_str() {
            "jpg" => Some(Self::Jpg),
            "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }

    /// The extension in lowercase, without the dot.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }

    /// The format's name for people.
    pub fn format_name(self) -> &'static str {
        match self {
            Self::Jpg | Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::Webp => "WebP",
        }
    }

    /// Whether `bytes` open with this extension's format signature.
    pub fn matches_signature(self, bytes: &[u8]) -> bool {
        match self {
            Self::Jpg | Self::Jpeg => bytes.starts_with(JPEG_SIGNATURE),
            Self::Png => bytes.starts_with(PNG_SIGNATURE),
            Self::Webp => {
                bytes.starts_with(RIFF_TAG)
                    && bytes
                        .get(WEBP_FORM_OFFSET..WEBP_FORM_OFFSET + WEBP_FORM.len())
                        .is_some_and(|form| form == WEBP_FORM)
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/image_format.rs"]
mod tests;
