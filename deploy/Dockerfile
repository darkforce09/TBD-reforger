# syntax=docker/dockerfile:1

# Release image of the API server binary `api` (package `api`, apps/api).
#
# Build from the repository root, which is the build context:
#
#   podman build -f deploy/Dockerfile -t tbd-website-api .   (or `docker build`)
#   # or:
#   docker compose -f deploy/compose.staging.yml --profile api build
#
# The builder stage compiles inside the real workspace: the root Cargo.toml, whose
# [workspace.package], [workspace.dependencies] and [workspace.lints] tables every member
# inherits, and Cargo.lock under `--locked`. Cargo loads every member before it builds one, and
# the lock file holds only for the member set it records, so the root .dockerignore narrows the
# context to the folders that hold the members plus the contract folders whose files the API
# embeds at compile time (contracts/definitions, contracts/rules); the API's migrations, which
# `sqlx::migrate!` embeds, ride with apps/api. The base image's tag is the toolchain
# rust-toolchain.toml pins.
#
# The runtime listens on 0.0.0.0:$PORT (default 8080). Supply at least DATABASE_URL and
# JWT_SECRET (see apps/api/.env.example). This image does not serve the single-page app:
# deploy/caddy/Caddyfile does, with COOP/COEP and the /api reverse proxy.

FROM docker.io/library/rust:1.95.0-bookworm AS builder
WORKDIR /src

COPY . .

ENV CARGO_TERM_COLOR=never
RUN cargo build --release --locked -p api --bin api \
 && test -x /src/target/release/api

FROM docker.io/library/debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /src/target/release/api /usr/local/bin/api

# The runtime storage root the compose file mounts a named volume on (UPLOAD_DIR points under
# it). Created here, owned by the API's uid, so the volume inherits a writable tree on first use
# instead of a root-owned mount point the process cannot write to.
RUN mkdir -p /srv/state/uploads && chown -R 65534:65534 /srv/state

# nobody (65534) — no shell, no home; secrets come from env / compose.
USER 65534:65534
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/api"]
