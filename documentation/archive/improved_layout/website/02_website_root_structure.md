**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Website root structure & peer crates

**Status:** Proposed design  
**Scope:** `apps/website/` root directory and peer crate topology  
**Context:** TBD Reforger platform monorepo  

Detailed specification of the top-level directory structure for `apps/website/`, explaining the
roles of peer crates, the elimination of root configuration clutter, and the absorption of ad-hoc
single-file directories.

---

## 1. Current State & Clutter Analysis

The current `apps/website/` directory contains a mix of Cargo workspace crates, container
configurations, and platform files:

```text
apps/website/ (CURRENT)
├── Dockerfile                      # Production API Dockerfile
├── README.md                       # Subsystem documentation
├── api_v2/                         # Crate `website-api`
├── docker-compose.staging.yml      # Staging compose configuration
├── frontend/                       # Crate `website-frontend`
├── graphics-engine/                # Crate `website-graphics-engine`
├── map-engine/                     # Crate `website-map-engine`
└── shared/                         # Ad-hoc single-file test data table
```

### Problems Identified
1. **Root-level infrastructure clutter**: `Dockerfile` and `docker-compose.staging.yml` sit directly
   in the website root. Meanwhile, local development Postgres compose is hidden inside
   `apps/website/api_v2/docker-compose.yml`. There is no single place to look for deployment and
   container files.
2. **Asymmetric responsibilities**: The root directory functions simultaneously as a multi-crate
   parent folder and as an operational deployment working directory.
3. **Missing shared contract crate**: Wire DTO models are hand-duplicated inside `frontend`, because
   the frontend cannot import the heavy native server crate `website-api`.
4. **Ad-hoc single-file directory**: `apps/website/shared/` exists solely to hold a single 86-item
   URL test table (`is_http_url_cases.rs`), cluttering the root.

---

## 2. Proposed Target Layout

```text
apps/website/ (PROPOSED)
├── README.md                       # Subsystem overview & directory index
├── api_v2/                         # Crate `website-api`: Axum server & REST/SSE endpoints
│   └── types/                      # [NEW CRATE] `website-api-types`: Shared wire DTOs (WASM + Linux)
├── deploy/                         # [NEW] Unified deployment, Docker, and compose files
├── frontend/                       # Crate `website-frontend`: Leptos / WASM single-page app
├── graphics-engine/                # Crate `website-graphics-engine`: Pure WGPU rendering primitives
├── improved_layout/                # [NEW] Planning anchor README linking to documentation_v2
└── map-engine/                     # Crate `website-map-engine`: World, streaming, & mission domain
```

---

## 3. Detailed Component Breakdown & Rationale

### 3.1. `deploy/` (New Peer Infrastructure Directory)
* **What is it**: A dedicated directory housing all container definitions, compose stacks, and web
  server proxy configurations for the website platform.
* **Why it is needed**:
  - Unifies development and staging container environments into one predictable location.
  - Removes container orchestration files from crate source roots (`api_v2/docker-compose.yml` and
    `apps/website/docker-compose.staging.yml`).
  - Provides a single point of truth for Docker builds (`deploy/Dockerfile.api`).
* **Detailed specification**: See [Deploy layout](03_deploy_layout.md).

### 3.2. `api_v2/` (`website-api`) & `api_v2/types/` (`website-api-types`)
* **Role**: The authoritative home for all backend services, database migrations, and shared wire types.
* **The `types/` sub-crate**:
  - Lightweight, pure serde data structs (compiles to both `x86_64-unknown-linux-gnu` and `wasm32-unknown-unknown`).
  - Receives contract types generated from `contracts_v2/definitions/*.schema.json` via `cargo xtask schema codegen`.
  - Imported by both `website-api` (for serialization) and `website-frontend` (for deserialization).
  - Absorbs the shared URL validation test fixture table (`is_http_url_cases.rs`).
