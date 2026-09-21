# Repository Tooling Suite (`tools_v2/` and `tools/`)

Documentation for the repository tooling, build orchestration, and verification infrastructure, mirroring `tools/` and `tools_v2/`.

## Tooling Subsystems
- `tools_v2/xtask/`: Declarative task runner (`cargo xtask`), CI verification gates, database management.
- `developer_tools/`: Heavy asynchronous CLI suite (`tbd-tools`: gate, enf, mcpd, world, map, capture).
- `ticket_engine/`: Ticket domain database, canonical TOML serialization and transactional storage (`ticket-engine`).
- `verification_core/`: Fail-closed 4-outcome static assertion library (`verification-core`: Verdict, GateLock).
- `ticketboard/`: Native desktop egui/eframe ticket and wave viewer application (`apps/ticketboard/`).

## Code Mapping
- `tools_v2/xtask/`
- `tools/tbd-tools/`
- `tools_v2/ticket-engine/`
- `tools_v2/verification-core/`
- `apps/ticketboard/`
