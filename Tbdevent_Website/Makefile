GO := ./.tools/go/bin/go
export PATH := $(CURDIR)/.tools/go/bin:$(PATH)

.PHONY: deps tidy migrate up down dev-api dev-web build-web build run

deps:
	$(GO) mod download

tidy:
	$(GO) mod tidy

migrate:
	DATABASE_URL=$${DATABASE_URL:-postgres://tbdevent:tbdevent@localhost:5432/tbdevent?sslmode=disable} \
	$(GO) run ./cmd/server/main.go & \
	sleep 2; kill $$! || true

up:
	docker compose up -d

down:
	docker compose down

dev-api:
	set -a && [ -f .env ] && . ./.env; set +a; \
	$(GO) run ./cmd/server/main.go

dev-web:
	cd web && npm run dev

build-web:
	cd web && npm ci && npm run build

build: build-web
	$(GO) build -o bin/tbdevent ./cmd/server

run: build
	set -a && [ -f .env ] && . ./.env; set +a; \
	./bin/tbdevent
