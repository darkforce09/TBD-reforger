# Utilities (`src/v2/core/utils`)

## Responsibilities
- **`datefmt.rs`**: Relative recency formatting ("5 minutes ago"), mission timestamps, and ISO date parsing.
- **`clipboard.rs`**: The one clipboard write: awaits the browser's `writeText` promise and toasts
  success only once it resolved, or the browser's reason when it rejected. The editor's exporters,
  Server Intel's copy button and Server Control's credential sheet all copy through it.
- **`countdown.rs`**: Live T-minus countdown timers for upcoming operations.
- **`sanitize.rs`**: URL escaping and security sanitizers.
- **`utc_timestamp.rs`**: RFC 3339 UTC instants parsed, compared by value and written back without
  the browser clock — the UTC line shown beside a local time, and the value a date-and-time field
  holds when it is read as UTC.
