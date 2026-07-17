# TBD Reforger Platform — monorepo dev tasks (delegates to apps/website/).
COMPOSE := $(shell command -v docker >/dev/null 2>&1 && echo "docker compose" || echo "podman compose")
WEB := apps/website
# Go is often installed under ~/.local/go/bin and not on PATH (see CLAUDE.md).
# ~/go/bin is the default GOPATH/bin where `go install` drops tools (editorconfig-checker, T-125.5);
# golangci-lint lives in ~/.local/go/bin. Both are prepended so `make ci-local` resolves them.
export PATH := $(HOME)/.cargo/bin:$(HOME)/.local/go/bin:$(HOME)/go/bin:$(PATH)

.PHONY: help db-up db-down db-logs seed registry-import api leptos leptos-build leptos-gates test build tidy tickets ticket-list ticket-sync ticket-check ticket-check-strict schema-validate schema-codegen verify-citations verify-coding-standards verify-doc-layout verify-editorconfig verify-terrain verify-migration map-water-everon map-cartographic-everon map-cartographic-verify mcp-selftest mcp-smoke ci-local ci-local-backend ci-local-leptos ci-local-schema rust-api rust-build rust-test rust-test-it rust-fmt rust-clippy rust-ci rust-sqlx-prepare wasm-ci

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
	cd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < internal/db/seeds/discord_roles.sql
	cd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < internal/db/seeds/registry_dev.sql

registry-import: ## Ingest the committed T-150 registry envelopes (items + compat) into the dev DB (T-068.9)
	cd $(WEB) && cargo run --bin import-registry -- \
		--items ../../packages/tbd-schema/registry/registry-items.workbench.json \
		--compat ../../packages/tbd-schema/registry/registry-compat.workbench.json

api: ## Run the API (loads apps/website/.env; migrates on boot)
	cd $(WEB) && cargo run --bin api

leptos: ## Run the Leptos dev server on :3000 (trunk serve; /api proxies to :8080 — T-159.24)
	cd apps/website-leptos && trunk serve

leptos-build: ## Release-build the Leptos SPA into apps/website-leptos/dist
	cd apps/website-leptos && trunk build --release

