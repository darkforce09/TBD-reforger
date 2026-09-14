# Administrative Audit Logs (`/admin/audit`)

## Architecture
- **`page.rs`**: Main layout file.
- **`filter_bar.rs`**: Filter by actor (who did it), action type (kick, role promote, event delete), and date.
- **`log_table.rs`**: Immutable chronological audit log table with expandable payload diffs.
