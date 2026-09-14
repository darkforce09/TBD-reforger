# Auth & Session Core (`src/v2/core/auth`)

## Responsibilities
- Discord OAuth2 redirect callback handler.
- LocalStorage token persistence with memory fallback.
- User role evaluation (`Admin`, `MissionMaker`, `Enlisted`, `Guest`).
- Route authorization guards (`url_guard.rs`).
