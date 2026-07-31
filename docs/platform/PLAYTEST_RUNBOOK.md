# PLAYTEST RUNBOOK — the live two-client E2E

**Closes `T-181.16` and `T-068.14` — the last two open programs.**
Both are `executor: human`; one session closes both.
Registry: [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) ·
T-181 hub: [`docs/mod/t181_event_mod_program.md`](../mod/t181_event_mod_program.md) ·
T-068 hub: [`t068_virtual_arsenal_program.md`](../specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md) ·
T-068.14 checklist: [`t068_14_phase2_e2e_gate.md`](../specs/Mission_Creator_Architecture/t068_14_phase2_e2e_gate.md)

You need: **you + one other person**, two Arma Reforger clients, one dedicated server.
Budget **90 minutes**: ~45 of pre-flight you can do alone the day before, ~45 with your friend.

> **Read §6 before you book the friend.** There are three things that can stop the
> session dead, and all three are checkable alone, headlessly, in about ten minutes
> (§2.5). Do that first.

---

## 1. What this proves and why it's the last thing

Every claim the mod makes today is **compile-verified, not runtime-verified**. `make mod-compile`
proves the Enfusion parses. `scripts/mod/world-boot.sh` proves the game-mode prefab wires up and a
mission document parses. Neither has ever had a **player** in it: a `--mission=` boot runs with zero
clients, so `BuildForPlayer`, `Serialise`, `Parse`, every RPC, every screen and every stage past
LOBBY have never executed — that limit is written down at
[`world-boot.sh:36-47`](../../scripts/mod/world-boot.sh) and again at
[`t181_event_mod_program.md:396-409`](../mod/t181_event_mod_program.md). One live session with two
real clients is the only instrument that can see any of it. It is not a formality; it is the first
observation.

It closes both programs at once because they meet at the same moment. **T-181.16** wants the whole
event loop — connect, slot, brief, deploy, die once terminally, be respawned by an admin.
**T-068.14** wants one frame of that loop looked at closely: the player who deploys is **wearing the
loadout a human authored in the website Arsenal**, gear *and* cargo, on a player entity and not the
test NPC. The second is a sub-step of the first, which is why T-068.14's own note says
"Satisfied by T-181.16". After this, T-181 and T-068 have no remaining slices, and the platform
factory's 339+ tickets have nothing left blocking them.

---

## 2. Pre-flight

### 2.0 Where commands run

**Everything in this runbook runs on the HOST** — your Bazzite/Fedora machine, not an agent
container. Agent shells are `debian:12` with glibc 2.36 and no C toolchain; `cargo` dies with
`linker cc not found` and host binaries die with `GLIBC_2.39 not found`
([`scripts/lib/hostrun.sh:5-17`](../../scripts/lib/hostrun.sh)). Neither means anything is broken.
You are the host, so just run them.

Repo root is `/home/Samuel/Projects/TBD-Reforger`. Every command below assumes you are there:

```bash
cd /home/Samuel/Projects/TBD-Reforger
```

### 2.1 Code gates (5 min, alone)

| # | Command | Expected | If not |
|---|---|---|---|
| P1 | `make mod-compile` | exits 0, ~1.3 s | non-zero + `file:line` → a `.c` is broken; do not proceed |
| P2 | `make mod-world-boot` | last line `WORLD BOOT: PASS` | `FAIL` → read `[TBD] roll-call:` in the printed log; a `=MISSING` name is a component that did not instantiate |

> **"`FAIL` → do not proceed" has exactly one documented exception, and you will hit it.** §2.5A's
> two boots both end `WORLD BOOT: FAIL` on `validator warnings rose:` — known, pre-existing, and
> not about your session. §2.5A says how to tell it from a real failure and what to read instead.
> That is the *only* `FAIL` this runbook tells you to walk past; treat every other one as real.
| P3 | `git -c filter.lfs.process= status --short` | only files you expect | a dirty mod tree means you are testing something that is not `main` |

`make mod-world-boot` prints a roll-call line of the form
`[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok PlayArea=ok Markers=ok Radio=ok Objectives=ok`
([`TBD_FrameworkManager.c:409-459`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c)).
All nine must read `=ok`.

### 2.2 The stack — API + SPA (10 min, alone)

Three terminals, all on the host, all from the repo root.

```bash
# T1
make db-up            # Postgres on :5434 (podman/docker compose)
# T2
make api              # Axum API on :8080, migrates on boot
# T3
make leptos           # Leptos SPA on :3000 (trunk serve --release; proxies /api -> :8080)
```

Verify:

```bash
curl -s http://127.0.0.1:8080/healthz
```

Expect a JSON body reporting database + migration state, HTTP 200
([`app.rs:906`](../../apps/website/api/src/app.rs), [`app.rs:1048`](../../apps/website/api/src/app.rs)).
A non-200, or `Connection refused`, means `make api` is not up — re-read T2's output; the API
hard-fails at boot on a bad `DATABASE_URL` / `JWT_SECRET`.

Seed the catalogue the Arsenal reads from (safe to re-run):

```bash
make seed
```

Grab the service token the game server will use — it is one value, not a list
([`config.rs`](../../apps/website/api/src/config.rs) reads `SERVICE_TOKEN`;
[`setup-server-profile.sh:33-37`](../../scripts/mod/setup-server-profile.sh) explains the
`GAME_SERVER_TOKENS` rename that used to break this):

```bash
grep '^SERVICE_TOKEN=' apps/website/api/.env
```

Expect one non-empty value. If the line is missing, add one (any long random string) and restart
`make api`, or the game server will 401 on every fetch.

Log in to the SPA without Discord:

```
http://127.0.0.1:3000
```
then open `http://127.0.0.1:8080/api/v1/auth/dev-login?role=admin` in the same browser
([`app.rs:883`](../../apps/website/api/src/app.rs)). It mints a real session and 302s to the SPA
callback. You should land logged in as an admin.

### 2.3 The mission (15 min, alone) — this is the T-068 half

You need a mission whose slots carry **gear and cargo**. The mission the server seeds by default
now has them: `setup-server-profile.sh` copies `bridgehead-at-levie.json` in as `msn_8f3a2c`
([`setup-server-profile.sh:92-100`](../../scripts/mod/setup-server-profile.sh)), and **T-605** gave
that golden real loadouts on 4 of its 18 slots — full gear + cargo (`blufor:Alpha:SL:0`), gear only
(`blufor:Alpha:AR:0`), cargo only aimed at a container the kit does **not** wear
(`blufor:Alpha:RFL:0` — the degrade path), the other faction (`opfor:Grom:SL:0`), and 14 left
kit-only. It really was **18 slots, 0 gear, 0 cargo** until then, which is exactly why the loadout
half of the spawn path had never been booted by any gate.

That covers T-181.16. It does **not** close T-068.14 on its own: that slice gates the **authoring**
path — SPA → Save Version → `/compiled` — and a hand-written golden says nothing about the
compiler. Author your own as well.

**Author it in the SPA (the path T-068.14 actually gates):**

1. `http://127.0.0.1:3000/missions` → **New Mission** (or `Cmd/Ctrl+N`). Terrain **Everon**.
2. In `/missions/:id/edit`, place at least **four** character slots across **two factions**
   (side discipline is only visible with two sides), in **two squads**.
3. Open the Arsenal on a slot — double-click the slot → **Attributes → Arsenal**, or select the
   squad in the ORBAT Manager and press **OPEN ARSENAL**
   ([`orbat_manager.rs:1576`](../../apps/website/frontend/src/orbat_manager.rs)).
   Author, on **different slots**, deliberately different shapes:
   - **slot A** — full gear: primary + optic + magazine + uniform + vest + helmet + pants + boots
     + backpack, **and** cargo rows into `vest` and `backpack`;
   - **slot B** — gear only, no cargo;
   - **slot C** — cargo only, no gear;
   - **slot D** — nothing (bare kit).
   This is exactly the coverage `packages/tbd-schema/golden-missions/slot-loadout-coverage.json`
   encodes; open it if you want a worked example of every field's shape.
4. **Save Version** (top strip). You need a saved version — `/compiled` 409s
   `no saved version to compile` without one
   ([`missions.rs:1606-1615`](../../apps/website/api/src/handlers/missions.rs)).
5. Copy the mission **UUID** out of the URL. **It must be a UUID.** `GET /missions/:id/compiled`
   calls `Uuid::parse_str` and 400s `invalid id` on anything else
   ([`missions.rs:1756-1759`](../../apps/website/api/src/handlers/missions.rs)) — so the
   content-hash ids like `msn_8f3a2c` only work through the on-disk fallback, never through the API.

**Prove the API will serve it, before the game server ever asks:**

```bash
MID=<paste-the-uuid>
TOK=$(grep '^SERVICE_TOKEN=' apps/website/api/.env | cut -d= -f2- | tr -d '"'"'"'\r')
curl -s -o /tmp/compiled.json -w '%{http_code}\n' \
  -H "X-Service-Token: $TOK" \
  "http://127.0.0.1:8080/api/v1/missions/$MID/compiled"
```

- **`200`** — good. Check it carries the loadouts:
  ```bash
  python3 -c "import json;d=json.load(open('/tmp/compiled.json'));s=d['slots'];print(len(s),'slots;',sum(1 for x in s if (x.get('loadout') or {}).get('gear')),'gear;',sum(1 for x in s if (x.get('loadout') or {}).get('cargo')),'cargo')"
  ```
  You want non-zero on both. Zero gear = the Arsenal edits never reached the saved version; go back
  to step 3 and Save Version again.
