# T-251 — release image for the Axum `api` binary (`website-api`).
#
# Build from the **monorepo root** (required: `website-map-engine` path dep +
# `contracts_v2` compile-time `include_str!` embeds in validate.rs / kit.rs):
#
#   docker build -f apps/website/Dockerfile -t tbd-website-api .
#   # or:
#   docker compose -f apps/website/docker-compose.staging.yml --profile api build
#
# Runtime listens on 0.0.0.0:$PORT (default 8080). Supply at least DATABASE_URL and
# JWT_SECRET (see apps/website/api_v2/.env.example). This image does **not** serve the
# Leptos SPA — use scripts/deploy/Caddyfile.website (COOP/COEP + /api reverse-proxy).

# syntax=docker/dockerfile:1

FROM docker.io/library/rust:1.95.0-bookworm AS builder
WORKDIR /src

# Trimmed workspace: only the crates the `api` binary links. Paths under apps/
# and contracts_v2/ stay monorepo-shaped so include_str! relatives resolve.
COPY Cargo.lock ./
RUN printf '%s\n' \
  '[workspace]' \
  'resolver = "3"' \
  'members = [' \
  '    "apps/website/api_v2",' \
  '    "apps/website/map-engine",' \
  '    "apps/website/graphics-engine",' \
  ']' \
  > Cargo.toml

COPY apps/website/api_v2 ./apps/website/api_v2
COPY apps/website/map-engine ./apps/website/map-engine
COPY apps/website/graphics-engine ./apps/website/graphics-engine
COPY contracts_v2/definitions ./contracts_v2/definitions
COPY contracts_v2/rules/kit-aliases.json ./contracts_v2/rules/kit-aliases.json

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
