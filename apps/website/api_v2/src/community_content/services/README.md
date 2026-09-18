# Community Content Services (`community_content/services/`)

Domain processing services for markdown sanitation, document indexing, and modpack manifest diffing.

---

## 1. Domain Services

### `markdown_sanitizer.rs` (<350 LOC)
- **Purpose**: Parses, validates, and cleans user-submitted Markdown content to prevent cross-site scripting (XSS) and broken markup.
- **Key Functions**:
  - `sanitize_and_extract_metadata(raw_md: &str) -> Result<(String, ArticleMetadata), ContentError>`: Extracts YAML frontmatter, generates reading time, and removes dangerous HTML elements.
  - `extract_table_of_contents(raw_md: &str) -> Vec<TocHeading>`: Builds hierarchical header index (`#`, `##`, `###`) for wiki sidebar navigation.

### `modpack_delta_calculator.rs` (<380 LOC)
- **Purpose**: Computes delta manifests and download bandwidth estimates between different modpack revisions.
- **Key Functions**:
  - `compute_manifest_diff(base: &Modpack, target: &Modpack) -> ModpackDelta`: Identifies added, updated, and removed addons.
  - `calculate_required_download_bytes(delta: &ModpackDelta) -> u64`: Sums raw download byte requirements for client preloading.