- **`400`** — the id is not a UUID.
- **`409 no placed slots`** — you saved a version with no character slots.
- **`409 no saved version to compile`** — you never pressed Save Version.
- **`500`** — the stored payload is unreadable or violates `mission.schema.json`
  ([`validated_compiled_body`, `missions.rs:1464`](../../apps/website/api/src/handlers/missions.rs)).
  The body names the reason. A `mission_versions` row is immutable, so **save a new version**; you
  cannot repair the old one.

**Attach it to an event** (needed only for roster-driven seating — see S6):

`http://127.0.0.1:3000/admin/events` → create an event → attach the mission. The auto-ORBAT
materialises slots from the mission's `orbat`. Copy the **event UUID**. Then confirm the game-server
view of it:

```bash
EID=<paste-the-event-uuid>
curl -s -H "X-Service-Token: $TOK" \
  "http://127.0.0.1:8080/api/v1/ingest/events/$EID/roster" | head -c 400; echo
```

Expect `{"eventId":"…","missionId":"…","assignments":{…}}`
([`events.rs:2469`](../../apps/website/api/src/handlers/events.rs)). `assignments` is keyed on
`users.arma_id` — **it will be empty until someone links their game identity** (S6). An empty map is
legal: everybody falls to round-robin seating, and the lobby picker still works.

### 2.4 The dedicated server (2 min, alone)

> **UPDATED 2026-07-31 (T-604). There is now one command; do not hand-assemble this any more.**
>
> ```bash
> bash scripts/mod/run-playtest-server.sh --mission-id=$MID
> ```
>
> **Start it WITHOUT `--admin`. You do not have your identityId yet and cannot get it here**
> (T-608). The engine only prints it when a client authenticates, so the value lives in the log
> of a server you have already joined — that is **S3**, and it is a chicken-and-egg you would
> otherwise hit on the very first command of the session. The flow is:
>
> | | |
> |---|---|
> | **here (§2.4)** | boot with no `--admin`. The script prints a `NOTE:` saying `#tbd` will answer "admin only" — that is correct and expected, not a fault |
> | **S2** | join. That authentication is what mints the id |
> | **S3** | grep the id out of the log, **stop this server**, restart with `--admin=<id>` |
>
> Everything up to S3 works fine without admin. Only `#tbd` and the S13 admin-respawn item need it.
>
> **Do not start a second one while the first is up.** Since T-608 it refuses rather than
> orphaning the first — the run dir `$HOME/tbd-playtest` is fixed and shared, and the old
> behaviour was to rewrite `server.json` under the running server, delete the pidfile that was
> the only handle on it, and then die on port 2001. Stop the first, wait for
> `==> stopped (process group confirmed gone)`, then start the next.
>
> It does every step 2.4.1–2.4.4 below (profile, backend config, addon symlink, `server.json`),
> passes both flags, and then **waits for the room registration and asserts the local addon won**
> before printing anything. It prints the join address and Direct Join Code from that boot's own
> log. `--dry-run` shows the rendered config and the exact command; `--help` lists the options.
> `--mission-file=<path>` stages a golden on disk so you can run with no API up.
>
> Three things it knows that the hand-assembly below did not:
>
> * **`game.admins[]` is schema-validated by the engine**, and a bad entry is a hard fatal ~90 s
>   into the boot, after a full script compile. The value must match `^[0-9]{17}$` (SteamID) or
>   `^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$` (identityId) — both patterns
>   quoted verbatim out of the engine's own rejection. The script checks before booting.
> * **`tbd-framework` IS published to the Workshop**, unlisted, under the same id as the local
>   gproj GUID, at a stale **1.0.1**. §6.2's "not published to the Workshop" was wrong. This means
>   `-config` alone does *not* fail loudly — it quietly downloads and runs June's build. The
>   script hard-fails if that copy wins.
> * **`-profile <dir>` must not be the same tree you point `-addonsDir` at.** The engine unpacks
>   the Workshop copy into `<profile>/addons/TBDFramework_<GUID>/` on every boot; with
>   `-addonsDir` present the live source still wins, and the script proves it did rather than
>   assuming.
>
> The manual steps below are kept because they are the anatomy of what the script does, and
> because you will need them if you are debugging the script itself.

**Why there was no script (the original text, now fixed).** The two that looked like they would
start a server, did not:

- [`scripts/mod/run-dev-server.sh`](../../scripts/mod/run-dev-server.sh) was **27 lines and never
  launched anything** — two preflight checks, then it ended. It is now a shim that execs
  `run-playtest-server.sh` or exits 2 with a pointer; it can no longer look like it worked.
- [`scripts/mod/deploy-staging.sh:1153`](../../scripts/mod/deploy-staging.sh) builds the `config`-mode
  ExecStart **without `-addonsDir`**, so the local addon in `game.mods[]` is not what loads — the
  stale Workshop copy is. Its `addons`-mode branch at `:1155` does pass `-addonsDir`, but with
  `-addons`/`-server` instead of `-config`, and that mode registers no backend room: re-measured
  2026-07-31, **zero** `Server registered with address:` lines against a healthy
  `[TBD][Stage] LOADING -> LOBBY` in the same log. Both branches are still wrong; T-604 did not
  touch `deploy-staging.sh`.

The combination that **is measured to work** is `-addonsDir <dir>` **plus** `-config <json>`, with
the addon listed in the config's `game.mods[]` keyed by the GUID from `addon.gproj`
([`world-boot.sh:11-27`](../../scripts/mod/world-boot.sh), engine 1.7.0.54). Confirmed 2026-07-31 to
register a **joinable room** as well as load the mod — which `world-boot.sh` had never been asked
to check, since it boots headless with zero players. `-config` and `-addons` together are a hard
fatal; `-addons` + `-scenarioId` with no `-config` prints "Game successfully created" and then
never hosts.

Stage it once:

```bash
# 2.4.1 — profile + backend config + registry fallback
bash scripts/mod/setup-server-profile.sh "$HOME/tbd-playtest/profile"
```
Expect `Profile ready at: /home/…/tbd-playtest/profile (game data under …/profile/profile)`.
`$profile:` resolves to `<-profile-arg>/profile/`, **not** `<-profile-arg>/` — seeding one level up
loads nothing, silently ([`world-boot.sh:383-384`](../../scripts/mod/world-boot.sh)).

```bash
# 2.4.2 — point the mod at the API and at YOUR mission
CFG="$HOME/tbd-playtest/profile/profile/TBD_BackendConfig.json"
python3 - "$CFG" "$MID" "$EID" "$TOK" <<'PY'
import json,sys
p,mid,eid,tok=sys.argv[1:5]
d=json.load(open(p))
d["backendUrl"]="http://127.0.0.1:8080"; d["serverToken"]=tok
d["missionId"]=mid; d["eventId"]=eid
json.dump(d,open(p,"w"),indent=2)
PY
cat "$CFG"
```
Expect all four keys populated, `serverToken` matching `SERVICE_TOKEN`, `missionId` the UUID.
Shape reference: [`Data/backend.example.json`](../../apps/mod/tbd-framework/Data/backend.example.json).

> `backendUrl` is `127.0.0.1:8080` **only if the API and the game server are on the same box.** If
> the server is elsewhere, put the API host's LAN IP here and make sure the API is reachable from it.

```bash
# 2.4.3 — addon staging dir (a symlink, exactly like deploy-staging.sh:1100)
mkdir -p "$HOME/tbd-playtest/addons"
ln -sfn "$PWD/apps/mod/tbd-framework" "$HOME/tbd-playtest/addons/tbd-framework"
ls -l "$HOME/tbd-playtest/addons"
```

```bash
# 2.4.4 — server config: the committed dev config, made joinable, with the LOCAL addon injected
LANIP=$(ip route get 1.1.1.1 | awk '{print $7; exit}')
python3 - scripts/mod/tbd-dev-server.config.json "$HOME/tbd-playtest/server.json" "$LANIP" <<'PY'
import json,sys
src,dst,ip=sys.argv[1:4]
c=json.load(open(src))
c["publicAddress"]=ip
c["game"]["visible"]=True
c["game"]["maxPlayers"]=8
c["game"]["admins"]=[]              # filled in S3 with your real identityId
c["game"]["mods"]=[{"modId":"B2C3D4E5F6A78901","name":"TBD_Framework"}]
json.dump(c,open(dst,"w"),indent=2)
PY
cat "$HOME/tbd-playtest/server.json"
```
`B2C3D4E5F6A78901` is the GUID from
[`apps/mod/tbd-framework/addon.gproj`](../../apps/mod/tbd-framework/addon.gproj) — the same one
`world-boot.sh` reads out of that file at `:376`. Ports stay **2001 game / 17777 A2S**; they must
differ or the engine logs `NETWORK (E): Unable to start replication` and exits **status 0**
([`STAGING-SERVER.md:246-250`](../mod/STAGING-SERVER.md)).

Open the ports:

```bash
sudo firewall-cmd --add-port=2001/tcp --add-port=2001/udp --add-port=17777/udp
```
(add `--permanent` and `--reload` if you want it to survive a reboot).

**Start it:**

```bash
cd "$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server"
./ArmaReforgerServer \
  -addonsDir "$HOME/tbd-playtest/addons" \
  -config    "$HOME/tbd-playtest/server.json" \
  -profile   "$HOME/tbd-playtest/profile" \
  -maxFPS 60 -logStats 30000 -nothrow
```

This is [`world-boot.sh:773-778`](../../scripts/mod/world-boot.sh) with real ports and a real
profile. Leave it in the foreground so you can watch it. The log also lands at:

