# TBD Reforger Platform — monorepo dev tasks (delegates to apps/website/).
COMPOSE := $(shell command -v docker >/dev/null 2>&1 && echo "docker compose" || echo "podman compose")
WEB := apps/website/api
# ~/.local/go/bin stays on PATH only for the Go toolchain that `go install`s editorconfig-checker.
# ~/go/bin is the default GOPATH/bin where `go install` drops tools (editorconfig-checker, T-125.5);
# ~/go/bin is prepended for the editorconfig-checker binary (`make verify-editorconfig`).
export PATH := $(HOME)/.cargo/bin:$(HOME)/.local/go/bin:$(HOME)/go/bin:$(PATH)

.PHONY: help db-up db-down db-logs seed registry-import api leptos leptos-debug leptos-build leptos-gates test build tickets ticket-list ticket-sync ticket-check ticket-check-strict schema-validate schema-codegen verify-citations verify-coding-standards verify-doc-layout verify-editorconfig verify-terrain verify-no-python verify-no-node map-water-everon map-cartographic-everon map-cartographic-verify mcp-selftest mcp-smoke ci-local ci-local-leptos ci-local-schema rust-api rust-build rust-test rust-test-it rust-fmt rust-clippy rust-ci rust-sqlx-prepare wasm-ci lfs-dem lfs-sat

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}'

db-up: ## Start local Postgres in the background
	cd $(WEB) && $(COMPOSE) up -d db

db-down: ## Stop local Postgres (keeps the data volume)
	cd $(WEB) && $(COMPOSE) down

db-logs: ## Tail the Postgres logs
	cd $(WEB) && $(COMPOSE) logs -f db

seed: ## Apply data seeds (Discord role mappings + registry catalog) to the running DB
	cd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/discord_roles.sql
	cd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/registry_dev.sql

registry-import: ## Ingest the committed T-150 registry envelopes (items + compat) into the dev DB (T-068.9)
	cd $(WEB) && cargo run --bin import-registry -- \
		--items ../../../packages/tbd-schema/registry/registry-items.workbench.json \
		--compat ../../../packages/tbd-schema/registry/registry-compat.workbench.json

api: ## Run the API (loads apps/website/api/.env; migrates on boot)
	cd $(WEB) && cargo run --bin api

leptos: ## Run the Leptos dev server on :3000 in RELEASE profile (T-173 P8 — the operator day-to-day path; /api proxies to :8080)
	cd apps/website/frontend && trunk serve --release

leptos-debug: ## Debug-profile dev server on :3000 — fast rebuilds, unoptimized wasm (editor perf is NOT representative; T-173 P8)
	cd apps/website/frontend && trunk serve

leptos-build: ## Release-build the Leptos SPA into apps/website/frontend/dist
	cd apps/website/frontend && trunk build --release

gate-doctor: leptos-build ## T-177 fail-fast editor-gate preflight: chromium/toolchain pins + RAM/orphans + a ~15s editor liveness probe (a wedge fails here with a diagnosis, not a 130s hang)
	cargo run -q -p tbd-tools --bin gate -- doctor

leptos-gates: leptos-build gate-doctor ## T-159 editor smokes + the frozen V-suite against a fresh release dist (doctor runs first)
	cargo run -q -p tbd-tools --bin gate -- editor-suite
	cargo run -q -p tbd-tools --bin gate -- v-suite verify

map-water-everon: ## One-button Everon water composite: restore → mask → composite → bundle + pyramid → verify (T-090.1.2.5.2)
	cp packages/map-assets/everon/staging/sap/everon-sap-ortho.pre-water.png packages/map-assets/everon/staging/sap/everon-sap-ortho.png
	cargo run -q -p tbd-tools --bin map -- reset-water-meta --terrain everon
	cargo run -q -p tbd-tools --bin map -- analyze-water
	cargo run -q -p tbd-tools --bin map -- composite-water
	cargo run -q -p tbd-tools --bin map -- build-unified --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png --out packages/map-assets/everon/satellite/everon-sat.tbd-sat --terrain everon
	cargo run -q -p tbd-tools --bin map -- patch-unified-bytes --terrain everon
	cargo run -q -p tbd-tools --bin map -- build-pyramid --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png --out packages/map-assets/everon/tiles/satellite --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
	cargo run -q -p tbd-tools --bin map -- verify-sap-ortho --terrain everon
	cargo run -q -p tbd-tools --bin map -- verify-unified --terrain everon
	cargo run -q -p tbd-tools --bin map -- verify-pyramid --terrain everon --expect-lossless

