# Vanilla source coverage — four lanes (T-181.3 / .3.1 / .3.2 / .3.3)

## LANE 4 IS THE ANSWER — full source WITH BODIES (T-181.3.3)

**arexplorer.zeroy.com** is a Doxygen build of the same game version (1.7.0.54) with
SOURCE_BROWSER enabled: **6,495 `*_source.html` pages — exactly matching the pak script
count** — each containing the complete file *including method bodies*. This is strictly better
than every other lane and supersedes the codec problem entirely; cracking the pak compression is
no longer needed.

```bash
bash scripts/mod/fetch-vanilla-source.sh            # curated spine set
bash scripts/mod/fetch-vanilla-source.sh --grep Respawn
cargo run -q -p tbd-tools --bin enf -- source       # rebuild .c from cached pages
cargo run -q -p tbd-tools --bin enf -- index vanilla --root apps/mod/vanilla_reference
```

Every class that was previously unreachable now resolves to real source:

| symbol | resolves to |
|---|---|
| `SCR_BaseGameMode` | `Source/SCR_BaseGameMode.c:133` |
| `SCR_RespawnSystemComponent` | `Source/SCR_RespawnSystemComponent.c:6` |
| `ChimeraMenuBase` | `Source/ChimeraMenuBase.c:69` |
| `SCR_SpawnRequestComponent` | `Source/SCR_SpawnRequestComponent.c:12` |

Combined vanilla index: **2,116 files · 88,670 LOC · 1,702 declarations**.

**BE A GOOD CITIZEN.** It is one person's site and a full mirror is gigabytes. The fetcher
defaults to a curated spine set, caches everything, never refetches, and sleeps between
requests. `--all` exists; think before using it.

---


**Current best:** `enf extract` (T-181.3.2) pulls vanilla scripts **by real path** out of the pak
file table — strictly better than carving where it works. Combined index today:
**2,099 files · 72,226 LOC · 1,659 declarations** (1,489 real-path + 610 carved blobs).

```bash
make enf-index                                            # CRF
cargo run -q -p tbd-tools --bin enf -- extract            # vanilla, by real path
cargo run -q -p tbd-tools --bin enf -- index vanilla --root apps/mod/vanilla_reference
make enf-apidoc                                           # 7,990 class signatures
```

## The pak compression barrier (measured, T-181.3.2)

The pak FILE table **does** list every script — `scripts/Game/GameMode/SCR_BaseGameMode.c`, 84 KB,
is right there (note the lowercase `scripts/`; `Scripts/` returns nothing). Of 6,495 script
entries:

| | count | why |
|---|---|---|
| extracted | **2,483** | stored **uncompressed** (`compressed=false`) |
| failed | **4,012** | `compressed=true`, and the payload is **neither zlib nor raw deflate** |

Fixing `PakVfs` to fall back from zlib to raw deflate was necessary but not sufficient. Sample
failing header: `49 b6 e5 36 4d b7 c0 16` (clen 3064 → dlen 14720, ~4.8x). The six entry
"method" bytes are **all zero**, so there is no codec flag to switch on — the codec is implicit.

`enfusion-mcp` cannot read these either: its pak reader only calls `inflateSync`, which is why
`game_browse` lists `SCR_BaseGameMode.c` but `game_read` returns `incorrect header check`. **This
is the single error that made the vanilla source look unavailable.**

**Follow-up (unstarted):** identify the codec — LZ4 block is the leading hypothesis from the
token-shaped first byte, but no lz4 tooling is installed and confirming it means adding a crate.
Doing so would unlock the remaining ~4,000 files including the whole gameplay layer.

---

## Legacy: byte-carving (T-181.3)


**Measured 2026-07-25. Read this before assuming a vanilla class is greppable.**

Regenerate: `cargo run -q -p tbd-tools --bin enf -- carve --game "$HOME/.local/share/Steam/steamapps/common/Arma Reforger" --out apps/mod/vanilla_reference` (~6 min, then `enf index vanilla`).

## What you get

| Measure | Value |
|---|---|
| paks scanned | 16 |
| printable runs ≥ 400 B | 9,976 |
| kept as Enfusion script | **610 files, 1.0 MB** |
| indexed LOC | 41,958 |
| declarations | **1,106** (882 of them `SCR_*`) |
| symbols (incl. methods) | 2,493 |

