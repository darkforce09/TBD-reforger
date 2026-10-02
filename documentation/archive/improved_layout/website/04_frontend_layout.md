**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Frontend architecture & cleanliness

**Status:** Proposed design  
**Scope:** `apps/website/frontend/` source and module structure  
**Context:** TBD Reforger platform monorepo  

Detailed specification for cleaning up `apps/website/frontend/`, establishing the dedicated
`core/transport/` and `src/v2/shell/` boundaries, decoupling inverted dependencies, and maintaining
clear workspace modularity.

---

## 1. Current State & Cleanliness Issues

```text
apps/website/frontend/src/ (CURRENT)
├── app_routes.rs               # Binds URL paths to Leptos components
├── main.rs                     # WASM start_app entrypoint & native main
├── router.rs                   # Route definitions, access tiers, & layout flags
├── tests/                      # Route authorization tests
└── v2/                         # Domain-driven tree
    ├── README.md
    ├── apps/                   # Full-screen CAD workspaces (aar, debug, editor, planner)
    ├── core/                   # Shared foundations
    │   ├── README.md
    │   ├── api/                # <- MISNOMER: The HTTP/SSE client layer & duplicate DTOs
    │   ├── auth/               # <- LEAK: hardcodes editor::shell::purge_local_documents
    │   ├── test_support/       # Testing utilities & fixtures
    │   ├── ui/                 # <- LEAK: search_box/select/slider import tokens from apps/editor
    │   └── utils/              # Pure helper utilities
    ├── map_engine/             # <- ORPHANED: 4 empty directories (camera, renderer, terrain, tools)
    ├── mod.rs                  # Declares pub mod apps; pub mod core; pub mod pages;
    ├── pages/                  # Routed pages grouped by navigation domain
    │   └── navigation/         # <- MISPLACED CHROME: AppLayout, Sidebar, TopNav inside pages/
    └── tests/                  # Production file documentation audit
```

### Problems Identified
1. **Confusing `core/api` module name**: Calling a frontend module `api` inside a monorepo that
   houses `api_v2` causes constant confusion. The frontend does not host an API—it consumes one
   via network transport.
2. **Duplicate wire DTO types**: Because the frontend could not import the native Axum crate `website-api`,
   it manually duplicated wire models in `core/api/dto/` and maintained 15+ parity test files.
3. **Inverted dependency leaks (`core` ➔ `apps`)**:
   - `core/ui/search_box.rs`, `select.rs`, and `slider.rs` import style tokens (`DISABLED_GLYPH`, `HOVER_FILL`)
     from `crate::v2::apps::editor::shell::layout`.
   - `core/auth/store.rs` hardcodes a direct call to `crate::v2::apps::editor::shell::hydrate::purge_local_documents`.
   - The foundation layer is reaching up into an application workspace.
4. **Misplaced application shell in `pages/navigation/`**: `AppLayout`, `TopNav`, `Sidebar`, and
   frame classification are platform chrome components mounted outside the router, yet they are
   nested under `pages/`.
5. **Orphaned `map_engine` directory**: An untracked, empty folder `src/v2/map_engine/` remains from
   before the map and graphics engines were promoted to top-level website crates.

---

## 2. Target Directory Layout

```text
apps/website/frontend/src/ (PROPOSED)
├── app_routes.rs               # Binds URL paths to route components
├── main.rs                     # WASM entrypoint; mounts Router and AppLayout
├── router.rs                   # Route definitions, access tiers, & layout flags
├── tests/
└── v2/
    ├── README.md
    ├── apps/                   # CAD workspaces: editor, debug (placeholders: aar, planner)
    ├── core/                   # Shared foundations (zero business logic)
    │   ├── README.md
    │   ├── auth/               # Session store, identity, logout hooks, route guards
    │   ├── test_support/       # Test utilities
    │   ├── transport/          # [RENAMED] Browser HTTP verbs, refresh, SSE, actions
    │   ├── ui/                 # UI primitives (Aegis design system) & tokens.rs
    │   └── utils/              # Pure utility helpers
    ├── shell/                  # [NEW] Application frame: AppLayout, TopNav, Sidebar
    ├── pages/                  # Strictly routed content pages (command_center, operations, etc.)
    └── tests/
```

---

## 3. Structural Components & Invariants

