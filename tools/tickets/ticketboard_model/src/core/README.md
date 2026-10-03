# Ticketboard model core

The ticket-free foundations: streamed subprocesses, the bounded output log, cargo discovery,
opening a path externally, and wall-clock labels.

## Contents

```text
tools/tickets/ticketboard_model/src/core/
├── mod.rs     the module tree: `process` and `time`
├── process/   streamed subprocesses, the bounded output log, cargo discovery and opening a path
└── time.rs    `utc_hms` and `utc_hms_now`, an explicit UTC label for a second or for now
```

## Boundaries

- Depends on: `std`; `time_source::SystemClock` for the current second.
- Used by: every feature, `crate::application_state` and the desktop application.
- Rules: imports no feature module and nothing from `ticket_model`.
