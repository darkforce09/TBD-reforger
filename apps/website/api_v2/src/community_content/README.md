# Community Content Subsystem (`community_content/`)

Front-page announcements, CMS publishing, Discord webhook integration, multipart media asset uploads, tactical doctrine knowledgebase, vehicle IFF catalog, and modpack manifests.

---

## 1. Subsystem Topology & Responsibilities

The `community_content/` domain decomposes `cms.rs` (693 LOC) and `modpacks.rs` (507 LOC) into cohesive modules <350 LOC:

```text
src/community_content/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/content & /api/v1/cms sub-router (<80 LOC)
│
├── models/
│   ├── mod.rs
│   └── content.rs                      <-- Announcement, WikiPage, Vehicle, Modpack, ModpackMod (<135 LOC)
│
├── handlers/
│   ├── mod.rs
│   ├── announcements_public.rs         <-- Public member feeds (<70 LOC)
│   ├── announcements_admin.rs          <-- CMS announcement CRUD & audit (<350 LOC)
│   ├── discord_webhook_push.rs         <-- Manual announcement push to Discord (<150 LOC)
│   ├── media_upload.rs                 <-- Multipart image upload & magic byte verification (<180 LOC)
│   ├── wiki_knowledgebase.rs           <-- Markdown tactical doctrine & SOP articles (<200 LOC)
│   ├── vehicle_database.rs             <-- Vehicle technical specs & IFF index (<120 LOC)
│   ├── modpack_catalog.rs              <-- Public modpack list & current manifest (<160 LOC)
│   └── modpack_admin.rs                <-- Modpack authoring, replace & active toggle (<340 LOC)
│
├── services/
│   ├── webhook_client.rs               <-- Discord webhook embed dispatcher (<170 LOC)
│   └── tests/webhook_client.rs         <-- Sibling unit tests (<100 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── announcements_admin.rs
    ├── media_upload.rs
    ├── wiki_knowledgebase.rs
    └── modpack_admin.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `GET` | `/api/v1/announcements` | `announcements_public::list_announcements` | `AuthUser` | Public feed of published announcements (pinned first). |
| `GET` | `/api/v1/announcements/{id}` | `announcements_public::get_announcement` | `AuthUser` | Read single announcement. |
| `GET` | `/api/v1/cms/announcements` | `announcements_admin::list_cms_announcements` | `AdminUser` | CMS paginated list including drafts. |
| `POST` | `/api/v1/cms/announcements` | `announcements_admin::create_announcement` | `AdminUser` | Create announcement; optionally triggers Discord webhook. |
| `PATCH`| `/api/v1/cms/announcements/{id}` | `announcements_admin::update_announcement` | `AdminUser` | Selective field updates; records audit log. |
| `DELETE`| `/api/v1/cms/announcements/{id}`| `announcements_admin::delete_announcement` | `AdminUser` | Soft-delete announcement (`deleted_at = now()`). |
| `POST` | `/api/v1/cms/announcements/{id}/push-discord`| `discord_webhook_push::push_announcement_discord`| `AdminUser`| Manually push announcement embed to Discord webhook. |
| `POST` | `/api/v1/cms/uploads` | `media_upload::upload_image` | `AdminUser` | Multipart image upload (custom 6 MB body limit). |
| `GET` | `/api/v1/wiki` | `wiki_knowledgebase::list_wiki` | `AuthUser` | List SOP doctrine articles ordered by `nav_order`. |
| `GET` | `/api/v1/wiki/{slug}` | `wiki_knowledgebase::get_wiki_page` | `AuthUser` | Retrieve markdown doctrine page by URL slug. |
| `PUT` | `/api/v1/wiki/{slug}` | `wiki_knowledgebase::upsert_wiki_page` | `AdminUser` | Create or replace doctrine page. |
| `GET` | `/api/v1/vehicle-database` | `vehicle_database::list_vehicles` | `AuthUser` | Filterable vehicle catalog & IFF index. |
| `POST` | `/api/v1/vehicle-database` | `vehicle_database::create_vehicle` | `AdminUser` | Add vehicle specification row. |
| `GET` | `/api/v1/modpacks` | `modpack_catalog::list_modpacks` | `AuthUser` | List all community modpacks. |
| `GET` | `/api/v1/modpacks/current` | `modpack_catalog::get_current_modpack` | `AuthUser` | Retrieve active modpack manifest with mod list. |
| `POST` | `/api/v1/modpacks` | `modpack_admin::create_modpack` | `AdminUser` | Create modpack container with nested mods. |
| `PUT` | `/api/v1/modpacks/{id}` | `modpack_admin::replace_modpack` | `AdminUser` | Atomic wholesale replacement of modpack and mod list. |
| `POST` | `/api/v1/modpacks/{id}/set-current`| `modpack_admin::set_current_modpack` | `AdminUser` | Switch active community modpack. |
| `DELETE`| `/api/v1/modpacks/{id}` | `modpack_admin::delete_modpack` | `AdminUser` | Delete modpack (verified zero references in servers/events). |

---

## 3. Key Invariants & Security Controls

### 3.1 Discord Webhook Sanitization (`sanitize_discord_embed_field`)
Discord embeds are sanitized prior to delivery:
- Control characters (ASCII 0–31, 127) are stripped.
- Cells starting with formula characters (`=`, `+`, `-`, `@`) are prepended with a Zero-Width Space (`\u{200B}`) to prevent formula execution if copied out of Discord into spreadsheet software.

### 3.2 Media Upload Magic Byte Verification (`media_upload.rs`)
Uploaded files are capped at 5 MB and validated against image magic byte signatures (JPEG: `FF D8 FF`, PNG: `89 50 4E 47`, WebP: `52 49 46 46 ... 57 45 42 50`, GIF: `47 49 46 38`). Files with invalid magic bytes are rejected with HTTP 400.