map-cartographic-everon: ## One-button Everon Map view (stylized cartographic): staging ortho → pyramid → manifest patch → verify (T-090.1.1)
	cargo run -q -p tbd-tools --bin map -- build-cartographic --terrain everon
	cargo run -q -p tbd-tools --bin map -- build-pyramid --input packages/map-assets/everon/staging/map/everon-map-ortho.png --out packages/map-assets/everon/tiles/map --minzoom 0 --maxzoom 6 --tilesize 256
	cargo run -q -p tbd-tools --bin map -- patch-map-tiles-meta --terrain everon
	$(MAKE) map-cartographic-verify

map-cartographic-verify: ## Verify the Everon Map pyramid (complete z0–6 + manifest agreement, T-090.1.1)
	cargo run -q -p tbd-tools --bin map -- verify-pyramid --terrain everon --view-map

test: ## Run backend unit tests
	$(MAKE) rust-test

test-it: ## Run backend integration tests against the local DB (needs `make db-up`)
	$(MAKE) rust-test-it

wasm-ci: ## Fmt + clippy + test the map-engine core/wasm/render crates (T-145/T-151)
	cargo fmt --check -p map-engine-core -p map-engine-wasm -p map-engine-render
	cargo clippy -p map-engine-core -p map-engine-wasm --all-targets --all-features -- -D warnings
	cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
	cargo test -p map-engine-core --all-features
	cargo test -p map-engine-render

build: ## Build the backend + the Leptos SPA
	cd $(WEB) && cargo build --release --bin api
	$(MAKE) leptos-build

# --- Rust port (T-145). Additive during the Go→Rust transition; these become the
# canonical `api`/`build`/`test` targets at cutover (plan Phase 11). Cargo is at
# ~/.cargo/bin (user-space rustup; never dnf on Bazzite).
rust-api: ## Run the Rust API (loads apps/website/api/.env; migrates on boot)
	cd $(WEB) && cargo run --bin api
rust-build: ## Build the Rust backend (all targets)
	cd $(WEB) && cargo build --all-targets
rust-test: ## Run Rust unit tests (no DB)
	cd $(WEB) && cargo test --lib --bins
rust-test-it: ## Run Rust integration tests against a fresh dedicated DB (needs `make db-up` @ :5434)
	@# A dedicated, Rust-migrated DB (the dev `tbd_reforger` predates sqlx tracking).
	-podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc "DROP DATABASE IF EXISTS rust_it WITH (FORCE);"
	podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc "CREATE DATABASE rust_it;"
	cd $(WEB) && TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable cargo test
rust-fmt: ## Check Rust formatting (FMT-1 analog)
	cd $(WEB) && cargo fmt --check
rust-clippy: ## Lint Rust with clippy (deny warnings; GO-2..8 analog)
	cd $(WEB) && cargo clippy --all-targets -- -D warnings
rust-sqlx-prepare: ## Refresh the committed sqlx offline query cache (.sqlx/)
	cd $(WEB) && cargo sqlx prepare
rust-ci: ## Rust CI gate locally — fmt + clippy + build + test-it (mirrors the ci.yml rust-backend job)
	$(MAKE) rust-fmt
	$(MAKE) rust-clippy
	$(MAKE) rust-build
	$(MAKE) wasm-ci
	$(MAKE) rust-test-it

schema-validate: ## Validate golden missions + T-090 map-object contracts (enums + glyphs + spec consistency) + T-152.16 height labels
	cargo run -q -p xtask -- schema validate
	cargo run -q -p xtask -- schema map-object-golden
	cargo run -q -p xtask -- schema map-glyphs
	cargo run -q -p xtask -- schema height-labels
	cargo run -q -p xtask -- schema map-object-enums
	cargo run -q -p xtask -- schema type-inventory
	cargo run -q -p xtask -- schema t090-specs
	cargo run -q -p xtask -- schema n6
	cargo run -q -p xtask -- schema n10

