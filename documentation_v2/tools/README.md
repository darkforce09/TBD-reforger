# Repository Tooling Suite (`tools_v2/`)

Documentation for the repository tooling, build orchestration, and verification infrastructure, mirroring `tools_v2/` (the former `tools/tbd-tools` suite lives in `tools_v2/developer-tools/`).

## Tooling Subsystems
- `tools_v2/xtask/`: Declarative task runner (`cargo xtask`), CI verification gates, database management.
- `developer_tools/`: Heavy asynchronous CLI suite (`developer-tools`: gate, enf, mcpd, world, map, capture).
- `ticket_engine/`: Ticket domain database, canonical TOML serialization and transactional storage (`ticket-engine`).
- `verification_core/`: Fail-closed 4-outcome static assertion library (`verification-core`: Verdict, GateLock).
- `ticketboard/`: Native desktop egui/eframe ticket and wave viewer application (`apps/ticketboard/`).

## Code Mapping
- `tools_v2/xtask/`
- `tools_v2/developer-tools/`
- `tools_v2/ticket-engine/`
- `tools_v2/verification-core/`
- `apps/ticketboard/`
