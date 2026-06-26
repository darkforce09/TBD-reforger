# TBD Reforger Platform — dev tasks.
# Uses `docker compose` if available, otherwise `podman compose`.
COMPOSE := $(shell command -v docker >/dev/null 2>&1 && echo "docker compose" || echo "podman compose")

.PHONY: help db-up db-down db-logs seed api web test build tidy tickets ticket-list ticket-sync ticket-check ticket-check-strict

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

db-up: ## Start local Postgres in the background
	$(COMPOSE) up -d db

db-down: ## Stop local Postgres (keeps the data volume)
	$(COMPOSE) down

db-logs: ## Tail the Postgres logs
	$(COMPOSE) logs -f db

seed: ## Apply data seeds (Discord role mappings) to the running DB
	$(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < internal/db/seeds/discord_roles.sql

api: ## Run the Go API (loads .env; runs migrations on boot)
	go run ./cmd/api

web: ## Run the Vite dev server
	cd frontend && npm run dev

test: ## Run Go unit tests
	go test ./...

test-it: ## Run integration tests against the local DB (needs `make db-up`)
	TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/tbd_reforger?sslmode=disable go test ./internal/handlers/...

build: ## Build the Go API and the frontend
	go build ./...
	cd frontend && npm run build

tidy: ## Tidy Go modules
	go mod tidy

tickets: ## Run Claude Code on ready tickets in parallel (see tickets/README.md)
	./scripts/ticket run

ticket-list: ## Show ticket queue status
	./scripts/ticket list

ticket-sync: ## Regenerate all ticket-derived docs from registry.json
	./scripts/ticket sync

ticket-check: ## Structural validation of ticket registry + outputs
	./scripts/ticket check

ticket-check-strict: ## Full validation including zero legacy planning IDs
	./scripts/ticket check --strict