* **The Single-Source-of-Truth Rule**: If you are working on the API or changing wire data models, you go to `api_v2/`.
* **Detailed specification**: See [API v2 layout](06_api_v2_layout.md).

### 3.3. `frontend/` (`website-frontend`)
* **Role**: The browser-facing single-page application built with Leptos 0.8 and compiled to
  WebAssembly via Trunk.
* **Boundary**: Consumes `website-api-types` for wire models and `website-map-engine` for client-side
  mission editing. It handles network communication through its dedicated `core/transport/` layer
  and application chrome through `src/v2/shell/`. It **never** imports `website-graphics-engine`
  directly (`cargo xtask verify engine-layers` rule 6).
* **Detailed specifications**: See [Frontend layout](04_frontend_layout.md) and [Transport layout](05_transport_layout.md).

### 3.4. Why `map-engine` and `graphics-engine` are Website Root Peers
A common point of confusion is whether the engines belong inside `frontend/`:
* **Historical context**: Earlier iterations had exploratory engine code under
  `frontend/src/v2/map_engine/`. That code was superseded and folded into standalone crates.
* **Why they are top-level peers**:
  1. **Multi-consumer architecture**: `map-engine` is NOT consumed only by `frontend`. It is also
     linked directly by `api_v2` (for server-side mission compilation and headless validation) and by
     `developer-tools` (for offline heightmap generation, satellite tile building, and headless
     browser verification).
  2. **Headless domain separation**: The core of `map-engine` (spatial geometry, terrain coordinates,
     ORBAT roster structures) contains no browser DOM (`web-sys`, `leptos`) dependencies. Placing
     it inside `frontend/` would create an inverted dependency where backend tooling and Axum
     servers would have to import from a frontend UI crate.
  3. **Engine layering wall**: `graphics-engine` is a pure GPU rendering library that knows no map
     concepts. It sits below `map-engine`, and only `map-engine` imports it. Keeping both engines as
     independent workspace members at the `apps/website/` root makes these layer boundaries clear,
     verifiable, and clean.

### 3.5. Absorption of the Legacy `shared/` Directory
* **Previous state**: `apps/website/shared/` held `is_http_url_cases.rs`, an 86-item test table for
  the URL scheme guard, included via `include!` in `api_v2` and `frontend`.
* **Target state**: Relocated into `<apps/website/api_v2/types/src/is_http_url_cases.rs>` (or under
  test support in `types/`).
* **Benefit**: Completely eliminates an ad-hoc, single-file folder from the website root.

### 3.6. `improved_layout/` (Planning Anchor)
* **Role**: Contains `README.md` which summarizes the architectural design and provides markdown
  links to the full specifications in `documentation_v2/website/improved_layout/`.
* **Rationale**: Satisfies the monorepo's `markdown-placement` gate while keeping the reorganization
  initiative front-and-center for developers working in the code tree.

---

## 4. Subsystem Workspace Membership

The Cargo workspace configuration in the repository root (`Cargo.toml`) reflects this peer structure:

```toml
[workspace]
resolver = "3"
members = [
    # Website subsystem crates:
    "apps/website/api_v2",
    "apps/website/api_v2/types",
    "apps/website/frontend",
    "apps/website/map-engine",
    "apps/website/graphics-engine",
    ...
]
```

Neither `deploy/` nor `improved_layout/` are Cargo crates; they do not appear in `members` and
require no Cargo manifest.

---

## 5. Summary of Benefits

1. **Clean Root**: `apps/website/` contains only crates and two clearly designated peer infrastructure
   directories (`deploy/` and `improved_layout/`). No ad-hoc loose files or single-file test directories.
2. **Predictable Navigation**: Developers know immediately where to find server and API code (`api_v2`),
   UI code (`frontend`), domain/rendering engines (`map-engine`, `graphics-engine`), and container
   orchestration (`deploy`).
3. **Preserved Invariants**: Zero impact on Cargo workspace resolution, xtask build lanes, or engine
   boundary rules.
