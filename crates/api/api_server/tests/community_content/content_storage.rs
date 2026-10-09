//! The image upload (`POST /api/v1/cms/uploads`) through the real HTTP router: an accepted image
//! sits in the upload directory under its public name, byte for byte, beside no staging file, and
//! a member or anonymous caller is refused.
//!
//! Each case owns its administrator, member and upload directory. Upload answers are checked
//! against `content-upload.schema.json`.

use crate::{content_support, contract_support};

use axum::http::StatusCode;
use serde_json::Value;

use content_support::{
    ContentSuite, assert_refusal, directory_entries, jpeg_bytes, png_bytes, webp_bytes,
};
use contract_support::assert_valid;

const SUITE: &str = "content_storage";
const UPLOAD_CONTRACT: &str = "content-upload.schema.json";

/// The stored file name of an accepted upload's `url`.
fn stored_name(answer: &Value) -> String {
    answer["url"]
        .as_str()
        .expect("the `url` field is a string")
        .strip_prefix("/uploads/")
        .expect("the stored upload URL starts with /uploads/")
        .to_owned()
}

#[tokio::test]
async fn content_storage_upload_stores_the_file_whole_with_no_staging_leftovers() {
    let suite = ContentSuite::new(SUITE).await;
    let uploads = [
        ("map-overlay.png", png_bytes(2 << 20), "png"),
        ("PHOTO.JPEG", jpeg_bytes(4096), "jpeg"),
        ("snapshot.jpg", jpeg_bytes(64), "jpg"),
        ("banner.webp", webp_bytes(512), "webp"),
    ];
    let mut stored = Vec::new();
    for (file_name, bytes, extension) in uploads {
        let (status, answer) = suite
            .upload_file(Some(&suite.admin), file_name, &bytes)
            .await;
        assert_eq!(status, StatusCode::CREATED, "{file_name}: {answer}");
        assert_valid(UPLOAD_CONTRACT, Some("UploadResponse"), &answer);
        let name = stored_name(&answer);
        assert!(name.ends_with(&format!(".{extension}")), "{name}");

        let on_disk = std::fs::read(suite.upload_dir.join(&name)).unwrap();
        assert!(on_disk == bytes, "{file_name} is stored whole");
        let (status, served) = suite
            .send_bytes(
                Some(&suite.member),
                "GET",
                answer["url"].as_str().unwrap(),
                None,
                Vec::new(),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert!(served == bytes, "{file_name} is served whole");
        stored.push(name);
    }
    stored.sort();
    assert_eq!(
        directory_entries(&suite.upload_dir),
        stored,
        "the directory holds the stored files and no staging file"
    );
}

#[tokio::test]
async fn content_storage_member_upload_answers_403_and_anonymous_401() {
    let suite = ContentSuite::new(SUITE).await;
    let bytes = png_bytes(256);
    let (status, answer) = suite
        .upload_file(Some(&suite.member), "member.png", &bytes)
        .await;
    assert_refusal(status, &answer, StatusCode::FORBIDDEN, None);
    let (status, answer) = suite.upload_file(None, "anonymous.png", &bytes).await;
    assert_refusal(status, &answer, StatusCode::UNAUTHORIZED, None);
    assert!(directory_entries(&suite.upload_dir).is_empty());
}
