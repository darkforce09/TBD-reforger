# Community Content Tests (`community_content/tests/`)

Sibling unit test specifications for wiki revisioning, markdown security sanitation, announcement expiration schedules, and modpack deltas.

---

## 1. Test Modules

Declared via Monorepo Law #7 (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

### `wiki_doctrine.rs`
- **Coverage**: Slug uniqueness collisions, category filtering, unauthenticated edit rejections, revision history increments, and soft deletion.

### `announcements.rs`
- **Coverage**: Expiration date filtering (excluding expired notices), priority ordering, and SSE broadcast message structure.

### `modpacks.rs`
- **Coverage**: Addon version validation, size calculation aggregations, workshop URL format validation, and default manifest toggling.

### `markdown_sanitizer.rs`
- **Coverage**: XSS injection payloads (script tags, iframe embeds, malicious event handlers), table-of-contents extraction, and malformed frontmatter.

### `modpack_delta_calculator.rs`
- **Coverage**: Addon addition, removal, and version upgrade delta computations.
