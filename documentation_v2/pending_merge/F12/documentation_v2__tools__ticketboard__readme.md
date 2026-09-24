# Ticketboard Desktop Viewer (`tools/ticketboard/`)

A high-performance native desktop GUI application built with `egui` and `eframe` for browsing tickets, inspecting dependency trees, and monitoring active waves.

## Features
- In-memory parsing of `.ai/tickets/` with sub-millisecond query performance.
- Reactive filesystem watcher (`notify` crate) triggering immediate UI repaints upon file edits on disk.
- Safe subprocess execution of `cargo xtask ticket <verb>`.
- In-app markdown previewer powered by `egui_commonmark`.
- Verbatim `wave.lock` dependency visualizer and subagent token receipt metrics.

## Code Mapping
- Source: `apps/ticketboard/`
