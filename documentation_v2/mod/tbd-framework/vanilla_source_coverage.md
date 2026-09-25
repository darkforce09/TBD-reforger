**Status:** live

# Vanilla source coverage

Where the vanilla Arma Reforger script source that the `enf` oracle indexes comes from: four lanes,
what each one reaches, and which to use for a question. Read it before assuming a vanilla class is
greppable. Every lane writes under `apps/mod/vanilla_reference/`, which is gitignored (Bohemia's
copyrighted source is never committed); only the derived `.ai/artifacts/enf-index/vanilla_*.tsv`
indexes are.

## The lanes

| Lane | Command | Reaches | Gives |
|---|---|---|---|
| Source pages | `cargo xtask fetch vanilla-source`, then `enf source` | any class, one page per file | complete `.c` files **with method bodies** |
| Pak extract | `enf extract` | the scripts stored uncompressed in the paks, by real path | complete files |
| API docs | `enf apidoc`, `cargo xtask fetch vanilla-api <Class>…` | all 7,990 classes of the Script API | signatures and inheritance, no bodies |
| Byte carving | `enf carve` | plaintext script fragments in the paks | 610 fragment files |

`enf` is `cargo run -q -p developer-tools --bin enf --`. After any lane, rebuild the index:

```bash
cargo run -q -p developer-tools --bin enf -- index vanilla --root apps/mod/vanilla_reference
```

The committed index (`.ai/artifacts/enf-index/vanilla_files.tsv`) holds 2,146 files, 98,475
lines and 1,775 class declarations: 610 carved fragments, 47 reconstructed source files and the
extracted scripts.

### Source pages: the answer for bodies

A public Doxygen build of the same game version (1.7.0.54) at arexplorer.zeroy.com, with its source
browser enabled, publishes one
`*_source.html` page per script: 6,495 pages, exactly the pak's script count, each with the whole
file including method bodies. `cargo xtask fetch vanilla-source` mirrors a curated spine set
(`--grep <pattern>` adds matches) into `apps/mod/vanilla_reference/source_html/`, and `enf source`
rebuilds `.c` files from the cached pages into `apps/mod/vanilla_reference/Source/`. The spine
classes resolve there: `SCR_BaseGameMode`, `SCR_RespawnSystemComponent`, `ChimeraMenuBase`,
`SCR_SpawnRequestComponent`.

**Be a good citizen.** It is one person's site and a full mirror is gigabytes. The fetcher defaults
to the spine set, caches everything, never refetches and sleeps between requests. `--all` exists;
think before using it.

### Pak extract, and the compression barrier

`enf extract` reads scripts by name through the pak file table. The table lists every script
under a lowercase `scripts/` (`scripts/Game/GameMode/SCR_BaseGameMode.c`; `Scripts/` finds
nothing), but of the 6,495 entries only 2,483 are stored uncompressed and extract. The other 4,012
are compressed with a codec that is neither zlib nor raw deflate, and the entries' method bytes
are all zero, so nothing names it. A sample failing header reads `49 b6 e5 36 4d b7 c0 16`
(3,064 bytes that expand to 14,720). LZ4 block is the leading guess; confirming it needs a new
crate. The `enfusion-mcp` pak reader fails on the same entries, which is why `game_browse` lists
`SCR_BaseGameMode.c` while `game_read` answers `incorrect header check`. The source pages make this
barrier moot for reading source.

### API docs: signatures for every class

Bohemia publishes the complete Script API as Doxygen HTML. `enf apidoc` parses the class index
into `vanilla_api_classes.tsv` and `vanilla_api_members.tsv`; `cargo xtask fetch vanilla-api`
fetches single class pages. Measured member counts: `SCR_BaseGameMode` 145,
`SCR_RespawnSystemComponent` 49, `SCR_PossessSpawnRequestComponent` 35, `ChimeraMenuBase` 16,
`SCR_PossessSpawnData` 12, including the call the possess deploy rests on:

```text
SCR_PossessSpawnData    static SCR_PossessSpawnData FromEntity (notnull IEntity entity)
```

Fetch the class pages a slice needs, never all 7,990. The page cache is gitignored.

### Byte carving

`enf carve --game "<Arma Reforger install>" --out apps/mod/vanilla_reference` scans the 16 paks
for printable runs of at least 400 bytes and keeps the Enfusion script among them: 610 files,
1.0 MB, 41,958 lines, 1,106 declarations (882 of them `SCR_*`), 2,493 symbols. It takes about six
minutes. It reaches the AI and behaviour-tree, camera, action-condition, UI-component and
Workbench-plugin layers, not the spawn pipeline: `SCR_BaseGameMode`, `SCR_PossessSpawnData`,
`SCR_PossessSpawnRequestComponent`, `SCR_RespawnSystemComponent` and `ChimeraMenuBase` sit in
compressed blocks (the plaintext `SCR_BaseGameMode` hits are `.et` prefab references). The
400-byte floor matters: `class SCR_AIDangerReaction` sits in a 1,312-byte run, and a 2 KB floor
keeps 127 files instead of 610.

## Which lane answers what

1. Behaviour of a vanilla class: its reconstructed source under `apps/mod/vanilla_reference/Source/`;
   fetch its page first when it is missing.
2. A signature or member list: `rg '^<Class>\t' .ai/artifacts/enf-index/vanilla_api_members.tsv`,
   or `cargo xtask mcp call api_search '{"query":"<Class>"}'`.
3. How a pipeline is driven in practice: CRF's usage, through `enf lookup <symbol>` on the CRF
   lane. `CRF_SCR_PossessSpawnHandlerComponent.c` is how the possess pipeline was found.
4. Component composition, GUIDs and default property values: `.et` prefab configs, which are
   plaintext in the paks even where the `.c` is compressed.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — §5 cites the facts these lanes
  proved
- [Enfusion tooling](/tools_v2/developer-tools/src/enfusion_tooling/README.md) — the `enf`
  subcommands
- [Enfusion pak reader](/tools_v2/developer-tools/src/enfusion_pak/README.md) — the pak file table
  and payload decoding the extract lane uses
