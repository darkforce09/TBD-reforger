use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum MapCmd {
    /// Classify staged Workbench export for TERRAIN / PHASE (T-869).
    /// Args mirror `export-terrain.sh` (unknown tokens → rc=1; missing raw → rc=2).
    #[command(name = "export-terrain", disable_help_flag = true)]
    ExportTerrain {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Building-blueprint ingest: profile TBD_Export → assets_v2/terrains, serde-validated
    /// against the BuildingBlueprint contract ([--src <dir>] [--filter <substr>]).
    #[command(name = "ingest-blueprints")]
    IngestBlueprints {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Replay the Workbench parity oracle through `evaluate_los` (BVH raycast over the `.bvh`
    /// sidecar + blueprint attribution) and report agreement
    /// (--pairs <parity.json> --blueprint <blueprint.json> --sidecar <file.bvh>).
    #[command(name = "parity-report")]
    ParityReport {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Interpret raw Workbench voxel dumps (action "dump") into BuildingBlueprint JSON —
    /// all extraction heuristics run offline here ([--filter <substr>] [--algo segments|grid]
    /// [--src <dir>] [--out <dir>] [--params <file.json>] [--debug-dir <dir>]).
    #[command(name = "blueprint-from-voxels")]
    BlueprintFromVoxels {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Generate a standard voxel dump from real triangle geometry (a Reforger .xob) by
    /// analytic ray-marching — same wire format as the Workbench sensor, no engine needed
    /// (--mesh <file.xob> --slug <s> [--geometry auto|coll|visual] [--coll-record <i>]
    /// [--out <dir>] [--resource <str>] [--lod <tier>] [--reference <dump.jsonl[.gz]>]
    /// [--axes x,y,-z] [--flip-winding] [--exclude-material <substr>]... [--stats]).
    /// Default geometry is the COLL fire-collision chunk when present — the exact surface
    /// engine LOS traces; visual LODS are the fallback.
    #[command(name = "voxels-from-mesh")]
    VoxelsFromMesh {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// One-number LOS parity proof: BVH any-hit raycast over the COLL fire-collision
    /// trimesh (both-sided, all records) or an emitted sidecar, replayed against the
    /// Workbench parity oracle ((--mesh <file.xob> | --sidecar <file.bvh>)
    /// --pairs <parity.json> [--record <i>] [--t-eps <meters>]
    /// [--dump-misses <path.jsonl>]).
    #[command(name = "bvh-parity")]
    BvhParity {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Emit the binary `.bvh` occlusion sidecar (COLL trimesh + BVH, deterministic bytes)
    /// next to the blueprint JSON in assets_v2/terrains/everon/prefabs/buildings/
    /// (--mesh <file.xob> --slug <slug> [--out <dir>]).
    #[command(name = "bvh-emit")]
    BvhEmit {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-090.11.2 — walk a building prefab's closure straight out of the game paks: the
    /// shell sidecar (v2, kinds from COLL game materials), one BLAS per child model under
    /// prefabs/blas/, and `<slug>.instances.json` (--prefab <Prefabs/…/X.et> [--slug <s>]
    /// [--out <dir>] [--paks <dir>] [--extract <dir>] [--scene <spec.json>]
    /// [--kind <record>=<kind>]… [--dry-run]). T-090.12.2: `--all-prefabs [--terrain everon]
    /// [--only-kind K]… [--limit N] [--hot N] [--dry-run]` walks every catalogue prefab into
    /// prefabs/descriptors/<pid>.json + the shared BLAS library + prefabs/blas-manifest.json.
    #[command(name = "bvh-batch")]
    BvhBatch {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-090.11.2 — print what the XOB decoder sees: string table, node records + sockets,
    /// COLL records with layer preset and per-material triangle runs, kinds histogram
    /// (<file.xob | in-pak path> [--paks <dir>] [--extract <dir>] [--strings]
    /// [--kind <record>=<kind>]…).
    #[command(name = "xob-inspect")]
    XobInspect {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Read one game-pak entry of any type (`<in-pak path> [--paks <dir>] [--head <bytes>]
    /// [--out <file>]`) — the peek / census tool for `.ent` worlds and `.et` prefabs.
    #[command(name = "pak-cat")]
    PakCat {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-090.11.3 — match every `xobSocket` instance against a Workbench recon dump
    /// (`--instances <slug>.instances.json --recon <slug>_children.json`); exit 1 on any
    /// mismatch over 2 cm / 1° or an unmatched instance. T-090.12.1: `--world-row --chunk
    /// <cx_cy.json.gz> --prefabs <prefabs.json.gz>` also places every matched child through the
    /// committed chunk row and compares with the recon worldPos (the wire-v2 transform pin).
    #[command(name = "instances-verify")]
    InstancesVerify {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-090.11.3 — rank the 48 Euler-composition hypotheses against a recon sample of a
    /// tilted parent with a rotated child (`--fixture <json>`); exit 1 unless
    /// `Rigid::from_enfusion` (Y·X·Z, positive signs) wins.
    #[command(name = "rotation-pin")]
    RotationPin {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-090.12.3 — the world occluder on the committed catalogue: `--cell <cx_cy> [--census]
    /// [--probe a b] [--bench N] [--pairs <world_parity.json>] [--glass-blocks]
    /// [--foliage-blocks] [--proxy-only] [--min-agree F] [--dump-misses <jsonl>]`.
    #[command(name = "world-los")]
    WorldLos {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