```bash
LOG="$(ls -td "$HOME"/tbd-playtest/profile/logs/logs_* | head -1)/console.log"
echo "$LOG"
```

**The server binary exits 0 even when compilation fails. Read the log, never `$?`**
([`t181_event_mod_program.md:245`](../mod/t181_event_mod_program.md)).

### 2.5 Do this ALONE, before you book the friend (10 min)

Three headless checks that catch the three things most likely to end the session. All are cheap;
none needs a client.

**A — will a loadout-carrying mission even reach LOBBY?**

> ### READ THIS FIRST — this step ends in `WORLD BOOT: FAIL` and that is EXPECTED (T-608)
>
> §2.1 P2 tells you "`FAIL` → do not proceed." **That rule does not apply to this step**, and if
> nobody had said so you would have stopped here, on your highest-value solo pre-flight, over a
> problem that has nothing to do with your session. Measured on `main` 2026-07-31, both boots in
> this section end exactly like this:
>
> ```
>   ok    mission validated: mission result=PASS errors=0 warnings=4
>   FAIL  validator warnings rose: 4 > baseline 0 for msn_5c1de7
> WORLD BOOT: FAIL
> ```
>
> and `5 > 0 for msn_8f3a2c` for the bridgehead mission. Note the line above it: the mission
> **validated**, `result=PASS errors=0`. The `FAIL` is the warning *ratchet*
> ([`world-boot.sh:248-258`](../../scripts/mod/world-boot.sh)), not the validator.
>
> **How to tell this known FAIL from a real one — check all three:**
>
> 1. The failing line says **`validator warnings rose:`**. Any other `FAIL` line is real.
> 2. The line above it says **`mission result=PASS errors=0`**.
> 3. Every warning is from the **unconsumed-key family** — keys the mission document authors
>    that the mod's structs do not model yet. There are exactly five, and no others are known:
>
>    | | |
>    |---|---|
>    | `environment` | weather / time of day not applied at load |
>    | `settings` | respawn policy, spectator rules, night-vision not applied |
>    | `layers` | decoration-layer aliases discarded on load |
>    | `factions.tickets` | respawn pools not applied |
>    | `orbat.roles.radio` | spawn net tuning not applied |
>
>    ```bash
>    grep -F '[TBD][Validate] WARNING' <run-dir>/profile/logs/logs_*/console.log
>    ```
>
>    `slot-loadout-coverage` produces four of them (no `layers`); `bridgehead-at-levie` produces
>    all five. **A warning naming anything else is not this, and is worth stopping for.**
>
> **Why it is not fixed here.** Both the validator and
> [`.world-boot-warning-baseline`](../../.world-boot-warning-baseline) predate this wave; the
> warnings are pre-existing and cosmetic to the session. Widening the baseline to `4`/`5` would
> silence a real ratchet to make a doc read nicer, so T-605 refused and so does T-608. The ratchet
> is doing its job; the runbook was the thing that was wrong, by not warning you.
>
> **So what IS the pass signal for step A?** Not the exit code. These two lines, from the log:
>
> ```
> [TBD][Slots] loadout settle complete — …  — spawn open
> [TBD] Stage → LOBBY
> ```
>
> If both are present, a loadout-carrying mission reached LOBBY and step A has passed. Match the
> **prefix** `[TBD][Slots] loadout settle complete` — the counts after it are expected to vary and
> their wording has already changed once (T-605). Verbatim from `main` 2026-07-31:
> `[TBD][Slots] loadout settle complete — 7 application(s), 0 unplayable, 3 with a shortfall — spawn open`
> for `slot-loadout-coverage`, and `18 application(s), 0 unplayable, 2 with a shortfall` for
> `bridgehead-at-levie`.

```bash
bash scripts/mod/world-boot.sh --mission=slot-loadout-coverage --keep-logs
```
Then in the printed run directory:
```bash
grep -E 'loadout settle|loadout delivery REFUSED|loadout SHORTFALL|loadout pass complete|loadout DEGRADED|Stage →' <run-dir>/profile/logs/logs_*/console.log
```
> **Raise the capture window or you will see none of this.** The loadout pass takes ~3–4 s to
> settle (6 × 500 ms wear verify + weapon phase + cargo) and the harness kills the server
> `TBD_WORLDBOOT_SETTLE` seconds (default **4**) after the `mission result=` line — which lands
> *before* the bodies do. Prefix the command with `TBD_WORLDBOOT_SETTLE=25` for every check in this
> section. At the default the log simply ends mid-pass and looks clean.

- **WANT:** `[TBD][Slots] loadout settle complete — N application(s), 0 unplayable, M with a
  shortfall — spawn open`
  ([`TBD_SpawnManager.c`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c))
  followed by `[TBD] Stage → LOBBY`. **A non-zero `M` is fine** — see the next bullet.
- **NOT a stop:** `[TBD][Slots] loadout SHORTFALL on M of N slot(s) — the session IS open …`, and
  the per-slot `loadout SHORTFALL` / `loadout DEGRADED` WARNINGs behind it. Those players are
  dressed, armed and playable; they are carrying an item somewhere other than where the mission
  put it, or fewer of it. `slot-loadout-coverage` produces **3 of 7** today (two full vests and
  two optics that will not seat) and reaches LOBBY. Read them, note them, carry on.