schema-codegen: ## Regenerate Rust contract types from packages/tbd-schema/schema via typify (T-165.3; loadout.rs is hand-maintained)
	cargo run -q -p xtask -- schema codegen

verify-citations: ## Verify @contract citations (DOCUMENTATION_STANDARDS §10; T-165.1 Rust port)
	cargo run -q -p xtask -- schema citations

verify-coding-standards: ## SIZE file length + doc layout (CODING_STANDARDS §11). Rust GO-2..9/ERR-4/LOG-3 analogs are enforced by clippy + the centralized ApiError type + `cargo fmt`.
	$(MAKE) verify-doc-layout
	@cargo run -q -p xtask -- verify file-length
	@bash scripts/website/verify-no-select-star.sh

verify-doc-layout: ## DOCUMENTATION_STANDARDS §8.2: no markdown spec trees under apps/**/docs or packages/**/docs
	@! find apps packages -type f -path '*/docs/*.md' ! -path '*/node_modules/*' 2>/dev/null | grep -q . || \
		(echo "FORBIDDEN: markdown under apps/**/docs/ or packages/**/docs/ — use docs/website/ instead" && exit 1)

# FMT-2 (CODING_STANDARDS §7): root .editorconfig honored across apps/, packages/, docs/, scripts/.
# Excludes live in .editorconfig-checker.json. Install (drops binary in ~/go/bin):
#   go install github.com/editorconfig-checker/editorconfig-checker/v3/cmd/editorconfig-checker@latest
verify-editorconfig: ## FMT-2: run editorconfig-checker from repo root (CODING_STANDARDS §7)
	editorconfig-checker

verify-terrain: ## Manifest + anchor verify (stub mode OK for Arland-only)
	cargo run -q -p xtask -- schema terrain-manifest --terrain everon
	cargo run -q -p xtask -- schema terrain-alignment --terrain everon

verify-terrain-strict: ## Full anchor alignment gate (T-091.0 GetSurfaceY DEM + anchors)
	cargo run -q -p xtask -- schema terrain-manifest --terrain everon
	cargo run -q -p xtask -- schema terrain-alignment --terrain everon --strict

# T-090.0.2 — map-object contract verifiers (run inside schema-validate). Real gates.
.PHONY: map-object-enums-verify map-object-golden-verify map-glyphs-verify t090-spec-verify
map-object-enums-verify: ## T-090.2 enum single-source: prefab-classify + golden prefabs + glyph kinds subset of map-object-enums
	cargo run -q -p xtask -- schema map-object-enums

map-object-golden-verify: ## T-090.2 semantic golden gates S2–S9: prefabId resolve, dedup, closed-enum coverage
	cargo run -q -p xtask -- schema map-object-golden

map-glyphs-verify: ## T-090.5 glyph coverage: every golden prefab render.iconKey has an SVG + manifest entry
	cargo run -q -p xtask -- schema map-glyphs

t090-spec-verify: ## T-090 spec consistency grep gates (DoD): zoom space, picking, audit-closure, command existence
	cargo run -q -p xtask -- schema t090-specs

.PHONY: verify-t090-spec-consistency
verify-t090-spec-consistency: t090-spec-verify ## Alias — spec corpus cites this name (DoD rule 7)

# T-090.3.1 — map export pipeline (data-only, Map Engine v2). map-export-all stays a stub until a
# second terrain has a Workbench export.
.PHONY: map-export map-export-all map-export-validate map-verify-phase map-census map-glyphs-build map-render-verify
map-export: ## T-090.3.1 — classify staged Workbench export for TERRAIN=<id> PHASE=Pn (exit 2 = run the documented Workbench step first)
	@test -n "$(TERRAIN)" || (echo "map-export: TERRAIN=<id> required"; exit 1)
	@test -n "$(PHASE)" || (echo "map-export: PHASE=Pn required (e.g. PHASE=P1_buildings)"; exit 1)
	bash scripts/map-assets/export-terrain.sh "$(TERRAIN)" --phase "$(PHASE)"
map-export-all: ## T-090.3 stub — export every terrain in terrain-registry.json
	@echo "map-export-all: not implemented (T-090.3)"; exit 1
map-export-validate: ## T-090.3.1 — validate committed export artifacts for every registry terrain (CI-safe)
	cargo run -q -p tbd-tools --bin world -- validate-exports
