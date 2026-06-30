# TBD Reforger Platform — monorepo dev tasks (delegates to apps/website/).
COMPOSE := $(shell command -v docker >/dev/null 2>&1 && echo "docker compose" || echo "podman compose")
WEB := apps/website
# Go is often installed under ~/.local/go/bin and not on PATH (see CLAUDE.md).
# ~/go/bin is the default GOPATH/bin where `go install` drops tools (editorconfig-checker, T-125.5);
# golangci-lint lives in ~/.local/go/bin. Both are prepended so `make ci-local` resolves them.
export PATH := $(HOME)/.local/go/bin:$(HOME)/go/bin:$(PATH)

.PHONY: help db-up db-down db-logs seed api web test build tidy tickets ticket-list ticket-sync ticket-check ticket-check-strict schema-validate schema-codegen verify-citations verify-coding-standards verify-doc-layout verify-editorconfig verify-terrain verify-migration map-assets-link ci-local ci-local-backend ci-local-frontend ci-local-schema

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

api: ## Run the Go API (loads apps/website/.env; runs migrations on boot)
	cd $(WEB) && go run ./cmd/api

web: map-assets-link ## Run the Vite dev server
	cd $(WEB)/frontend && npm run dev

map-assets-link: ## Symlink packages/map-assets → frontend public/ (T-091.1 DEM fetch)
	@mkdir -p $(WEB)/frontend/public
	ln -sfn ../../../../packages/map-assets $(WEB)/frontend/public/map-assets

test: ## Run Go unit tests
	cd $(WEB) && go test ./...

test-it: ## Run integration tests against the local DB (needs `make db-up`)
	cd $(WEB) && TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/tbd_reforger?sslmode=disable go test ./internal/handlers/...

build: ## Build the Go API and the frontend
	cd $(WEB) && go build ./...
	cd $(WEB)/frontend && npm run build

tidy: ## Tidy Go modules
	cd $(WEB) && go mod tidy

schema-validate: ## Validate golden missions against shared schema
	cd packages/tbd-schema && npm ci --silent && node scripts/validate.mjs

schema-codegen: ## Regenerate Go + TS contract types from packages/tbd-schema/schema (DOCUMENTATION_STANDARDS §9.1)
	cd packages/tbd-schema && npm ci --silent && node scripts/codegen.mjs
	gofmt -w $(WEB)/internal/contract

verify-citations: ## Verify @contract citations + GO-7 @route route-match (DOCUMENTATION_STANDARDS §10, CODING_STANDARDS §2)
	node packages/tbd-schema/scripts/verify-contract-citations.mjs

verify-coding-standards: ## GO-9 imports + ERR-4 envelope + LOG-3 logging + SIZE file length + doc layout (CODING_STANDARDS §11)
	$(MAKE) verify-doc-layout
	@bash scripts/website/verify-handler-imports.sh
	@bash scripts/website/verify-error-envelope.sh
	@bash scripts/website/verify-handler-logging.sh
	@node scripts/website/verify-file-length.mjs

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
# editorconfig (FMT-2) -> backend -> frontend (incl. format:check FMT-3) -> schema; each
# sub-target is a separate $(MAKE) so a non-zero recipe
# halts the run (fail-fast). `go` resolves via the ~/.local/go/bin PATH export above; the
# frontend job uses whatever `nvm use` (.nvmrc -> Node 26) selected.
# Backend sub-steps run in this exact order: gofmt (FMT-1) -> CI-1 (scripts/website/verify-ci1.sh)
# -> golangci-lint run ./... -> go build -> test-it. golangci-lint resolves via the same
# ~/.local/go/bin PATH export as go. (CI-1's `only-new-issues` literal lives in the script, not this
# Makefile, so the §G doc-accuracy forbidden-rg can scan the Makefile cleanly.)
# golangci-lint: go install github.com/golangci/golangci-lint/v2/cmd/golangci-lint@latest
ci-local: ## Full CI gate locally — mirrors ci.yml (run `make db-up` + `nvm use` first)
	$(MAKE) verify-editorconfig
	$(MAKE) ci-local-backend
	$(MAKE) verify-coding-standards
	$(MAKE) ci-local-frontend
	$(MAKE) ci-local-schema

ci-local-backend: ## CI gate: gofmt (FMT-1) + CI-1 + golangci-lint + go build + test-it (needs `make db-up` @ :5434)
	test -z "$$(gofmt -l $(WEB)/internal $(WEB)/cmd)"
	@bash scripts/website/verify-ci1.sh
	cd $(WEB) && golangci-lint run ./...
	cd $(WEB) && go build ./...
	$(MAKE) test-it

ci-local-frontend: ## CI gate: npm ci + format:check (FMT-3) + lint + build + unit tests (run `nvm use` -> Node 26 first)
	cd $(WEB)/frontend && npm ci && npm run format:check && npm run lint && npm run build && npm test

ci-local-schema: ## CI gate: schema validate (TEST-3) + @contract citation verify
	$(MAKE) schema-validate
	$(MAKE) verify-citations
