# Community Content Handlers (`community_content/handlers/`)

HTTP endpoint controllers managing community doctrine wiki pages, real-time platform announcements, and modpack repository manifests.

---

## 1. Handlers & Route Mappings

### `wiki_doctrine.rs` (<420 LOC)
- **`GET /api/v1/wiki/articles`** (`list_articles`): Lists published wiki articles grouped by category.
- **`GET /api/v1/wiki/articles/{slug}`** (`get_article`): Returns full article markdown and author metadata.
- **`POST /api/v1/admin/wiki/articles`** (`create_article`): Authors a new doctrine article. Requires `admin` or `leader` role.
- **`PATCH /api/v1/admin/wiki/articles/{slug}`** (`update_article`): Updates article content and records a revision entry.
- **`DELETE /api/v1/admin/wiki/articles/{slug}`** (`delete_article`): Unpublishes or deletes an article.

### `announcements.rs` (<320 LOC)
- **`GET /api/v1/announcements`** (`list_active_announcements`): Lists currently active, unexpired announcements.
- **`POST /api/v1/admin/announcements`** (`create_announcement`): Publishes a new announcement. Broadcasts to real-time SSE stream. Requires `admin` role.
- **`DELETE /api/v1/admin/announcements/{id}`** (`dismiss_announcement`): Deletes or expires an announcement.

### `modpacks.rs` (<400 LOC)
- **`GET /api/v1/modpacks`** (`list_modpacks`): Lists available modpack manifests.
- **`GET /api/v1/modpacks/{id}`** (`get_modpack`): Returns detailed list of Workshop addons and download hashes.
- **`POST /api/v1/admin/modpacks`** (`create_modpack`): Registers a new modpack manifest. Requires `admin` role.
- **`PUT /api/v1/admin/modpacks/{id}`** (`update_modpack`): Updates mod manifest items and recalculates total size.
- **`DELETE /api/v1/admin/modpacks/{id}`** (`delete_modpack`): Removes an unused modpack manifest.
