# Auth & Session Core (`src/v2/core/auth`)

## Responsibilities
- Discord OAuth2 redirect callback handler.
- LocalStorage token persistence with memory fallback.
- User role evaluation (`Admin`, `MissionMaker`, `Enlisted`, `Guest`).
- Route authorization guards (`url_guard.rs`).
- Cold-start session restore (`session_restore.rs`): every request waits until the stored
  session is restored or found absent, so it is sent under the restored session generation.
