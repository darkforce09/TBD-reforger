# Administrative Audit Logs (`/admin/audit`)

The trail of administrative actions, newest first, beside the expanded record of the one selected.

## Architecture
- **`page.rs`**: route component — fetches the first page, puts it behind the administrator gate,
  and owns the keyset paging helpers the load control uses.
- **`filter_bar.rs`**: the search field above the trail, and the text one entry is matched against.
- **`log_table.rs`**: the accumulated trail with its load control, the level tokens, and the entry
  inspector.
- **`tests/audit.rs`**: the keyset paths, the cursor read, and the load-more append.

## Not present in the legacy page
- **An action-type selector and a date range**: the filter is one free-text box, matched
  client-side against the stamp, the level, the action, the actor and the message of everything
  already loaded.
- **Expandable payload diffs**: an entry's metadata is free-form, and the inspector prints it as
  formatted JSON rather than as a diff.

## Related documentation

- [Audit logs page](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
