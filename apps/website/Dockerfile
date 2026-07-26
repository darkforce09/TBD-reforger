# T-251 — release image for the Axum `api` binary (`website-api`).
#
# Build from the **monorepo root** (required: `map-engine-core` path dep +
# `packages/tbd-schema` compile-time `include_str!` embeds in validate.rs / kit.rs):
#
#   docker build -f apps/website/Dockerfile -t tbd-website-api .
#   # or:
#   docker compose -f apps/website/docker-compose.staging.yml --profile api build
#
# Runtime listens on 0.0.0.0:$PORT (default 8080). Supply at least DATABASE_URL and
# JWT_SECRET (see apps/website/api/.env.example). This image does **not** serve the
# Leptos SPA — use scripts/deploy/Caddyfile.website (COOP/COEP + /api reverse-proxy).

# syntax=docker/dockerfile:1

FROM docker.io/library/rust:1.95.0-bookworm AS builder
WORKDIR /src

# Trimmed workspace: only the crates the `api` binary links. Paths under apps/,
# crates/, and packages/ stay monorepo-shaped so include_str! relatives resolve.
COPY Cargo.lock ./
RUN printf '%s\n' \
  '[workspace]' \
  'resolver = "3"' \
  'members = [' \
  '    "apps/website/api",' \
  '    "crates/map-engine-core",' \
  ']' \
  > Cargo.toml

COPY apps/website/api ./apps/website/api
COPY crates/map-engine-core ./crates/map-engine-core
COPY packages/tbd-schema/schema ./packages/tbd-schema/schema
COPY packages/tbd-schema/registry/kit-aliases.json ./packages/tbd-schema/registry/kit-aliases.json

ENV CARGO_TERM_COLOR=never
RUN cargo build --release -p website-api --bin api \
 && test -x /src/target/release/api

FROM docker.io/library/debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/api /usr/local/bin/api

# nobody (65534) — no shell, no home; secrets come from env / compose.
USER 65534:65534
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/api"]