leptos-gates: leptos-build ## T-159 editor smokes + the frozen V-suite against a fresh release dist
	@set -e; for s in .ai/artifacts/t159_gates/driver/*_editor.mjs; do \
		echo "== $$s"; node $$s; done
	node .ai/artifacts/t159_gates/driver/gate_v_suite.mjs verify

map-water-everon: ## One-button Everon water composite: restore → mask → composite → bundle + pyramid → verify (T-090.1.2.5.2)
	cp packages/map-assets/everon/staging/sap/everon-sap-ortho.pre-water.png packages/map-assets/everon/staging/sap/everon-sap-ortho.png
	node -e "const f='packages/map-assets/everon/staging/sap/TBD_SatExport_meta.json',fs=require('fs'),m=JSON.parse(fs.readFileSync(f,'utf8'));delete m.waterComposite;fs.writeFileSync(f,JSON.stringify(m,null,2)+'\n')"
	node scripts/map-assets/analyze-water-sources.mjs
	node --max-old-space-size=8192 scripts/map-assets/composite-water-ortho.mjs
	node scripts/map-assets/build-unified-satellite.mjs --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png --out packages/map-assets/everon/satellite/everon-sat.tbd-sat --terrain everon
	node -e "const fs=require('fs'),mp='packages/map-assets/everon/manifest.json',m=JSON.parse(fs.readFileSync(mp,'utf8'));m.tiles.satellite.unified.bytes=fs.statSync('packages/map-assets/everon/satellite/everon-sat.tbd-sat').size;fs.writeFileSync(mp,JSON.stringify(m,null,2)+'\n')"
	bash scripts/map-assets/build-tile-pyramid.sh --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png --out packages/map-assets/everon/tiles/satellite --minzoom 0 --maxzoom 6 --tilesize 256 --lossless
	node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
	node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
	EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon

map-cartographic-everon: ## One-button Everon Map view (stylized cartographic): staging ortho → pyramid → manifest patch → verify (T-090.1.1)
	TERRAIN=everon node scripts/map-assets/build-map-cartographic.mjs
	bash scripts/map-assets/build-tile-pyramid.sh --input packages/map-assets/everon/staging/map/everon-map-ortho.png --out packages/map-assets/everon/tiles/map --minzoom 0 --maxzoom 6 --tilesize 256
	node -e "const fs=require('fs'),mp='packages/map-assets/everon/manifest.json',m=JSON.parse(fs.readFileSync(mp,'utf8'));Object.assign(m.tiles.map,{source:'workbench-cartographic',encoding:'webp-lossy'});fs.writeFileSync(mp,JSON.stringify(m,null,2)+'\n')"
	$(MAKE) map-cartographic-verify

map-cartographic-verify: ## Verify the Everon Map pyramid (complete z0–6 + manifest agreement, T-090.1.1)
	VIEW=map node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon

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

tidy: ## Tidy Go modules
	cd $(WEB) && go mod tidy

# --- Rust port (T-145). Additive during the Go→Rust transition; these become the
# canonical `api`/`build`/`test` targets at cutover (plan Phase 11). Cargo is at
# ~/.cargo/bin (user-space rustup; never dnf on Bazzite).
rust-api: ## Run the Rust API (loads apps/website/.env; migrates on boot)
	cd $(WEB) && cargo run --bin api
rust-build: ## Build the Rust backend (all targets)
	cd $(WEB) && cargo build --all-targets
rust-test: ## Run Rust unit tests (no DB)
	cd $(WEB) && cargo test --lib --bins
rust-test-it: ## Run Rust integration tests against a fresh dedicated DB (needs `make db-up` @ :5434)
	@# A dedicated, Rust-migrated DB (the dev `tbd_reforger` is Go-owned — no sqlx tracking).
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
	cd packages/tbd-schema && npm ci --silent && node scripts/validate.mjs && npm run verify-map-object-enums && npm run verify-map-object-golden && npm run verify-map-glyphs && npm run verify-type-inventory && npm run verify-t090-specs && npm run verify-n6 && npm run verify-n10 && node ../../scripts/map-assets/verify-height-labels.mjs

schema-codegen: ## Regenerate TS + Rust contract types from packages/tbd-schema/schema (DOCUMENTATION_STANDARDS §9.1)
	cd packages/tbd-schema && npm ci --silent && node scripts/codegen.mjs

verify-citations: ## Verify @contract citations + GO-7 @route route-match (DOCUMENTATION_STANDARDS §10, CODING_STANDARDS §2)
	node packages/tbd-schema/scripts/verify-contract-citations.mjs

verify-coding-standards: ## SIZE file length + doc layout (CODING_STANDARDS §11). Rust GO-2..9/ERR-4/LOG-3 analogs are enforced by clippy + the centralized ApiError type + `cargo fmt`.
	$(MAKE) verify-doc-layout
	@node scripts/website/verify-file-length.mjs
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
	cd packages/tbd-schema && npm ci --silent && npm run verify-terrain

verify-terrain-strict: ## Full anchor alignment gate (T-091.0 GetSurfaceY DEM + anchors)
	cd packages/tbd-schema && npm ci --silent && node scripts/verify-terrain-manifest.mjs && node scripts/verify-terrain-alignment.mjs --strict

# T-090.0.2 — map-object contract verifiers (run inside schema-validate). Real gates.
.PHONY: map-object-enums-verify map-object-golden-verify map-glyphs-verify t090-spec-verify
map-object-enums-verify: ## T-090.2 enum single-source: prefab-classify + golden prefabs + glyph kinds subset of map-object-enums
	cd packages/tbd-schema && npm ci --silent && npm run verify-map-object-enums

map-object-golden-verify: ## T-090.2 semantic golden gates S2–S9: prefabId resolve, dedup, closed-enum coverage
	cd packages/tbd-schema && npm ci --silent && npm run verify-map-object-golden

map-glyphs-verify: ## T-090.5 glyph coverage: every golden prefab render.iconKey has an SVG + manifest entry
	cd packages/tbd-schema && npm ci --silent && npm run verify-map-glyphs

t090-spec-verify: ## T-090 spec consistency grep gates (DoD): zoom space, picking, audit-closure, command existence
	cd packages/tbd-schema && npm ci --silent && npm run verify-t090-specs

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
	node scripts/map-assets/validate-export-artifacts.mjs
map-verify-phase: ## T-090.3.1 — mathematical phase gate G1-G12 + P1-* + E6 for TERRAIN=<id> PHASE=Pn (needs staging)
	@test -n "$(TERRAIN)" || (echo "map-verify-phase: TERRAIN=<id> required"; exit 1)
	@test -n "$(PHASE)" || (echo "map-verify-phase: PHASE=Pn required"; exit 1)
	node scripts/map-assets/verify-phase.mjs --terrain "$(TERRAIN)" --phase "$(PHASE)"
map-census: ## T-090.2 — validate type-inventory.json; compute counts after export (TERRAIN=<id>)
	@test -n "$(TERRAIN)" || (echo "map-census: TERRAIN=<id> required"; exit 1)
	node scripts/map-assets/census-types.mjs
map-glyphs-build: ## T-090.5.2 — build world-glyph atlas (webp + Deck mapping) from packages/map-assets/glyphs/svg
	node scripts/map-assets/build-glyph-atlas.mjs
map-render-verify: ## T-090.5 stub — per-phase render smoke (layer instance count + purity)
	@echo "map-render-verify: not implemented (T-090.5)"; exit 1

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

verify-migration: ## Run monorepo migration gate checks (V1–V27)
	./scripts/verify-monorepo-migration.sh

# ci-local mirrors .github/workflows/ci.yml (CODING_STANDARDS.md §0.3 CI-2, §11). Order:
# editorconfig (FMT-2) -> rust backend -> coding standards -> Leptos SPA -> schema; each
# sub-target is a separate $(MAKE) so a non-zero recipe halts the run (fail-fast). The Node
# steps (schema validate/citations + the t159 gate driver) use whatever `nvm use` selected.
ci-local: ## Full CI gate locally — mirrors ci.yml (run `make db-up` + `nvm use` first)
	$(MAKE) verify-editorconfig
	$(MAKE) rust-ci
	$(MAKE) verify-coding-standards
	$(MAKE) ci-local-leptos
	$(MAKE) ci-local-schema

ci-local-leptos: ## CI gate: Leptos SPA fmt + clippy(wasm32) + native tests + trunk release build (mirrors ci.yml website-leptos)
	cargo fmt -p website-leptos --check
	cargo clippy -p website-leptos --target wasm32-unknown-unknown
	cargo test -p website-leptos
	cd apps/website-leptos && trunk build --release

ci-local-schema: ## CI gate: schema validate (TEST-3) + @contract citation verify
	$(MAKE) schema-validate
	$(MAKE) verify-citations

