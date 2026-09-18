# Community Content Models (`community_content/models/`)

Database schemas and wire transfer representations for doctrine articles, wiki documents, community announcements, and modpack distribution manifests.

---

## 1. Domain Entities & Schemas

### `article.rs` (<350 LOC)
- **Database Table**: `articles`, `article_revisions`
- **Fields**:
  - `id: Uuid`: Unique article identifier.
  - `slug: String`: URL-safe unique path (e.g., `infantry-squad-tactics`).
  - `title: String`: Display headline.
  - `category: String`: Categorization (e.g., `Doctrine`, `Standard Operating Procedures`, `Vehicle Manuals`).
  - `content_markdown: String`: Raw markdown source body.
  - `author_id: String`: Discord snowflake of the author.
  - `is_published: bool`: Publication state flag.
  - `revision_number: u32`: Monotonically increasing edit counter.
  - `created_at: DateTime<Utc>`, `updated_at: DateTime<Utc>`.

### `announcement.rs` (<220 LOC)
- **Database Table**: `announcements`
- **Fields**:
  - `id: Uuid`: Unique announcement identifier.
  - `title: String`: Headline text.
  - `body_markdown: String`: Announcement message body.
  - `severity: AnnouncementSeverity`: `Info`, `Warning`, `Critical`.
  - `starts_at: DateTime<Utc>`: Scheduled visibility start.
  - `expires_at: Option<DateTime<Utc>>`: Auto-dismissal timestamp.
  - `created_by: String`: Author Discord snowflake.

### `modpack.rs` (<320 LOC)
- **Database Table**: `modpacks`, `modpack_entries`
- **Fields**:
  - `id: Uuid`: Modpack manifest identifier.
  - `name: String`: Unique package name (e.g., `TBD Primary Modern v4.2`).
  - `version: String`: Semantic version string.
  - `total_size_bytes: u64`: Aggregate byte size of all included addons.
  - `is_default: bool`: Server default flag.
  - `entries: Vec<ModpackEntry>`: Child list of Workshop items, GUIDs, versions, and hashes.
