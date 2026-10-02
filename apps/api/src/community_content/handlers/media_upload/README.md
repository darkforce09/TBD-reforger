# Media upload handler

The content manager's image upload: `POST /api/v1/cms/uploads` takes one image from an
administrator, stores it in the upload directory and answers the URL it is served at.

## Contents

```text
apps/api/src/community_content/handlers/media_upload/
├── image_format.rs  the accepted extensions and the leading-byte signature of each format
├── mod.rs           `upload_image`, the route handler: read, check, store, answer
├── tests/           unit tests of the format checks and of the upload directory writer
└── upload_store.rs  the upload directory writer: staging file, flush, atomic rename
```

## How it works

The handler takes `AdminUser` and a multipart body under the route's body limit
(`core::middleware::MAX_MULTIPART_BODY`). It reads the first field named `file`, skipping the
others, and answers:

| Case | Answer |
|---|---|
| body over the limit, or a file over `MAX_UPLOAD_BYTES` (5 MiB) | 413, `details.code = request_too_large` |
| body that is not multipart or cannot be read | axum's own status and reason (400) |
| no `file` field | 400 |
| extension other than `jpg`, `jpeg`, `png`, `webp` (checked before the bytes are read) | 415 |
| leading bytes that are not the extension's format | 415 |
| storage failure (logged with the io error) | 503, `details.code = storage_unavailable` |
| stored | 201 `{"url": "/uploads/<uuid-v4>.<extension>"}` |

The file is stored under a fresh UUID with the lowercase extension it arrived with. The writer
creates the directory when it is missing, writes the bytes to a staging file with a random
dot-name, flushes it and renames it to the public name, so `/uploads/<name>` serves the whole file
or nothing. A failed write removes its staging file. The answered URL is checked against the
contract type `UploadResponseUrl` before it is sent.

## Boundaries

- Depends on: `models::generated::content_upload` (`UploadResponse`, `UploadResponseUrl`,
  `ContentRefusalCode`); `core` for the application state, the `AdminUser` extractor, `ApiError`
  and `Config::upload_dir` (`UPLOAD_DIR`); tokio for the file writes.
- Used by: the domain's `routes.rs`, which registers `upload_image` behind the multipart body
  limit; over HTTP, the hero upload of the content manager under
  `apps/frontend/src/v2/pages/administration/content_manager/`.
- Rules: this folder is the only writer of the upload directory, which `core::http_router` serves
  at `/uploads`; the wire shapes are `contracts/definitions/content-upload.schema.json`.

## Related documentation

- [Administration and community content design](/documentation/apps/api/verification_evidence/administration_and_content.md)
  — the upload semantics this handler implements.
- [API environment variables](/documentation/apps/api/environment_variables.md) —
  `UPLOAD_DIR`.
- [Content manager page](/documentation/apps/frontend/pages/administration/content_manager/content_manager_page.md)
  — the page that uploads through this route.