Query it:
```bash
cargo run -q -p tbd-tools --bin enf -- lookup SCR_AIDangerReaction \
  --index .ai/artifacts/enf-index/vanilla_symbols.tsv
rg 'class SCR_AIDecoTest' apps/mod/vanilla_reference/
```

## What it does NOT reach — and why

**Only a subset of vanilla script ships uncompressed.** The rest lives in compressed blocks of
the FORM/PAC1 archives and cannot be recovered by byte-scanning. Verified absent as plaintext:

| Class | Status | Notes |
|---|---|---|
| `SCR_BaseGameMode` | **ABSENT** | the 23 plaintext hits in `data007.pak` are `.et` *prefab config* references (`SCR_BaseGameMode : "{GUID}Prefabs/…"`), not a class declaration |
| `SCR_PossessSpawnData` | **ABSENT** | compressed |
| `SCR_PossessSpawnRequestComponent` | **ABSENT** | compressed |
| `SCR_RespawnSystemComponent` | **ABSENT** | compressed |
| `ChimeraMenuBase` | **ABSENT** | compressed |

That is the honest result for *carving*: it delivers the AI/behaviour-tree, camera,
action-condition, UI-component and Workbench-plugin layers, but not the spawn pipeline.

## RESOLVED — the official API docs cover every one of them (T-181.3.1)

Operator's suggestion ("it might exist already online to download") was correct. Bohemia
publishes the complete Script API as Doxygen HTML: **7,990 classes**, and all five "absent"
classes are there with full member lists.

```bash
make enf-apidoc          # fetch the class index + parse (1 request for the index)
cargo xtask fetch vanilla-api SCR_BaseGameMode SCR_PossessSpawnData   # per-class pages
rg '^SCR_PossessSpawnData\t' .ai/artifacts/enf-index/vanilla_api_members.tsv
```

Measured members recovered: `SCR_BaseGameMode` **145**, `SCR_RespawnSystemComponent` **49**,
`SCR_PossessSpawnRequestComponent` **35**, `ChimeraMenuBase` **16**, `SCR_PossessSpawnData` **12**
— including the exact call the handoff fought four Workbench restarts to find:

    SCR_PossessSpawnData    static SCR_PossessSpawnData FromEntity (notnull IEntity entity)

**Signatures and inheritance, no bodies** — same class of information as `api_search`, but bulk,
offline and greppable. For *behaviour* (what the body actually does), CRF remains the oracle.

Do not bulk-fetch all 7,990 class pages; pull the ones a slice needs. The cache is gitignored;
only the derived `vanilla_api_*.tsv` is committed.

The earlier claim that "33.9 MB / 1,859 class declarations" were carveable from `data007.pak`
conflated two things: a large share of that printable volume is `.et` prefab config, not script.

**Fallbacks for anything listed ABSENT, in order:**
1. `bash scripts/mod/mcp-call.sh api_search '{"query":"SCR_PossessSpawnData"}'` — signatures only, no bodies.
2. CRF's own usage as the behavioural oracle — e.g.
   `CRF_SCR_PossessSpawnHandlerComponent.c` is exactly how the possess pipeline was found.
   `cargo run -q -p tbd-tools --bin enf -- lookup <symbol>` (CRF lane).
3. `.et` prefab config *is* plaintext and greppable in the paks — useful for component
   composition, GUIDs, and default property values even when the `.c` is not available.

## Known follow-up

`tools/tbd-tools/src/world/pak.rs` (`PakVfs`) already parses FORM/PAC1 **with zlib inflate** for
named files. Scripts are not name-addressable in the FILE tree, but inflating every compressed
block wholesale and re-running the carver over the inflated bytes is the plausible route to the
remaining corpus. Not attempted in T-181.3 — filed as the next step if the spawn/menu sources
are needed badly enough.

## Sizing note

Blob floor is **400 bytes**, not 2 KB: `class SCR_AIDangerReaction` sits in a 1,312-byte run,
and a 2 KB floor silently dropped ~2/3 of real script (127 files vs 610). Vanilla ships many
small fragments, not a few large ones.
