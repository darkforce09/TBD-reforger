# TBD Reforger Platform — monorepo dev tasks (delegates to apps/website/).
COMPOSE := $(shell command -v docker >/dev/null 2>&1 && echo "docker compose" || echo "podman compose")
WEB := apps/website
# Go is often installed under ~/.local/go/bin and not on PATH (see CLAUDE.md).
export PATH := $(HOME)/.local/go/bin:$(PATH)

.PHONY: help db-up db-down db-logs seed api web test build tidy tickets ticket-list ticket-sync ticket-check ticket-check-strict schema-validate verify-terrain verify-migration

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

web: ## Run the Vite dev server
	cd $(WEB)/frontend && npm run dev

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

verify-terrain: ## Manifest + anchor verify (stub mode OK until T-091.0 export)
	cd packages/tbd-schema && npm ci --silent && npm run verify-terrain

verify-terrain-strict: ## Full anchor alignment gate (T-091.0 Claude Code + MCP export)
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