- **STOP if you see:** `[TBD][Slots] loadout delivery REFUSED at spawn boundary — N slot body(ies)
  are UNPLAYABLE — LOBBY/deploy will not open`
  ([`TBD_SpawnManager.c`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
  Since **T-605** this fires only for a body nobody could play — an authored prefab that does not
  load, a storage or weapon slot the character does not have, a garment that would not go on — and
  the line names the slot and the item. It is still mission-wide, deliberately: fix the document.
  See §6.1.
- **Rule of thumb for the whole log:** a TBD loadout `SCRIPT (E)` means the session will not open;
  a TBD loadout `SCRIPT (W)` means it will, carrying less than authored.

Now do the same against **your** mission (`--compiled` fetches from the live API, so `make db-up` +
`make api` must be running):

```bash
bash scripts/mod/world-boot.sh --compiled=$MID --keep-logs
```
Same greps, same verdict — **including the same expected `WORLD BOOT: FAIL`**, which on this lane
is keyed `compiled` rather than `msn_…`. Apply the same three-part test from the box above: the
failing line says `validator warnings rose:`, the line above it says `result=PASS errors=0`, and
every warning belongs to the five-key family. **This is the single highest-value pre-flight in the
runbook** — it feeds the exact bytes the game server will get to the real Enfusion parser, with no
client needed. Judge it on `loadout settle complete` + `Stage → LOBBY`, not on the exit code.

**B — will the second client be able to find the server?** `run-playtest-server.sh` now checks this
for you: it will not print its "SERVER UP" banner at all unless `Server registered with address:`
appeared in that boot, and the address it prints is parsed from that line rather than guessed. If
it fails, it names the phase the engine actually reached and exits 1.

> **The one failure to know by name: a REGISTRATION HANG** (T-608, measured 2026-07-31). The world
> comes up, the mod loads the mission, the bodies spawn, `Stage → LOBBY` appears — and the room
> never registers. The script now says so in those words, prints every `BACKEND` line, and tells
> you plainly that **no `(E)` line names this**: the engine logs nothing at all when the backend
> handshake stalls, so the evidence is an absence. Its fingerprint is
> `BACKEND : Attempting online Game Config instead.` with no BACKEND line after it.
>
> **It is not your mod and it is not your config, and the fix is to run the same command again.**
> Measured: the identical command registered in **13 s** a few minutes after a boot where it never
> registered across a full 300 s. Registration time is variable — the script no longer implies a
> number, it reports the phase.
>
> Two things the script's own output settles for you, so do not go looking:
> `[TBD][Mission] backend refused the mission fetch — http=400` is **benign** (it is the on-disk
> fallback working, and it appears on passing boots too), and the ~79 `DEFAULT`/`MATERIAL`/
> `RESOURCES` `(E)` lines are the vanilla floor, present when everything works. The script demotes
> them and says how many it suppressed.
>
> **If you ever see a `STRAY SERVER` block, act on it before doing anything else.** It means the
> script could not confirm the server died — it names the process group and the exact command.
> Until that group is gone it holds 2001/17777 and your next boot fails on
> `Unable to start replication`, which looks like a completely different bug.

To check by hand (or against a server someone else started):
```bash
grep -E 'Server registered with address|Direct Join Code' "$LOG"
```
No registration line means Direct Join will answer "No server found" no matter how healthy the
server is. Also confirm the friend can reach you at all — `ping -c2 <server-lan-ip>` from their
machine — and that the room advertised your **LAN** address, not a container or loopback one:
```bash
grep -E 'Server registered with address' "$LOG"    # must show your 192.168.x.x, not 127.0.0.1
```
Ports: this machine had `1025-65535/tcp 1025-65535/udp` already open (`firewall-cmd --list-ports`),
so 2001/17777 needed nothing. Check yours before blaming the mod. See §6.2 — **registration is not
the same question as whether the client loads the right mod**, and only the first one is settled.

**C — can the mod parse and validate the mission?**
```bash
grep -E '\[TBD\]\[Mission\] loaded|\[TBD\]\[Validate\] mission result=' "$LOG"
```
Want `[TBD][Mission] loaded id=<uuid> name='…' slots=N source=backend` and
`[TBD][Validate] mission result=PASS errors=0 warnings=…`
([`TBD_Log.c:75-99`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Log.c)).
`source=profile` means the backend fetch failed and it fell back to disk — your token or URL is
wrong. `result=FAIL` keeps the server in LOADING forever; `#tbd validate` replays the findings.

### 2.6 The two operator-only RCON steps (10 min) — **OPTIONAL for this session**

> **Everything in §3 works without these.** They enable exactly one thing: the SPA's
> **Server Control** page (`/admin/server`) being able to start/stop/restart the game server unit
> over `POST /api/v1/admin/servers/{id}/rcon`
> ([`app.rs:837`](../../apps/website/api/src/app.rs)). Until both are done, that endpoint answers
> **503** — correctly — and the session is unaffected. **No agent has ever run either of these; they
> mutate a live host.** Do them if you want the last unexercised platform surface exercised too.

They only work when the API and the game server are **sibling `systemctl --user` units under one
uid on one box**. The transport is a UNIX socket in `$XDG_RUNTIME_DIR` with `SocketMode=0600`, so
the OS is the credential and there is no secret
([`.env.example:123-127`](../../apps/website/api/.env.example),
[`deploy-staging.sh:92-137`](../../scripts/mod/deploy-staging.sh)). **If your API runs under
`make api` on one machine and the game server on another, this cannot work at all** — the socket
would be on the wrong box.

**Step A — tell the API where the socket is.**

*If the API runs from a systemd user unit* (the staging/home-server shape,
[`HOME_SERVER.md:282-306`](../website/HOME_SERVER.md)):

```bash
systemctl --user edit --full tbd-website-api.service
#   add under [Service]:
#   Environment=GAME_AGENT_SOCKET=%t/tbd-reforger-agent.sock
systemctl --user daemon-reload
systemctl --user restart tbd-website-api.service
systemctl --user show -p Environment --value tbd-website-api.service | tr ' ' '\n' | grep GAME_AGENT_SOCKET
```
Expect `GAME_AGENT_SOCKET=/run/user/<uid>/tbd-reforger-agent.sock`. systemd expands `%t` for you;
the literal string to paste is documented verbatim at
[`.env.example:135`](../../apps/website/api/.env.example).

*If you are running the API with `make api`* there is no unit — put it in the env file the API
loads, then restart `make api`:
```bash
printf 'GAME_AGENT_SOCKET=/run/user/%s/tbd-reforger-agent.sock\n' "$(id -u)" >> apps/website/api/.env
grep GAME_AGENT_SOCKET apps/website/api/.env
```
The path must be **absolute and free of leading/trailing whitespace**, or the API refuses at boot
with `ConfigError::Malformed` ([`config.rs:83-90`](../../apps/website/api/src/config.rs)) — which is
deliberate: the alternative is an ENOENT at 03:00 that reads as "the game host is down".

**Step B — install the host control agent, once.**

Prove it locally first (no ssh, no deploy, no `deploy.env` needed — the script exits at
[`deploy-staging.sh:657-660`](../../scripts/mod/deploy-staging.sh) before it requires any of that):

```bash
bash scripts/mod/deploy-staging.sh --agent-selftest /tmp/tbd-agent-selftest
```
Expect the rendered agent to be driven against a stub `systemctl` and report the unit's **real**
state. It must be able to report a *dead* unit as dead even when the verb exited 0 — that is the
entire reason the agent exists ([`deploy-staging.sh:139-152`](../../scripts/mod/deploy-staging.sh)).

Then install for real. **This runs a full deploy** — rsync, compose rebuild, game-server restart —
and needs `scripts/deploy/deploy.env`, which **does not exist in this checkout**:

```bash
cp scripts/deploy/deploy.env.example scripts/deploy/deploy.env
$EDITOR scripts/deploy/deploy.env    # TBD_SSH_HOST, token, paths
TBD_INSTALL_AGENT=1 bash scripts/mod/deploy-staging.sh --dry-run   # look first
TBD_INSTALL_AGENT=1 bash scripts/mod/deploy-staging.sh
```
Expect, near the end:
```
==> host control agent (T-289)
  agent VALID: unit=tbd-reforger.service socket=%t/tbd-reforger-agent.sock dwell=8s
  agent socket listening at ${XDG_RUNTIME_DIR}/tbd-reforger-agent.sock
```
Without `TBD_INSTALL_AGENT=1` you get `[SKIP] agent install — TBD_INSTALL_AGENT=1 to enable.`
([`deploy-staging.sh:1229-1231`](../../scripts/mod/deploy-staging.sh)). The step deliberately fails
the deploy if the socket did not come up, rather than reporting it installed.

Verify end to end from the SPA: `/admin/server` → a status/restart action should return **202** with
`delivered` / `state` / `detail` in the body, not 503.

---

## 3. The session, numbered

Both clients: Arma Reforger with the `tbd-framework` mod loaded. Keep the server log tailing in a
terminal the whole time:

```bash
tail -f "$LOG" | grep --line-buffered -E '\[TBD\]'
```

---

**S1 — start the server and let it settle.**
Run the §2.4 command. Watch for, in order:

```
[TBD] roll-call: SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok PlayArea=ok Markers=ok Radio=ok Objectives=ok
[TBD][Mission] loaded id=<uuid> name='…' slots=N source=backend
[TBD][Validate] mission result=PASS errors=0 warnings=…
[TBD][Slots] Slot-1 <slot-id> (<faction>) kit <kit> at <pos>          (× N)
[TBD][Spawn] slot=<id> Y=… jsonY=… surfaceY=… delta=… heading=…       (× N)
[TBD][Loadout][Slot] slot=<id> loadout pass complete gear=x/x cargo=y/y
[TBD][Slots] materialized N/N bodies — A with a JSON loadout, B kit-only, 0 failed
[TBD][Slots] loadout settle complete — N application(s), 0 unplayable, M with a shortfall — spawn open
[TBD][Stage] LOADING -> LOBBY
[TBD] Stage → LOBBY
[TBD][Spawn] LOBBY: auto-deploy wave OFF — the slot picker is the way in (admin override: '#tbd deploy <playerId>').
```

**Means what if you don't see it:**
- any `=MISSING` in the roll-call → that component's class did not resolve; the feature will never
  run and no other error will say so
  ([`TBD_FrameworkManager.c:409-459`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c)).
- `source=profile` → the backend fetch failed; you are testing a stale on-disk mission, not the one
  you authored. Fix the token/URL and restart.
- `mission result=FAIL` → the server stays in LOADING on purpose. `#tbd validate` in chat replays the
  findings ([`TBD_MissionValidator.c:1353`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c)).
- `loadout delivery REFUSED at spawn boundary … UNPLAYABLE` → **stop**. LOBBY will not open for
  anyone; the line names the slot and the offending item. §6.1.
- `loadout SHORTFALL on M of N slot(s)` → **not** a stop. Those players are playable and the round
  starts; they are carrying an item elsewhere, or fewer of it, than the mission authored. §6.1.
- `LOBBY: auto-deploy wave ON` → the picker will open and close itself ~500 ms later and you will
  see no UI and no error. `m_bAutoDeploy` must be `0`; it is set to `0` on the prefab at
  [`TBD_GameMode.et:7`](../../apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et), so seeing
  `ON` means you are running a different prefab than the one in this tree.

---

**S2 — client 1 (you) connects.**
Multiplayer → **Direct Join** → `<server-lan-ip>` port `2001`, or paste the **Direct Join Code**
from the server log.

**Should see:** the world loads; you are *not* in a body; the **LOBBY** slot picker opens.

**Server log:** a vanilla `Authenticated player:` / player-audit line naming your player id.

**If Direct Join says "No server found":** the server registered no backend room. `grep -E 'Server
registered with address|Direct Join Code' "$LOG"` — if empty, see §6.2. This is *not* a firewall or
version problem if `ping` works and the A2S port answers.

---

**S3 — make yourself an admin.** (Do this now; you need it at S13.)

Find your identityId in the server log — the engine prints it on authentication, and the
comma-separated list of them is what `game.admins[]` takes
([`deploy.env.example:56-58`](../../scripts/deploy/deploy.env.example)):

```bash
grep -iE 'identityId' "$LOG" | tail -5
```

Then restart the server with it — the flag is repeatable. **STOP THE RUNNING SERVER FIRST**
(T-608):

```bash
# 1. In the terminal running the server: Ctrl-C. Wait for this exact line before going on —
#    it is the script confirming the process group is gone, not assuming it:
#        ==> stopped (process group confirmed gone)
#
#    If you get a STRAY SERVER block instead, run the command it prints and check
#    `pgrep -af '[A]rmaReforgerServer'` comes back empty. Do not skip this: a survivor
#    holds 2001/17777 and the next boot dies on `Unable to start replication`.
#
# 2. Only then:
bash scripts/mod/run-playtest-server.sh --mission-id=$MID --admin=<your-identityId>
```

> **Running it before stopping the first one used to break both servers**, and silently: the
> second invocation deleted the pidfile that was the first one's only handle, so the first
> printed "server exited" while its engine was still alive, and the second then died on the
> port. Since T-608 the second invocation **refuses** — it names the live process group and
> exits without touching anything. If you see that refusal, you skipped step 1.

You will lose the connection and have to Direct Join again; the **Direct Join Code is re-minted
on every boot**, so read the new one from the new run, not from your scrollback.

> **The engine schema-validates this, and a bad value is a HARD FATAL** ~90 s into the boot, after
> a full script compile — `BACKEND (E): JSON is invalid!` → `There are errors in server config!` →
> `ENGINE (E): Unable to initialize the game`. It must match **one** of these two, quoted verbatim
> out of the engine's own rejection (1.7.0.54):
>
> * `^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$` — identityId, **lowercase** hex
> * `^[0-9]{17}$` — SteamID, exactly 17 digits
>
> `run-playtest-server.sh` rejects anything else before booting, so you find out in a millisecond
> instead of a minute and a half.

> **`passwordAdmin` is a different mechanism and does NOT make you a `#tbd` admin.** `TBD_AdminService.IsAdmin`
> reads `SCR_PlayerListedAdminManagerComponent.IsPlayerOnAdminList`, whose list is populated only in
> `OnPlayerAuditSuccess` from the native `BackendApi.IsListedServerAdmin(playerId)` — i.e. from
> `game.admins[]` at connect time
> ([`SCR_PlayerListedAdminManagerComponent.c:26-40`](../../apps/mod/vanilla_reference/Source/SCR_PlayerListedAdminManagerComponent.c)).
> Logging in with `#login <passwordAdmin>` never touches that array. `game.admins[]` is the only route.

> **Ignore this warning; it is not about you.** A combined-mode boot logs
> `BACKEND (W): !!! JsonApi Array name="admins" found in JSON - which missing adequate script
> declaration, items will be skipped!` — it comes from a *scripted* mirror of the config, not from
> the native path the admin manager reads, and it fires identically for `disableNavmeshStreaming`,
> which is a fully supported engine key. The native side accepted the array: it is what
> schema-validated your id in the first place.

**Verify:** type `#tbd` in game chat.

**Should see:**
`TBD: #tbd missions | mission <n> | backend <url> [token] | refresh | validate | dead | respawn <playerId> | deploy <playerId> | stage [next|<NAME>] | safestart [status|go|<seconds>] | identity [status|override <phrase>|enforce] | audit | menu`
([`TBD_AdminCommands.c:255`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c)).

**`TBD: admin only.`** means you are not on the list. `IsAdmin` asks vanilla's
`SCR_PlayerListedAdminManagerComponent`
([`TBD_AdminService.c:60-70`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AdminService.c)),
which is populated from `game.admins[]` — and **`game.admins[]` only exists in `-config` mode.**
Without an admin you cannot do S13 (admin respawn) and T-181.16 cannot pass.

**Nothing at all** means the chat intercept did not fire. `F8` also opens the admin screen
([`Configs/System/Actions/TBD_AdminMenu.conf`](../../apps/mod/tbd-framework/Configs/System/Actions/TBD_AdminMenu.conf));
try that before concluding chat is dead.

---

**S4 — the identity gate.** In chat:

```
#tbd identity status
```

**Should see:** a verdict that ONE LIFE is enforceable. **If instead the log carries**

```
[TBD][Spawn] player=N has NO durable identity (keyMode=NUMERIC) — … This is NOT a supported state …
```
([`TBD_SpawnManager.c:1496`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)),
then **`SAFE_START` and `LIVE` will be refused** and the session cannot reach the death step. Two
options:

1. Fix it — a dedicated server with no backend identity is a misconfiguration; check `publicAddress`
   and that the server registered a room.
2. Waive it, verbatim, and record that you did:
   ```
   #tbd identity override I-ACCEPT-NO-ONE-LIFE
   ```
   ([`TBD_SpawnManager.c:256`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
   Deaths will **not** survive a reconnect, so S13's respawn is still testable but the *durability*
   of one life is not. **Write it in the sign-off.** `#tbd identity enforce` undoes it.

---

**S5 — client 2 (friend) connects.** Same Direct Join.

**Should see:** LOBBY opens for them too, and their client's roster shows **your** claim, without
either of you doing anything — the open lobby re-asks the server every **2000 ms**
([`TBD_LobbyScreen.c:58-66`](../../apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyScreen.c)).

**If their screen is blank/black with no rows:** the roster arrived but the layout collapsed. That
exact failure — a `~10px` sliver with one character per line — is a known shape from the first live
load-in, fixed at T-181.51 and gated by `bash scripts/mod/verify-ui-layouts.sh`. Run it; if it
passes, capture a screenshot and the client log (§5) and file it.

---

**S6 — squad reservation and roster seating (website → game).** *Optional but it is the only test of
the website↔game seat contract.*

In the SPA: `/events/:id` → the mission dossier → reserve a squad / register for a slot as one of the
two accounts. Then, in game, that player links their identity so the roster can match them:

```
#tbd link <code>
```
([`TBD_IdentityLink.c:147-210`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_IdentityLink.c)).
`assignments` in `/ingest/events/:id/roster` is keyed on `users.arma_id`, which **only** that command
writes ([`TBD_RosterLoader.c:1-17`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_RosterLoader.c)).

**Should see:** on the next mission load, `[TBD] Roster loaded (N assignments).` and the linked
player is seated into their reserved slot rather than round-robin.

**If `assignments` is `{}`:** nobody is linked. Everyone falls to round-robin. That is a legal state
— note it and move on; it does not block either ticket.

> The code in `#tbd link <code>` **is visible in chat before the mod can suppress it** — `super`
> distributes the message before TBD sees it, so returning true suppresses nothing (T-327, deferred,
> `executor: workbench`). Codes are single-use and consumed on success, so a leaked one is already
> spent. Don't be surprised by it.

---

**S7 — LOBBY: the slot picker.**

**Should see** ([`TBD_LobbyScreen.c:8-28`](../../apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyScreen.c)):
a header with the mission name, terrain, "N of M seats open" and the stage; a **PICK YOUR SEAT**
section listing sides; **one** side expanded and **one** group within it, never a flat wall; an
**ORDERS → View briefing** row; a footer status line and a single loud **DEPLOY** button, shown
**disabled** while you hold no seat.

- Sides are **not** filtered — you can see the other faction's seats. That is deliberate: "you cannot
  pick a side you cannot see; side discipline starts at the briefing" (T-181.9.1).
- There is **no Back button.** The stack is empty beneath LOBBY, so back would be a trap.

**If no screen opens at all:** check the server log for `GUI (E): Menu preset 'TBD_UILobby' not
found!`. That was a five-screen blocker; the cause was empty `GameProjectConfig` blocks, fixed by
listing **both** vanilla's and ours in `MenuConfigs`
([`addon.gproj:9-22`](../../apps/mod/tbd-framework/addon.gproj)). If the error is back, that file
regressed.

**Client-side log line to confirm the picker armed:** `[TBD][Lobby] Tick ARMED after N attempt(s)`
([`TBD_LobbyController.c:816`](../../apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyController.c)).
`[TBD][Lobby] Start GAVE UP` means `TBD_FrameworkManager` is not on the same prefab as
`TBD_LobbyComponent` — wiring, not timing.

---

**S8 — claim, contend, release.**

1. You click a seat. **Should see:** it flips to **YOUR SEAT** immediately (optimistic), DEPLOY
   enables, footer reads "You hold ALPHA · TL. Click it again to give it up."
   **Server log:** `[TBD][Spawn] claim player=<id> slot=<key>`
   ([`TBD_SpawnManager.c:785`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
   Watch it for ~4 s: it must **not** flicker away and back. A flicker means the pending-intent
   reconciliation regressed and reads exactly like losing the seat.
2. Friend clicks **the same seat**. **Should see:** refusal, with **your name** on the seat, cleared
   after ~5 s. **Server log:** `[TBD][Spawn] claim rejected player=<id> slot=<key> (held by another player)`
   ([`TBD_SpawnManager.c:780`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
   If the refusal names nobody, or names the wrong person, the refusal is not reading the authority's
   own roster — capture and file.
3. You click your own seat again. **Should see:** it returns to OPEN, DEPLOY disables.
   **Server log:** `[TBD][Spawn] release player=<id> slot=<key>` ([`:823`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
4. Re-claim it. Both of you end holding **different** seats — you on **slot A** (the fully-loaded
   one from §2.3), friend on **slot C** (cargo-only). This matters at S10.

---

**S9 — briefing.** Click **ORDERS → View briefing** in the lobby, or advance the stage:

```
#tbd stage next
```

**Should see:** `[TBD][Stage] LOBBY -> BRIEFING` + `[TBD] Stage → BRIEFING` in the log, and the
briefing screen opens **on both clients** — the controller opens it on the stage push and closes it
on any other phase ([`TBD_BriefingController.c:337-350`](../../apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_BriefingController.c)).
Content should be side-specific: your side's orders, not both.

**Nothing auto-advances out of BRIEFING** — `flow.briefingSeconds` is announced and advisory
([`TBD_FrameworkManager.c:568`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c)).
You drive it.

**If a client that joined late sees no briefing:** that is the catch-up path
(`TBD_CatchUpStage`); note whether it recovered on its own.

---

**S10 — DEPLOY, and the loadout. This is T-068.14.**

Back to LOBBY (`#tbd stage LOBBY`) if you left it, then both of you press **DEPLOY**.

**Should see, per player, in the server log:**

```
[TBD] SpawnManager: assigned slot <id> to player <n> at (<x>,<z>)
[TBD] SpawnManager: bound player <n> to slot <key> body (kit <kit>)
[TBD][Spawn] player=<n> possess request accepted
```
([`:666`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c),
[`:2381`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c),
[`:2549`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).

**Position and heading** — for your slot, the earlier line
`[TBD][Spawn] slot=<id> Y=… jsonY=… surfaceY=… delta=… heading=<deg>`
([`:1161`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)) must match the
X/Z/heading you authored, and `delta` must be small. A
`jsonY=… deviates … m from surfaceY=…` warning at `:1131` means stale DEM or a mis-authored slot.

**The kit actually worn** — the tag is **`[TBD][Loadout][Slot]`**, set at
[`TBD_SpawnManager.c:1179`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c).
(The T-068.14 spec and the header comment at
[`TBD_LoadoutEquipComponent.c:17`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c)
both say `[TBD][Loadout][Player]`. **That string does not exist in any Print.** Grep for `[Slot]`.)

```bash
grep -E '\[TBD\]\[Loadout\]\[Slot\]' "$LOG"
```
Per dressed slot you want:
```
[TBD][Loadout][Slot] slot=<id> primary equip OK {…}Rifle_M16A2.et [ … ]
[TBD][Loadout][Slot] slot=<id> cargo {…}Magazine_…et x6/6 -> vest
[TBD][Loadout][Slot] slot=<id> worn-audit jacket=1 pants=1 boots=1 kit=<kit> (settled on attempt K of 6, T ms)
[TBD][Loadout][Slot] slot=<id> loadout pass complete gear=10/10 cargo=8/8
```
([`:610`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c),
[`:1175`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c),
[`:1342`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c),
[`:1386`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c)).

**Then look with your eyes, because no log proves this:** open the inventory (`I`) and third-person
if enabled. You must see, on the **player entity**, the uniform/vest/helmet/pants/boots/gloves you
authored, the right rifle with the right optic and magazine, and the cargo in the containers you
named. **Screenshot it — this is T-068.14 P6 and there is no substitute.**

Negative control (T-068.14 P8): the test-NPC harness is **off by default**
(`m_bRunLoadoutTest` defaults `"0"` and `TBD_GameMode.et` does not override it —
[`TBD_LoadoutEquipComponent.c:107-108`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c)),
so any `[TBD][Loadout][TestNPC]` line in the log means something enabled it and your screenshot may
be of an NPC. There should be none.

**The no-garment degrade path.** Since **T-605** you no longer have to provoke this — the seeded
mission carries it. `blufor:Alpha:RFL:0` in `bridgehead-at-levie.json` is `kit:us_rifleman` (which
wears no backpack) with a cargo row aimed at `backpack`, so every boot of the session mission runs
the case. Confirm it in the log rather than building a second mission:

```
[TBD][Loadout][Slot] slot=blufor:Alpha:RFL:0 cargo:backpack DEGRADED item=… — this slot's kit wears no backpack — mission/kit authoring mismatch, NOT a mod fault; the item is still inserted via the any-storage fallback
[TBD][Loadout][Slot] slot=blufor:Alpha:RFL:0 loadout SHORTFALL gear=0/0 cargo=1/1 — the slot is playable and the session is NOT refused; …
[TBD][Slots] loadout settle complete — 18 application(s), 0 unplayable, 2 with a shortfall — spawn open
[TBD][Slots] loadout SHORTFALL on 2 of 18 slot(s) — the session IS open … blufor:Alpha:SL:0 [cargo:vest x2]; blufor:Alpha:RFL:0 [cargo:backpack x1]
[TBD][Admin] LOADOUT: 2 slot(s) did not get the authored loadout (session opened anyway): …
```

**The behaviour to judge, in game:** open that player's inventory. The tourniquet **is** on them,
in whatever storage would take it rather than in a backpack they do not have; the round started
normally; and `#tbd audit` shows the admin entry naming the slot. If the session instead refuses,
or if the DEGRADED line appears with no `SHORTFALL` roll-up and no admin entry, that is a finding.

**The `SCRIPT (E)` case** — a slot the mod calls UNPLAYABLE — is the one that still keeps everyone
in LOADING, and it is not reachable from the SPA (the Arsenal only offers registry items), so there
is nothing to rehearse here. If you see `loadout delivery REFUSED at spawn boundary — N slot
body(ies) are UNPLAYABLE`, the line names the slot and the item; fix the document.

---

**S11 — objectives, play area, markers, radio.**

| What | Do | Expect | If not |
|---|---|---|---|
| Objectives | open the map (`M`) | objectives from the mission document, per side | log has no `[TBD][Objectives]` lines → registry never built |
| Markers | open the map | markers you authored, on the vanilla placed-marker system | absent → check `[TBD][Markers]`; marker labels are byte-truncated (`CapLabel`, known UTF-8 exposure) |
| Play area | walk out of the boundary zone | private chat: `TBD: you are outside the play area (<zone>) -- return within Ns.` and, on a KILL zone, ` ONE LIFE: you will be killed and cannot respawn.` ([`TBD_PlayAreaComponent.c:378-405`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Zones/TBD_PlayAreaComponent.c)) | no message → either the mission authored no boundary zone (legal — restricts nobody) or the enforcer is not ticking |
| Radio | check your radio's frequency | net **assigned and displayed** | **automatic tuning will NOT work** — see §6.3. Log says `backbone: MISSING …` ([`TBD_RadioComponent.c:135`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioComponent.c)) |

---

**S12 — go LIVE and kill someone once.**

```
#tbd safestart status        # is damage off?
#tbd stage next              # BRIEFING -> SAFE_START
#tbd safestart go            # end warmup early
#tbd stage next              # SAFE_START -> LIVE
```

If `SAFE_START` is refused with
`SAFE_START has no enforcement on this world (TBD_SafestartManager is missing…)`
([`TBD_FrameworkManager.c:756-762`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c)),
go straight to `#tbd stage LIVE` and **tell your friend weapons are hot**.
If it is refused with an identity reason, go back to S4.

Now have your friend die — once, terminally.

**Should see:**
```
[TBD][Spawn] player=<n> KILLED — one life spent (key=<key>), slot retained, awaiting admin
```
([`TBD_SpawnManager.c:2710`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
Their client should go to **SPECTATOR** (`F` free cam, `←`/`→` next/prev, `TAB` roster, `V` view —
[`Configs/System/Actions/`](../../apps/mod/tbd-framework/Configs/System/Actions)).

**Then prove one life is real:** have them try to take another seat. **Should see**
`[TBD][Spawn] claim rejected player=<n> slot=<key> (one life spent)`
([`:773`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)) and
`release rejected … (one life spent — slot retained)` ([`:808`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
If they can claim a fresh seat, **one life is broken** — that is a headline finding.

Instead of `re-armed for respawn (slot retained)` at `:2715`? That means `m_bOneLife` is off on the
prefab and you are not testing the event configuration.

---

**S13 — admin respawn.** In chat, as the admin from S3:

```
#tbd dead
#tbd respawn <playerId>
```

**Should see:**
```
[TBD][Admin] respawn player=<n> by=<You(3)> result=<…>
[TBD][Admin] respawn player=<n> by=<You(3)> — back in the world, life restored
```
([`:2860`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c),
[`:2878`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).
Their client leaves spectator and they are in a **freshly dressed** body — the slot body is
rematerialised, so **check the loadout again**: `[TBD][Slots] rematerialized body for slot <key> (…)
— freshly dressed from mission JSON` ([`:2280`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).

- `respawn REFUSED … — not dead` / `— disconnected` ([`:2839`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c), [`:2845`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)) — you named the wrong id; `#tbd dead` lists them.
- `— RETRY queued, player stays DEAD until a body lands` ([`:2886`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)) — wait, then re-run the command.
- `did NOT deploy (…) — player REMAINS dead, run '#tbd respawn <n>' again` ([`:2891`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)) — run it again; if it never lands, capture and file.

Then `#tbd audit` — the trail should carry both the death and your respawn.

---

**S14 — reconnect (JIP).** Have your friend **disconnect and rejoin** while still alive, then
again after dying.

**Should see:**
- alive: `[TBD][JIP] player=<n> left ALIVE — seat <key> released, reclaim recorded under key <k> keyMode=<mode>` ([`:3050`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c))
- dead: `[TBD][JIP] player=<n> left DEAD — seat <key> retained under key <k> reclaimable=<0|1>` ([`:3047`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c))
  and on rejoin `[TBD][Spawn] player=<n> rejoined on a spent life — slot <key> handed back (still dead)` ([`:719`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c)).

If a dead player rejoins **alive**, one life did not survive the reconnect — expected only if you
took the S4 waiver.

---

**S15 — end the round.** `#tbd stage END`, or let a JSON win condition fire.

**Should see:** `[TBD][Stage] LIVE -> END` and, if the mission authored one, a win-condition line
naming the winner. `winConditions.endOn` on your mission decides which are reachable — a `time_limit`
of 5400 s will not fire in a 90-minute session, so force it.

---

**S16 — capture before you tear down.** §5. Do this **while the server is still running** — the log
directory is per-run.

---

## 4. What counts as PASS

Both tickets close **only if every line below passes.** One FAIL closes neither.

**T-181.16 — the event loop**

| # | Item | Evidence |
|---|---|---|
| 1 | Two real clients connected to a dedicated server simultaneously | both in the roster; two `Authenticated player` lines |
| 2 | LOBBY slot picker opened on **both** clients and rendered readable rows | screenshot ×2 |
| 3 | Claiming a seat worked, and did **not** flicker | `[TBD][Spawn] claim player=… slot=…` |
| 4 | A contended claim was refused **and named the actual holder** | `claim rejected … (held by another player)` + screenshot of the refusal |
| 5 | Releasing a seat worked in LOBBY | `[TBD][Spawn] release player=…` |
| 6 | Briefing opened on both clients and was side-specific | screenshot ×2 |
| 7 | Deploy put both players in the world at the authored X/Z/heading | `assigned slot`, `possess request accepted`, `slot=… heading=…` |
| 8 | Objectives, markers and the play-area warning were all observed | screenshots + the play-area chat line |
| 9 | One terminal death; the dead player could **not** claim a new seat | `KILLED — one life spent` + `claim rejected … (one life spent)` |
| 10 | Spectator worked for the dead player | screenshot |
| 11 | An admin respawned them and they came back **dressed** | `respawn … back in the world, life restored` + `rematerialized body` + screenshot |
| 12 | The round reached END | `[TBD][Stage] … -> END` |

**T-068.14 — the Arsenal**

| # | Item | Evidence |
|---|---|---|
| P1 | Loadout assigned to a character slot in the Mission Creator, version saved | SPA screenshot + version semver |
| P2 | `GET /api/v1/missions/<uuid>/compiled` → **200**, body carries `slot.loadout.gear` **and** `slot.loadout.cargo` | the `curl` + the python count from §2.3 |
| P3 | The dedicated server loaded that document from the **backend**, not disk | `[TBD][Mission] loaded … source=backend` |
| P4 | Slot claimed through the production LOBBY picker (not the auto-deploy wave) | `LOBBY: auto-deploy wave OFF` + `claim player=…` |
| P5 | Spawned at the slot with the correct kit alias | `bound player … to slot … body (kit …)` |
| **P6** | **Screenshot of the human player wearing the authored loadout** — primary, uniform, vest, helmet, and the cargo in the authored containers | **the screenshot. Nothing else counts.** |
| P7 | `[TBD][Loadout][Slot] … loadout pass complete gear=x/x cargo=y/y` for that slot, and a `claim` line for the same slot id | log excerpt |
| P8 | No `[TBD][Loadout][TestNPC]` lines anywhere — P6 is a player entity | `grep -c TestNPC "$LOG"` → `0` |
| P9 | The no-garment degrade path was provoked and behaved as documented (§S10) | the three log lines |

Sign-off template: [`t068_14_phase2_e2e_gate.md:50-62`](../specs/Mission_Creator_Architecture/t068_14_phase2_e2e_gate.md).
On PASS:

```bash
./scripts/ticket advance-slice T-068
./scripts/ticket done T-068
./scripts/ticket done T-181
./scripts/ticket sync
```

---

## 5. When something fails — capture once, act later

**You should never have to reproduce a bug twice.** Capture before you restart anything; the log
directory is created per run and a restart starts a new one.

**Server log** (the `-profile` argument, then `logs/logs_*`):

```bash
LOG="$(ls -td "$HOME"/tbd-playtest/profile/logs/logs_* | head -1)/console.log"
ERR="$(dirname "$LOG")/error.log"
mkdir -p ~/tbd-playtest/evidence/$(date +%F-%H%M)
cp "$LOG" "$ERR" ~/tbd-playtest/evidence/$(date +%F-%H%M)/ 2>/dev/null
```

**Both client logs** (Proton, app id **1874880** — the client and the server are different Steam
apps, so never compare their buildids; compare the `1.7.0.x` version string):

```bash
CLOG="$(ls -td "$HOME"/.local/share/Steam/steamapps/compatdata/1874880/pfx/drive_c/users/*/My\ Games/ArmaReforger/logs/logs_* | head -1)/console.log"
cp "$CLOG" ~/tbd-playtest/evidence/$(date +%F-%H%M)/client-1.log
```
Get your friend to send you theirs from the same path on their machine.

**Also capture:**
- the compiled document you were running: `cp /tmp/compiled.json ~/tbd-playtest/evidence/…/`
- the server config: `cp "$HOME/tbd-playtest/server.json" ~/tbd-playtest/evidence/…/`
- `#tbd audit` output (screenshot — it is chat, not a file)
- **screenshots of every screen that was wrong**, plus one of a screen that was right, for contrast

**The greps that classify a failure fastest:**

```bash
# did the game mode wire up?
grep -E '\[TBD\] roll-call' "$LOG"
# unresolvable class — the engine's own diagnostic, name-independent, the load-bearing one
grep -E "WORLD \(E\): Unknown class" "$LOG"
# any TBD error at all
grep -E '\(E\):.*TBD|\[TBD\].*(REFUSED|FAILED|NAKED|HALF-DRESSED|MISSING)' "$LOG"
# menu presets — a screen that cannot open
grep -E "Menu preset '.*' not found" "$LOG" "$ERR"
# the loadout chain end to end
grep -E '\[TBD\]\[Loadout\]|\[TBD\]\[Slots\]' "$LOG"
# the spawn chain
grep -E '\[TBD\]\[Spawn\]|SpawnManager:' "$LOG"
# stages
grep -E '\[TBD\]\[Stage\]|\[TBD\] Stage' "$LOG"
```

**Do not classify error ownership by message text and let the remainder pass.** Plenty of
TBD-relevant errors carry neither a `[TBD]` tag nor a path — e.g. `Instance of class TBD_SpawnManager
is null`, or a `Virtual Machine Exception` with no `(E)` marker. That heuristic let six genuine TBD
failures through once ([`t181_event_mod_program.md:226-234`](../mod/t181_event_mod_program.md)).
When in doubt, keep the whole log.

**Filing it:** add a `queued` ticket to [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json)
with the evidence paths in the `summary`, then `./scripts/ticket sync`. Put the log files under
`.ai/artifacts/` (pipeline output only) and reference them by path.

**Do not use `scripts/mod/remote-log-grep.sh` to decide PASS/FAIL.** It is stale: it requires the
literals `Mission loaded` and `built slot spawn`
([`remote-log-grep.sh:48`](../../scripts/mod/remote-log-grep.sh)), and **neither is emitted by the
current code** — `"Mission loaded"` survives only inside an error string
(`TBD_FrameworkManager.c:488` "Mission loaded but invalid"), and `built slot spawn` exists nowhere.
The pass-criteria list in [`STAGING-SERVER.md:192-203`](../mod/STAGING-SERVER.md) is stale for the
same reason. Use the greps above.

---

## 6. Known limitations going in

These are things that are **already known** to be broken, unbuilt, or unverifiable. None is a
surprise; all of them can bite during the session.

### 6.1 A single degraded cargo row refuses the LOBBY for everyone — **FIXED (T-605)**

Kept here rather than deleted, because it was this runbook's highest-risk entry and the shape of the
fix is what you will be reading in the log.

**What it was.** `IsComplete()` is `m_aFailures.IsEmpty() && m_aDegraded.IsEmpty()`
([`TBD_LoadoutEquipHelper.c`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipHelper.c)),
so **DEGRADED counted as incomplete**, and T-541 made incomplete a hard gate at the spawn boundary:
one degraded item on one slot set `m_bLoadoutDeliveryRefused`, `m_bSlotBodiesMaterialized` stayed
false, and **nobody** left LOADING. It was never a hypothetical — booted against the committed
golden `slot-loadout-coverage.json`, 3 of its 7 slots refused the world, one of them
(`blufor:Ranger:SL:0`) while reporting `gear=10/10 cargo=8/8`: nothing missing at all, six magazines
simply displaced out of a full vest.

**What it is now.** The boundary reads `HasBlockingFailure()`, not `IsComplete()`. It refuses only a
body nobody could play — an authored prefab that will not load, a storage or weapon slot the
character does not have, a garment that would not go on. A full container, an optic that will not
seat, or a container the kit does not wear is a **SHORTFALL**: the session opens, that player plays,
and the miss is reported per item (WARNING), per slot, once for the session, and in the admin trail
(`#tbd audit`). Log level is the contract — loadout `SCRIPT (E)` = the session will not open,
loadout `SCRIPT (W)` = it will, carrying less.

**And the gate now runs it.** `bridgehead-at-levie.json` — the mission `wave.sh`'s `world boot
+mission` step boots and the one `setup-server-profile.sh` seeds — had **0 gear and 0 cargo**, which
is why none of this had ever executed. T-605 gave it four real loadouts including the unworn-
container degrade, so every wave gate from now on drives the path.

**Still worth doing:** §2.5 A against your own mission before the friend arrives — a shortfall is not
a stop, but you would rather know which players are short before they are standing in the AO.

### 6.2 The client may silently load a STALE mod — **FIXED server-side, open client-side**

**Server side: closed by T-604.** [`scripts/mod/run-playtest-server.sh`](../../scripts/mod/run-playtest-server.sh)
starts a joinable, mod-loaded, admin-capable server in one command, and refuses to report success
unless the room registered *and* your checkout is the copy that loaded. `run-dev-server.sh` is now
a shim that execs it. `deploy-staging.sh`'s two branches are both still wrong at
[`:1153`](../../scripts/mod/deploy-staging.sh) / [`:1155`](../../scripts/mod/deploy-staging.sh) —
T-604 did not touch that file.

**Client side: still open, and this is the live risk for the session.** The premise this section
used to rest on was wrong: **`tbd-framework` IS published to the Workshop.** Unlisted, under the
*same* id as the local gproj GUID `B2C3D4E5F6A78901`, pinned at **version 1.0.1** (2026-06-14).
`TBD_WORKSHOP_MOD_ID` being commented out at
[`deploy.env.example:43`](../../scripts/deploy/deploy.env.example) meant only that nobody wrote the
id down — not that no publish exists. Measured 2026-07-31 on a clean profile with no `-addonsDir`:
the engine printed `BACKEND: Addon Download started B2C3D4E5F6A78901 - TBD Framework` /
`Downloading B2C3D4E5F6A78901 version 1.0.1` and pulled it over the network.

So the failure mode is **not** "the mod does not load". It is:

- **On the server, if you use `-config` without `-addonsDir`:** June's build runs and everything
  looks fine. Tell: **7** `[TBD]` lines in the old flat `[TBD] ...` format instead of ~108 in the
  current `[TBD][Subsystem] ...` format. `run-playtest-server.sh` hard-fails on this. To check any
  log by hand, count the **format**, never a sentence (T-608):

  ```bash
  grep -c '\[TBD\]\[' "$LOG"     # current checkout: >100.  Stale Workshop 1.0.1: exactly 0.
  ```

  The stale build has no subsystem tag at all, so this holds no matter how the individual
  `Print`s are worded. Pinning the check to one quoted sentence is what broke
  [`STAGING-SERVER.md:48`](../mod/STAGING-SERVER.md): T-604 quoted the settle line, T-605 rewrote
  that `Print` in the same wave, and for a week the stale-build detector told operators on the
  *correct* build that their expected string was missing. Match prefixes, not sentences.
- **On the friend's client, always:** it resolves `game.mods[]` from the Workshop and gets 1.0.1
  while the server runs your checkout. **Unverified — no second machine has connected to this
  combination.** It may refuse on a content mismatch, or it may join clean and run old script.

**First thing to check when the friend connects:** have them type `#tbd` in chat. The current
build answers with the full command list; 1.0.1 does not have that command. If they get nothing
and you get the list, you have the skew — **re-publish `tbd-framework` from Workbench**
([`STAGING-SERVER.md` §Dev loop](../mod/STAGING-SERVER.md)) so the Workshop copy matches the
checkout, and restart. Same test doubles as §6.4's first probe.

### 6.3 Radio automatic tuning cannot work on this world

`worlds/TBD_Dev_POC.ent` is a **62-byte bare SubScene** of vanilla `Eden.ent` and places nothing —
in particular no `RadioManagerEntity`, which the engine announces on every boot as
`DEFAULT (W): World doesn't contain RadioManagerEntity to support any BaseRadioComponent.`
Net **assignment and display still work**; automatic **tuning** does not, and the mod says so
honestly rather than pretending
([`TBD_RadioComponent.c:135`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioComponent.c)).
The fix is a Workbench world edit, not a code change
([`t181_event_mod_program.md:177-188`](../mod/t181_event_mod_program.md)). **Dial frequencies in by
hand.** Do not file this.

### 6.4 `modded class SCR_PlayerController` runtime coexistence has never been observed

Six blocks now exist (`TBD_MissionBrowser`, `TBD_BriefingController`, `TBD_LobbyController`,
`TBD_SpectatorHost`, `TBD_MarkerController`, `TBD_RadioController`). They compile and cross-call
fine at N=6, but **no gate can see runtime coexistence** — `world-boot.sh` boots with zero players
and every one of these only does anything with a client connected
([`t181_event_mod_program.md:508-525`](../mod/t181_event_mod_program.md)). **This session is the
first observation.** If one screen works and another silently does nothing, this is the first thing
to suspect — say so in the finding.

The same file records that whether four separate `modded enum ChimeraMenuPreset` blocks produce
distinct values is unprovable from the compile lane (duplicate `switch` case labels compile clean).
If two different screens open the *same* menu, that is what you are looking at.

### 6.5 Spectator and admin **keybinds** may not bind

The five spectator `.conf` action files and `TBD_AdminMenu.conf` do now carry `.meta` sidecars in
this tree, but the "resource not registered: Setting null GUID" class of failure was live as
recently as T-181's landmine list. If `F`/`←`/`→`/`TAB`/`V`/`F8` do nothing, check
`grep -i "Setting null GUID" "$LOG"`. Chat (`#tbd`) is the surface that has always worked and is not
gated on any of this ([`TBD_AdminCommands.c:23-25`](../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c)).

### 6.6 `#tbd link <code>` leaks the code into chat (T-327, deferred)

`super.OnNewMessage()` distributes the message before TBD can see it, so suppression is impossible
from script. Codes are single-use and consumed on success, and are never logged. Cosmetic, but do
not be alarmed. `executor: workbench`; the real fix is an in-game UI field.

### 6.7 Two more, worth knowing but unlikely to surface

- **`SCR_BaseGameMode.OnPlayerDisconnected` deletes the disconnecting player's controlled entity** —
  i.e. our materialised slot body. `SCR_ReconnectComponent` would reserve it, but the re-apply hangs
  off a join path TBD swallows. If S14's reconnect leaves a hole in the world where a body was, this
  is why ([`t181_event_mod_program.md:309-312`](../mod/t181_event_mod_program.md)).
- **`ScriptCallQueue.Remove` cancels by FUNCTION, not by arguments**, and dedicated-server player ids
  are recycled. Deferred per-player callbacks carry a connection epoch to stop a fresh joiner being
  deployed into a dead player's slot. If a joiner lands in someone else's seat, that epoch stamp is
  what failed.

### 6.8 Deferred tickets that will *not* bite

Checked against all 44 `deferred` rows. The mod-adjacent ones are all either data-completeness work
with no runtime path (**T-205** vehicle seats/crew/turrets, **T-206** item metadata — both
`executor: workbench`, both blocked on a schema change), documentation drift (**T-404**), or
website-only (**T-069** markers-in-editor, **T-082**, **T-084**, **T-086**/**T-088** server-control
UI, **T-094**, **T-096**, **T-121**, **T-370**/**T-388**, **T-571**, **T-573**–**T-575**,
**T-578**, **T-579**, **T-588**). None of them changes what the two clients see. **T-069 markers
being deferred refers to the *editor* surface** — the mod's own marker service is shipped and is
what S11 tests.

---

## Appendix — the log lines, in one place

Copy this into a second terminal:

```bash
tail -f "$LOG" | grep --line-buffered -E \
 '\[TBD\] roll-call|\[TBD\]\[Mission\]|\[TBD\]\[Validate\]|\[TBD\]\[Slots\]|\[TBD\]\[Loadout\]|\[TBD\]\[Spawn\]|\[TBD\]\[Stage\]|\[TBD\] Stage|\[TBD\]\[Admin\]|\[TBD\]\[JIP\]|\[TBD\]\[Lobby\]|Unknown class|Menu preset'
```

| Line | Source | Means |
|---|---|---|
| `[TBD] roll-call: …=ok` ×9 | `TBD_FrameworkManager.c:409` | every component on `TBD_GameMode.et` instantiated |
| `[TBD][Mission] loaded id=… source=backend` | `TBD_Log.c:75` | the API served the document (not the disk fallback) |
| `[TBD][Validate] mission result=PASS` | `TBD_Log.c:85` | the mission is loadable |
| `[TBD][Slots] loadout settle complete … 0 unplayable, M with a shortfall — spawn open` | `TBD_SpawnManager.TickLoadoutSettle` | the lineup is playable; `M`>0 means some of it is not what was authored |
| `[TBD][Slots] loadout SHORTFALL on M of N slot(s)` | `TBD_SpawnManager.TickLoadoutSettle` | **not** a stop — names the slots carrying less/elsewhere than authored (§6.1) |
| `[TBD][Slots] loadout delivery REFUSED at spawn boundary … UNPLAYABLE` | `TBD_SpawnManager.TickLoadoutSettle` | **stop** — nobody can leave LOADING; the line names the slot and the item |
| `[TBD] Stage → LOBBY` | `TBD_FrameworkManager.c:794` | the lobby is open |
| `[TBD][Spawn] LOBBY: auto-deploy wave OFF` | `TBD_SpawnManager.c:2094` | the picker is the way in (correct) |
| `[TBD][Spawn] claim player=N slot=K` | `TBD_SpawnManager.c:785` | a seat was taken |
| `[TBD][Spawn] claim rejected … (held by another player)` | `TBD_SpawnManager.c:780` | contention refused correctly |
| `[TBD][Spawn] claim rejected … (one life spent)` | `TBD_SpawnManager.c:773` | one life is enforced |
| `[TBD] SpawnManager: assigned slot … to player …` | `TBD_SpawnManager.c:666` | seated |
| `[TBD][Spawn] slot=… Y=… jsonY=… heading=…` | `TBD_SpawnManager.c:1161` | authored transform applied |
| `[TBD][Loadout][Slot] … loadout pass complete gear=x/x cargo=y/y` | `TBD_LoadoutEquipHelper.c:1386` | **the T-068 line** |
| `[TBD][Loadout][Slot] … worn-audit jacket=1 pants=1 boots=1` | `TBD_LoadoutEquipHelper.c:1342` | actually dressed, not just "equip OK" |
| `[TBD][Loadout][Slot] … NAKED …` / `HALF-DRESSED …` | `TBD_LoadoutEquipHelper.c:1223` / `:1228` | the nakedness guard fired |
| `[TBD] SpawnManager: bound player … to slot … body (kit …)` | `TBD_SpawnManager.c:2381` | body handed over |
| `[TBD][Spawn] player=N possess request accepted` | `TBD_SpawnManager.c:2549` | in the world |
| `[TBD][Spawn] player=N KILLED — one life spent` | `TBD_SpawnManager.c:2710` | terminal death |
| `[TBD][Admin] respawn player=N by=… — back in the world, life restored` | `TBD_SpawnManager.c:2878` | admin respawn worked |
| `[TBD][Slots] rematerialized body for slot … — freshly dressed from mission JSON` | `TBD_SpawnManager.c:2280` | respawn re-applied the loadout |
| `WORLD (E): Unknown class '<Name>'` | engine | a prefab component's class does not resolve |
| `GUI (E): Menu preset '<name>' not found!` | engine | a screen cannot open |
