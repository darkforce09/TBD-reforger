**Status:** live

# API decisions

The decisions that shape the website [API](/documentation_v2/glossary.md#api) as a whole, one
entry each in the [decisions entry](/documentation_v2/standards/templates/decisions_entry.md)
format, oldest first. A decision that concerns one domain alone lives in that domain's evidence
note under [verification evidence](/documentation_v2/website/api_v2/verification_evidence/README.md).

### 2026-07-31 — Configuration fails closed on values that cannot work

**Context:** A Discord bot token with a stray newline, blank OAuth credentials in production or an
upload directory left relative all loaded without complaint and failed later, at first use, as a
remote error: Discord answering 401 or the callback landing on `#error=discord_unreachable`,
which read as an outage rather than a misconfiguration.

**Decision:** `Config::validate` in `apps/website/api_v2/src/core/configuration/mod.rs` refuses
the boot, naming the variable, when a required value is empty (`DATABASE_URL`, `JWT_SECRET`, and
outside development the Discord client id, secret and redirect URL and `UPLOAD_DIR`) or a set
value can never work (`DISCORD_BOT_TOKEN` with whitespace, a relative or padded `UPLOAD_DIR`, a
malformed `TRUSTED_PROXIES` entry); `DbPoolConfig::from_env` does the same for the
`TBD_DB_POOL_*` settings. An optional integration left empty (bot, webhook, guild) stays legal and
reports its absence by name where a path needs it.

**Consequences:** A misconfigured deployment never starts, and its error names the fix. A setting
enters `Config` together with the code that reads it, so no variable looks configured while
doing nothing. The rules are unit-tested in
`apps/website/api_v2/src/core/configuration/tests/configuration.rs`; the
[environment variable reference](/documentation_v2/website/api_v2/environment_variables.md)
lists them.

**Supersedes:** none.

### 2026-08-01 — Map assets are mounted below the rate limiter

**Context:** Every route sat behind an in-memory token bucket of 20 requests a second with a burst
of 40 per client. A cold [Mission Creator](/documentation_v2/glossary.md#mission-creator) boot
fetches hundreds of terrain files from `/map-assets` at once, the satellite image as dozens of
range requests among them. Measured on the live stack, 145,858 of the 145,861 `429`s the limiter
had issued were `/map-assets`, and none were on `/auth/` or `/ingest/`, the routes it exists for.
A higher tier for the mount was one option; exemption was the other.

**Decision:** `apps/website/api_v2/src/core/http_router.rs` mounts `/map-assets` and
`/map-assets/glyphs` below the `rate_limit` layer, so the limiter never sees them. Everything the
API answers for — `/api/v1`, `/healthz`, `/metrics`, `/uploads` and the built app — stays above
it and stays limited.

**Consequences:** The mount is a plain directory service with no database, session or credential,
so exemption opens nothing a request limit protected; the resource there is bytes, which a
request meter cannot price. The order is load-bearing and is held twice:
`the_exempt_mount_is_registered_below_the_rate_limit_layer` in
`apps/website/api_v2/src/core/middleware/tests/rate_limiting.rs` checks the source, and
`apps/website/api_v2/tests/map_assets_rate_limit_exemption.rs` checks that the routes above still
refuse.

**Supersedes:** none.

### 2026-08-01 — The rate limiter believes X-Forwarded-For only from listed proxies

**Context:** On the deploy host Caddy proxies from loopback, so the connection peer of every
public client was Caddy, and the whole community shared one strict bucket on `/auth/`: the
eleventh member to open the site within ten seconds was refused a token refresh. The header that
names the real client is text any client can write.

**Decision:** The rate limiter keys on the connection peer unless that peer is listed in
`TRUSTED_PROXIES`, and then on the rightmost `X-Forwarded-For` hop that is not a trusted proxy.
An empty list, the default, ignores the header entirely. A set entry that does not parse, or a
CIDR block not written as its network address, stops the boot
(`apps/website/api_v2/src/core/configuration/proxy_network.rs`).

**Consequences:** A key any client could forge would limit nobody, while a shared key still
limits everyone, so the fail-safe default is the shared key. An operator behind a proxy must list
it; the staging compose file defaults the variable to `127.0.0.1/32`. The rule is tested in
`apps/website/api_v2/tests/forwarded_for_trust.rs`.

**Supersedes:** none.

### 2026-09-18 — The crate is organised by domain

**Context:** The API is one crate serving eight areas of the platform. Organised by layer, every
change to one area touched folders shared with all the others, and nothing stopped one area's
handlers from calling another's or the shared layer from naming an area's concepts.

**Decision:** `apps/website/api_v2/src/` holds `core`, `background_workers` and eight domains
(`identity_and_access`, `administration`, `operations`, `missions`, `match_telemetry`,
`command_center`, `community_content`, `server_infrastructure`). Each domain owns its route
table, handlers, services and models; `core::http_router` merges the tables under `/api/v1`
without a prefix, so a public path is the literal in the domain's `routes.rs`; and a route's
access tier is the extractor its handler takes, never its place in the router.

**Consequences:** `apps/website/api_v2/src/tests/architecture_rules.rs` holds the layout on every
unit-test run: `core` imports no domain outside `application_state.rs` and `http_router.rs`, no
domain's handlers import another domain's handlers, every domain exports a merged route table,
and `background_workers` is imported only by the binary. A cross-domain need goes through the
owning domain's services and models. `cargo xtask verify route-tags` requires the `/// @route`
tag on every handler.

**Supersedes:** none.

### 2026-09-18 — Background workers own the schedule, not the work

**Context:** Several parts of the platform need work done without a request: expired tokens,
stale leaderboards, silent game servers, Discord changes nobody signs in to pick up. Timers
written inside domains would hide who runs what and when.

**Decision:** `apps/website/api_v2/src/background_workers/` holds every interval task, and
`spawn_all` arms them once at boot from `apps/website/api_v2/src/bin/api.rs`. A worker holds its
loop and calls a service of the domain that owns the data; three intervals are tunable by
environment variable, the rest are constants.

**Consequences:** A handler or a test reaches the same operation without a timer. Only the binary
imports the module (`background_workers_used_only_by_the_binary` in the architecture rules), and
the boot log states every resolved interval. Workers that claim shared work do so through leases
in Postgres, so two API processes never handle the same item at once.

**Supersedes:** none.

### 2026-09-23 — Game hosts are reached only through work they claim

**Context:** Operators control game servers (start, stop, restart, broadcast, kick, list players,
load a [mission](/documentation_v2/glossary.md#mission)), and game servers need the compiled
missions they run. An API that reaches into hosts needs inbound access to each of them, and
missions staged as files on a host drift from what was approved.

**Decision:** An administrator's request becomes a durable
[fleet command](/documentation_v2/glossary.md#fleet-command) row. The
[fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) and the
[game runtime](/documentation_v2/glossary.md#game-runtime) poll the API outbound, each with a
per-server [machine credential](/documentation_v2/glossary.md#machine-credential), claim the
commands of their executor kind and report the outcome. A game runtime fetches the bytes of the
[artifact](/documentation_v2/glossary.md#artifact) its server's
[mission deployment](/documentation_v2/glossary.md#mission-deployment) names from
`GET /api/v1/game-runtime/artifacts/{artifactId}`; nothing is staged on disk. The API has no RCON
console route.

**Consequences:** A host needs no inbound port from the API, a machine acts only for its own
server and executor kind, and a revoked credential ends its sessions. Commands expire, re-queue or
settle as indeterminate in the fleet command reconciler, and a deployment is confirmed only by a
runtime session that reports the artifact's exact SHA-256. The
[fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
and [mission artifacts](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
notes record the full semantics.

**Supersedes:** none.