### 3.1. The Four-Tier Frontend Hierarchy
Restoring clean, strict one-way architectural flow:
$$\text{apps (CAD workspaces)} \longrightarrow \text{pages (routed docs)} \longrightarrow \text{shell (chrome)} \longrightarrow \text{core (foundations)}$$

1. **`core/`**: Foundational, zero-business-logic primitives (transport, session, UI tokens, math).
   Never imports from `shell`, `pages`, or `apps`.
2. **`shell/`**: Application frame (`AppLayout`), global navigation (`TopNav`, `Sidebar`), and frame
   classification (`Bare`, `Chromeless`, `Chrome`). Mounts outside the router; wraps `pages`.
3. **`pages/`**: Routed document pages that render inside the `<main>` container of `shell/`.
4. **`apps/`**: Full-screen standalone CAD workspaces that bypass shell chrome (`Chromeless`).

### 3.2. Remediation of Inverted Dependencies
1. **Design Tokens in `core/ui/tokens.rs`**:
   - `DISABLED_GLYPH`, `HOVER_FILL`, and related interaction tokens are moved into `core/ui/tokens.rs`.
   - `core/ui/search_box.rs`, `select.rs`, and `slider.rs` import tokens locally.
   - `apps::editor` imports tokens from `crate::v2::core::ui::tokens`.
2. **Event-Driven Logout Hook in `core/auth/`**:
   - In `core/auth/store.rs`, the hardcoded call to editor purge is replaced with an event hook:
     ```rust
     pub fn register_logout_hook(callback: Box<dyn Fn(&str) + Send + Sync + 'static>);
     ```
   - On application startup, `apps::editor` registers its own IndexedDB document purge callback.
     `core/auth` remains completely decoupled from `apps::editor`.

### 3.3. Dedicated Application Shell (`src/v2/shell/`)
- Extract `layout.rs`, `sidebar.rs`, `top_nav.rs`, and `nav_config.rs` from `pages/navigation/` into
  `<apps/website/frontend/src/v2/shell/>`.
- `pages/` becomes strictly routed document pages: `account`, `administration`, `command_center`,
  `doctrine_and_info`, `field_tools`, `mission_hub`, and `operations`.

### 3.4. Introduction of `core/transport/`
* `core/api/` is renamed to `core/transport/`.
* Complete elimination of `core/api/dto/` and `dto/tests/`. Wire models are imported directly from
  the new shared `website-api-types` crate.
* For the complete deep dive on `core/transport/`, see [Transport layout](05_transport_layout.md).

### 3.5. Placeholder Workspaces (`apps::aar` and `apps::planner`)
* `apps/website/frontend/src/v2/apps/aar/` and `apps/website/frontend/src/v2/apps/planner/` are stub directories reserved for upcoming CAD workspaces
  (After-Action Review player and Tactical Planning whiteboard).
* They contain only descriptive README files and are intentionally omitted from `apps/website/frontend/src/v2/apps/mod.rs` until
  active implementation begins.

### 3.6. Removal of Orphaned `map_engine`
- **Location**: `apps/website/frontend/src/v2/map_engine/{camera, renderer, terrain, tools}`
- **Action**: Delete the entire directory.

---

## 4. Trunk, Tailwind, and Build Outputs

- **Configuration (`Trunk.toml`)**:
  - `dist = "dist"`: Trunk outputs build artifacts to `apps/website/frontend/dist`.
  - `ignore = ["dist", "style/aegis.css"]`: Prevents recursive rebuild loops when Tailwind or Trunk
    writes assets.
  - `tailwindcss = "4.3.2"`: Pins the Tailwind v4 CLI version for build reproducibility.
  - Proxy rules forward `/api` and `/map-assets` to local Axum backend on port 8080.
- **Gitignore Safety**:
  - Root `.gitignore` explicitly ignores `/apps/website/frontend/dist/` and
    `/apps/website/frontend/dist-debug/`.
  - Build outputs remain strictly outside version control.

---

## 5. Verification Commands

Following future migration of frontend modules:
```bash
# 1. Check formatting
cargo fmt -p website-frontend --check

# 2. Compile for WASM
cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets

# 3. Run native unit tests
cargo test -p website-frontend

# 4. Verify Trunk release build
cd apps/website/frontend && trunk build --release
```
