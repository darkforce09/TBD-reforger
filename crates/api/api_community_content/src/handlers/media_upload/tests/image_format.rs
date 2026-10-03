use super::*;

const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'];
const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
const WEBP: &[u8] = b"RIFF\x24\x00\x00\x00WEBPVP8 ";

#[test]
fn extension_is_read_from_the_last_dot_without_case() {
    for (name, expected) in [
        ("hero.jpg", Some(UploadImageExtension::Jpg)),
        ("HERO.JPEG", Some(UploadImageExtension::Jpeg)),
        ("archive.tar.PnG", Some(UploadImageExtension::Png)),
        ("banner.webp", Some(UploadImageExtension::Webp)),
        ("image.png.exe", None),
        ("image.gif", None),
        ("image.svg", None),
        ("png", None),
        ("image.", None),
        ("", None),
    ] {
        assert_eq!(
            UploadImageExtension::from_file_name(name),
            expected,
            "{name}"
        );
    }
}

#[test]
fn stored_extension_keeps_the_accepted_spelling() {
    assert_eq!(UploadImageExtension::Jpg.as_str(), "jpg");
    assert_eq!(UploadImageExtension::Jpeg.as_str(), "jpeg");
    assert_eq!(UploadImageExtension::Png.as_str(), "png");
    assert_eq!(UploadImageExtension::Webp.as_str(), "webp");
}

#[test]
fn each_extension_accepts_its_own_signature_only() {
    let cases = [
        (UploadImageExtension::Jpg, JPEG),
        (UploadImageExtension::Jpeg, JPEG),
        (UploadImageExtension::Png, PNG),
        (UploadImageExtension::Webp, WEBP),
    ];
    for (extension, own) in cases {
        assert!(extension.matches_signature(own), "{extension:?}");
        for (other, bytes) in cases {
            if other.format_name() != extension.format_name() {
                assert!(
                    !extension.matches_signature(bytes),
                    "{extension:?} must refuse {other:?} bytes"
                );
            }
        }
    }
}

#[test]
fn truncated_empty_and_text_files_are_refused() {
    for extension in [
        UploadImageExtension::Jpg,
        UploadImageExtension::Png,
        UploadImageExtension::Webp,
    ] {
        assert!(!extension.matches_signature(b""), "{extension:?} empty");
        assert!(
            !extension.matches_signature(b"<svg onload=alert(1)>"),
            "{extension:?} text"
        );
    }
    assert!(!UploadImageExtension::Jpg.matches_signature(&JPEG[..2]));
    assert!(!UploadImageExtension::Png.matches_signature(&PNG[..7]));
    assert!(!UploadImageExtension::Webp.matches_signature(b"RIFF\x24\x00\x00\x00WEB"));
    assert!(!UploadImageExtension::Webp.matches_signature(b"RIFF\x24\x00\x00\x00WAVEfmt "));
}
