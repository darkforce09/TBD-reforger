# Deployment & Server Operations Runbook

This runbook defines the architecture and operational procedures for deploying the web platform suite and dedicated staging game server.

---

## 1. Deployment Topology

The platform deploys to a dedicated Linux staging server (`dooley` on LAN, e.g. `192.168.0.124` / `192.168.0.140`).

### Service Architecture & Ports
- **Reverse Proxy**: Caddy on port `:3080` (`scripts/deploy/Caddyfile.website`).
  - Serves static Leptos frontend assets from `/home/sam/tbd/dist/`.
  - Enforces `Cross-Origin-Opener-Policy: same-origin` (COOP) and `Cross-Origin-Embedder-Policy: credentialless` (COEP) to permit `SharedArrayBuffer` in WebAssembly threads.
  - Reverse-proxies `/api/*`, `/uploads/*`, `/map-assets/*`, and `/healthz` to `:8081`.
  - Provides client-side single-page application fallback (`try_files {path} /index.html`).
- **Web API Service**: Axum REST API binary running on `:8081`, supervised by systemd user unit `tbd-website-api.service`.
- **Database**: Staging PostgreSQL container running on port `:5433` (`apps/website/docker-compose.staging.yml`), isolated from default port `:5432`.
- **Dedicated Game Server**: Linux Arma Reforger dedicated server running via SteamCMD, configured with custom modpacks and direct-join capabilities.

---

## 2. Website Deployment (`cargo xtask deploy website`)

The deployment command synchronizes the repository to the target server, compiles release binaries, and restarts services:

```bash
cargo xtask deploy website
```

### Execution Pipeline (`tools_v2/xtask/src/commands/deploy/website.rs`):
1. **Target Validation**: Asserts `TBD_REMOTE_DIR` is under `/home/sam/tbd/` and never a PrairieLearn path.
2. **Asset Preflight**: Probes the server for `assets_v2/terrains/terrain-registry.json`; refuses to continue while the host still keeps its assets at the pre-relocation `packages/map-assets`, printing the `mv` to run.
3. **Workspace Synchronization**: `rsync --delete` with the exclude set in `deploy/website/rsync_argv.rs` — secrets, build output, the terrain and scratch asset trees, the legacy `packages/`, the oracle lanes. Every entry is also what protects that path from `--delete` on the server; `cargo xtask deploy website --dry-run` prints the list.
4. **Container Healthcheck**: Asserts staging Postgres container on port `:5433` is running and healthy.
5. **Remote Compilation**:
   - Compiles Axum API: `cargo build --release -p website-api`
   - Compiles Leptos SPA: `trunk build --release` inside `apps/website/frontend/`
5. **Atomic Service Restart**:
   - Reloads and restarts `tbd-website-api.service` via `systemctl --user restart`.
   - Probes `GET http://localhost:8081/healthz` until an HTTP 200 response confirms a clean boot.

---

## 3. Dedicated Staging Server Deployment (`cargo xtask deploy staging`)

To deploy and test mod changes on the live Arma Reforger dedicated server:

```bash
cargo xtask deploy staging
```

### Execution Steps:
1. Validates mod compilation status via `cargo xtask mod compile`.
2. Packages the `tbd-framework` and `tbd-export` addons.
3. Rsyncs addons to the server's `-addonsDir` path.
4. Renders `server.config.json` with the active mission header and mod list.
5. Restarts the dedicated server systemd unit.
6. Tails `console.log` and asserts zero fatal script errors during world initialization.
