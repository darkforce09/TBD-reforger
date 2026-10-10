**Status:** live

# API decisions

The decisions that shape the website [API](/documentation/glossary/a_to_f.md#api) as a whole, one
entry each in the [decisions entry](/documentation/standards/templates/decisions_entry.md)
format, oldest first. A decision that concerns one domain alone lives in that domain's design
note under [API design notes](/documentation/crates/api/api_server/verification_evidence/README.md).

### 2026-07-31 — Configuration fails closed on values that cannot work

**Context:** A Discord bot token with a stray newline, blank OAuth credentials in production or an
upload directory left relative all loaded without complaint and failed later, at first use, as a
remote error: Discord answering 401 or the callback landing on `#error=discord_unreachable`,
which read as an outage rather than a misconfiguration.

**Decision:** `Config::validate` in `crates/api/api_configuration/src/configuration/mod.rs` refuses
the boot, naming the variable, when a required value is empty (`DATABASE_URL`, `JWT_SECRET`, and
outside development the Discord client id, secret and redirect URL and `UPLOAD_DIR`) or a set
value can never work (`DISCORD_BOT_TOKEN` with whitespace, a relative or padded `UPLOAD_DIR`, a
malformed `TRUSTED_PROXIES` entry); `DbPoolConfig::from_env` does the same for the
`TBD_DB_POOL_*` settings. An optional integration left empty (bot, webhook, guild) stays legal and
reports its absence by name where a path needs it.

**Consequences:** A misconfigured deployment never starts, and its error names the fix. A setting
enters `Config` together with the code that reads it, so no variable looks configured while
doing nothing. The rules are unit-tested in
`crates/api/api_configuration/src/configuration/tests/configuration.rs`; the
[environment variable reference](/documentation/crates/api/api_server/environment_variables.md)
lists them.

**Supersedes:** none.

### 2026-08-01 — Map assets are mounted below the rate limiter

**Context:** Every route sat behind an in-memory token bucket of 20 requests a second with a burst
of 40 per client. A cold [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) boot
fetches hundreds of terrain files from `/map-assets` at once, the satellite image as dozens of
range requests among them. Measured on the live stack, 145,858 of the 145,861 `429`s the limiter
had issued were `/map-assets`, and none were on `/auth/` or `/ingest/`, the routes it exists for.
A higher tier for the mount was one option; exemption was the other.

**Decision:** `crates/api/api_server/src/router.rs` mounts `/map-assets` and
`/map-assets/glyphs` below the `rate_limit` layer, so the limiter never sees them. Everything the
API answers for — `/api/v1`, `/healthz`, `/metrics`, `/uploads` and the built app — stays above
it and stays limited.

**Consequences:** The mount is a plain directory service with no database, session or credential,
so exemption opens nothing a request limit protected; the resource there is bytes, which a
request meter cannot price. The order is load-bearing: the mount must stay below the `rate_limit`
layer in `crates/api/api_server/src/router.rs`.

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
(`crates/api/api_configuration/src/configuration/proxy_network.rs`).

**Consequences:** A key any client could forge would limit nobody, while a shared key still
limits everyone, so the fail-safe default is the shared key. An operator behind a proxy must list
it; the staging compose file defaults the variable to `127.0.0.1/32`. The rule is tested in
`crates/api/api_server/tests/http_infrastructure/forwarded_for_trust.rs`.

**Supersedes:** none.

### 2026-09-18 — The crate is organised by domain

**Context:** The API is one crate serving eight areas of the platform. Organised by layer, every
change to one area touched folders shared with all the others, and nothing stopped one area's
handlers from calling another's or the shared layer from naming an area's concepts.

**Decision:** `crates/api/api_server/src/` holds `core`, `background_workers` and eight domains
(`identity_and_access`, `administration`, `operations`, `missions`, `match_telemetry`,
`command_center`, `community_content`, `server_infrastructure`). Each domain owns its route
table, handlers, services and models; `core::http_router` merges the tables under `/api/v1`
without a prefix, so a public path is the literal in the domain's `routes.rs`; and a route's
access tier is the extractor its handler takes, never its place in the router.

**Consequences:** `core` imports no domain outside `application_state.rs` and `http_router.rs`, no
domain's handlers import another domain's handlers, every domain exports a merged route table,
and `background_workers` is imported only by the binary. A cross-domain need goes through the
owning domain's services and models. Every handler carries the `/// @route` tag.

**Supersedes:** none.

### 2026-09-18 — Background workers own the schedule, not the work

**Context:** Several parts of the platform need work done without a request: expired tokens,
stale leaderboards, silent game servers, Discord changes nobody signs in to pick up. Timers
written inside domains would hide who runs what and when.

**Decision:** `crates/api/api_background_workers/src/` holds every interval task, and
`spawn_all` arms them once at boot from `crates/api/api_server/src/bin/api_server.rs`. A worker holds its
loop and calls a service of the domain that owns the data; three intervals are tunable by
environment variable, the rest are constants.

**Consequences:** A handler or a test reaches the same operation without a timer. Only the binary
imports the module (`background_workers_used_only_by_the_binary` in the architecture rules), and
the boot log states every resolved interval. Workers that claim shared work do so through leases
in Postgres, so two API processes never handle the same item at once.

**Supersedes:** none.

### 2026-09-23 — Game hosts are reached only through work they claim

**Context:** Operators control game servers (start, stop, restart, broadcast, kick, list players,
load a [mission](/documentation/glossary/g_to_m.md#mission)), and game servers need the compiled
missions they run. An API that reaches into hosts needs inbound access to each of them, and
missions staged as files on a host drift from what was approved.

**Decision:** An administrator's request becomes a durable
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) row. The
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) and the
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) poll the API outbound, each with a
per-server [machine credential](/documentation/glossary/g_to_m.md#machine-credential), claim the
commands of their executor kind and report the outcome. A game runtime fetches the bytes of the
[artifact](/documentation/glossary/a_to_f.md#artifact) its server's
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) names from
`GET /api/v1/game-runtime/artifacts/{artifactId}`; nothing is staged on disk. The API has no RCON
console route.

**Consequences:** A host needs no inbound port from the API, a machine acts only for its own
server and executor kind, and a revoked credential ends its sessions. Commands expire, re-queue or
settle as indeterminate in the fleet command reconciler, and a deployment is confirmed only by a
runtime session that reports the artifact's exact SHA-256. The
[fleet command ledger](/documentation/crates/api/api_server/verification_evidence/fleet_command_ledger.md)
and [mission artifacts](/documentation/crates/api/api_server/verification_evidence/mission_artifacts.md)
notes record the full semantics.

**Supersedes:** none.

### 2026-09-29 — Administrators send console commands over RCON, each line transmitted once

**Context:** Operators need the dedicated server's own console commands, such as `#players`, on
every [fleet instance](/documentation/glossary/a_to_f.md#fleet-instance) without a shell on the
host. The 2026-09-23 entry left the API without an RCON console route. BattlEye RCON runs over UDP
with no delivery guarantee, and the agent's RCON client retransmits an unanswered command, so a
console line sent that way could run twice.

**Decision:** `console_command` is a [fleet command](/documentation/glossary/a_to_f.md#fleet-command)
for the host-agent executor, open to administrators (`AdminUser`). Its one argument is a line of 1
to 256 bytes with no line break, no control character and no leading `@`; it is not idempotent, it
may change the server, and its window is 30 seconds. Migration `0061_fleet_console_command.sql`
admits the action in `fleet_commands_action_check`, and the request audit records the line. The
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) sends it through
`SessionRequest::ExecuteOnce`: it may repeat the login, transmits the line in one datagram once and
never resends it. The outcome is the reply cut to at most 4,096 bytes on a character boundary with
`response_truncated`; without a reply the result reads "no RCON response; the command may or may
not have run". Server Control sends it from `console_command_form.rs` and shows the outcome.

**Consequences:** The API still never connects to a host: a console line travels as a claimed
command like any other. A lost reply is reported as unknown and never retried, so the operator
decides whether to send the line again. The `fleet_console_command` API tests, the console and
`execute_once` cases of the `rcon_transport` suite, the contract parity fixture of
`fleet-command.schema.json` hold the decision in place.

**Supersedes:** the sentence "The API has no RCON console route." of
[2026-09-23 — Game hosts are reached only through work they claim](#2026-09-23--game-hosts-are-reached-only-through-work-they-claim);
the rest of that entry holds.

### 2026-10-03 — The API is a thin application over API crates

**Context:** The 2026-09-18 domain layout kept every domain, the shared layer and the background
workers as folders of one crate, so the dependency rules between them were a walk over source
imports that a test had to keep up to date, and nothing could be built, tested or reused apart
from the whole.

**Decision:** Each domain, each kernel concern and the background workers are crates under
`crates/api/` (`api_<domain>`, the kernel crates such as `api_state` and `api_http_layer`, and
`api_background_workers`). `crates/api/api_server/src/` holds only the thin application: `router.rs`, which
merges the eight domain crates' route tables under `/api/v1`; `composition.rs`, which builds the
application state with its concrete services; and the `api` and `import-registry` binaries. There
is no `core` folder.

**Consequences:** The crate boundary makes an undeclared import a compile error, so the Cargo
manifests hold the crate graph: the kernel crates depend on no domain, the domain crates depend on one another only along the one-way
domain graph, only the application depends on `api_background_workers` and only its `api` binary
names it, no crate imports another domain's handlers, and every domain crate's one route table is
merged by the router. The source-import layer ratchet that held the folders to the planned graph
before the crates existed is retired. Each crate tests and lints on its own
(`cargo test -p api_<name>`).

**Supersedes:** the folder layout of
[2026-09-18 — The crate is organised by domain](#2026-09-18--the-crate-is-organised-by-domain)
(`crates/api/api_server/src/` holding `core`, `background_workers` and eight domains, and the `core` import
rule); its domain ownership of route tables, handlers, services and models holds.

### 2026-10-03 — Ids at an API crate's boundary are typed

**Context:** With the API split into crates, every id crossing a crate boundary was a bare `Uuid`,
`String` or `i64`, so an event id passed where a mission id belonged compiled, and the Discord
user id appeared as `String`, `&str` and `Option<String>` in different domains.

**Decision:** Every id concept at a public boundary has one newtype in `api_identifiers`, declared
with the `newtype_ids` macros: the UUID table keys, the text keys (Discord snowflakes, Arma
player ids, keys the game runtime chooses) and the `BIGINT` keys. Each is serde- and
sqlx-transparent. A client's raw text that the handler must refuse with the contract's own message
is a submitted id (`SubmittedEventId`, `SubmittedMissionId`) the handler parses itself.

**Consequences:** The JSON, the SQL binds, the path parses, the response goldens and the stored
digests are byte-equal to the bare values', so no contract changes.
`cargo xtask verify crate-anatomy` refuses a public `id` or `*_id` field or parameter of a bare
type in any crate. A new table key gets its type in `api_identifiers` before its first handler.

### 2026-10-09 — The API server is a crate of the layout

**Context:** The thin application that assembles the API crates was the one API member outside the
crate layout: a package named `api` in its own top-level folder, with binaries `api` and
`import-registry`, outside the tier order the crate-tier law judges every other crate by.

**Decision:** The thin application is the crate `api_server` at `crates/api/api_server/`, tier 11
in the `crates/api` category, with the binaries `api-server` (the server) and
`import-item-registry` (the item registry import). It is an application package: no member
depends on it, in any table.

**Consequences:** `cargo xtask verify crate-tiers` judges it as any crate; its rule 7 keeps every
other member off it. The deploy builds `cargo build --release -p api_server --bin api-server`,
and the unit `deploy/systemd/tbd-website-api.service` runs `target/release/api-server` from
`crates/api/api_server/`, whose gitignored `.env` it loads. The unit name, its state directory
and the image tag keep their `tbd-website-api` names.

**Supersedes:** the package and binary names of
[2026-10-03 — The API is a thin application over API crates](#2026-10-03--the-api-is-a-thin-application-over-api-crates);
the rest of that entry holds.

### 2026-10-09 — Directory defaults are folders of the checkout

**Context:** The terrain and glyph mounts fell back to paths relative to the process working
directory, so a server started from another folder served nothing, and `UPLOAD_DIR` and
`EQUIPMENT_DATA_DIR` defaulted to paths that held only from the API crate's own folder.

**Decision:** In development, an unset `UPLOAD_DIR`, `EQUIPMENT_DATA_DIR`, `MAP_ASSETS_DIR` or
`GLYPH_ASSETS_DIR` is a folder of the checkout (`assets/scratch/api/uploads`, `assets/equipment`,
`assets/terrains`, `assets/glyphs`) joined onto the checkout root the walk up to the
`.ai/ROOT` marker finds, and a development boot outside any checkout with one of them
unset is refused. Outside development, `UPLOAD_DIR`, `MAP_ASSETS_DIR` and `GLYPH_ASSETS_DIR` are
required and absolute.

**Consequences:** The API resolves the same folders from any working directory inside the
checkout, and no default is a path relative to the working directory. The production refusal is
held by `production_refuses_each_unusable_setting_by_name` in
`crates/api/api_configuration/src/configuration/tests/configuration.rs`; the
[environment variable reference](/documentation/crates/api/api_server/environment_variables.md)
lists them.

**Supersedes:** the required directories of
[2026-07-31 — Configuration fails closed on values that cannot work](#2026-07-31--configuration-fails-closed-on-values-that-cannot-work),
which named `UPLOAD_DIR` alone; the rest of that entry holds.