map-verify-phase: ## T-090.3.1 — mathematical phase gate G1-G12 + P1-* + E6 for TERRAIN=<id> PHASE=Pn (needs staging)
	@test -n "$(TERRAIN)" || (echo "map-verify-phase: TERRAIN=<id> required"; exit 1)
	@test -n "$(PHASE)" || (echo "map-verify-phase: PHASE=Pn required"; exit 1)
	cargo run -q -p tbd-tools --bin world -- verify-phase --terrain "$(TERRAIN)" --phase "$(PHASE)"
map-census: ## T-090.2 — validate type-inventory.json; compute counts after export (TERRAIN=<id>)
	@test -n "$(TERRAIN)" || (echo "map-census: TERRAIN=<id> required"; exit 1)
	cargo run -q -p tbd-tools --bin world -- census --terrain "$(TERRAIN)"
map-glyphs-build: ## T-090.5.2 — build world-glyph atlas (webp + Deck mapping) from packages/map-assets/glyphs/svg
	cargo run -q -p tbd-tools --bin map -- build-glyph-atlas
map-render-verify: ## T-090.5 stub — per-phase render smoke (layer instance count + purity)
	@echo "map-render-verify: not implemented (T-090.5)"; exit 1

# T-171 — selective LFS pulls (the only two LFS objects in the repo; a plain clone gets
# pointer files and the API serves 404 for them until pulled). DEM is enough for
# map-engine-core tests + hillshade; the satellite bundle is the full-res editor basemap.
lfs-dem: ## Pull the Everon DEM from LFS (72 MB — map-engine tests + hillshade)
	git lfs pull --include packages/map-assets/everon/dem/everon-dem-16bit.png
lfs-sat: ## Pull the Everon satellite bundle from LFS (153 MB — full-res editor basemap)
	git lfs pull --include packages/map-assets/everon/satellite/everon-sat.tbd-sat

mcp-selftest: ## Offline MCP gates (19/19) — no Workbench
	bash scripts/mod/mcp-call-selftest.sh
mcp-smoke: ## Live MCP smoke — wb_connect + wb_state (Workbench Net API up)
	bash scripts/mod/mcp-smoke.sh

tickets: ## Run Claude Code on ready tickets in parallel
	./scripts/ticket run

ticket-list: ## Show ticket queue status
	./scripts/ticket list

ticket-sync: ## Regenerate all ticket-derived docs from registry.json
	./scripts/ticket sync

ticket-check: ## Structural validation of ticket registry + outputs
	./scripts/ticket check

ticket-check-strict: ## Full validation including zero legacy planning IDs
	./scripts/ticket check --strict

verify-no-python: ## T-162 hard gate — zero .py files / no Python interpreter in scripts
	./scripts/verify-no-python.sh

verify-no-node: ## T-165.10 hard gate — zero tracked .mjs/.cjs; node only as the enfusion-mcp floor
	cargo run -q -p xtask -- verify no-node

# ci-local mirrors .github/workflows/ci.yml (CODING_STANDARDS.md §0.3 CI-2, §11). Order:
# editorconfig (FMT-2) -> rust backend -> coding standards -> Leptos SPA -> schema; each
# sub-target is a separate $(MAKE) so a non-zero recipe halts the run (fail-fast). Node-free
# since T-165 — every gate is cargo/bash (Node exists solely as the enfusion-mcp runtime).
ci-local: ## Full CI gate locally — mirrors ci.yml (run `make db-up` first)
	$(MAKE) verify-editorconfig
	$(MAKE) verify-no-python
	$(MAKE) verify-no-node
	$(MAKE) rust-ci
	$(MAKE) verify-coding-standards
	$(MAKE) ci-local-leptos
	$(MAKE) ci-local-schema

ci-local-leptos: ## CI gate: Leptos SPA fmt + clippy(wasm32) + native tests + trunk release build (mirrors ci.yml website-frontend)
	cargo fmt -p website-frontend --check
	cargo clippy -p website-frontend --target wasm32-unknown-unknown
	cargo test -p website-frontend
	cd apps/website/frontend && trunk build --release

ci-local-schema: ## CI gate: schema validate (TEST-3) + @contract citation verify
	$(MAKE) schema-validate
	$(MAKE) verify-citations

