use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum SchemaCmd {
    /// Contract codegen: JSON Schema → Rust via typify
    Codegen,
    /// Print the `schema-validate` sub-gate SET, one per line, derived from the code that runs
    /// them, so the wave drift tripwire compares against the live gate list.
    #[command(name = "list-gates")]
    ListGates,
    /// Full contract-validation suite
    Validate,
    /// Validate one mission JSON file or stdin (`-`)
    #[command(name = "validate-file")]
    ValidateFile { target: String },
    /// @contract citation integrity
    Citations,
    /// Specification-consistency gates 1-12 over the Mission Creator specification corpus
    #[command(name = "specification-consistency")]
    SpecificationConsistency,
    /// N6 building-geometry sentence single-source
    N6,
    /// N10 tile-budget single-source
    N10,
    /// Semantic golden gates S2-S9 + S11-S15
    #[command(name = "map-object-golden")]
    MapObjectGolden,
    /// Height-label gates G2-G6 + ASL oracle
    #[command(name = "height-labels")]
    HeightLabels {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// DEM vs GetSurfaceY anchor alignment
    #[command(name = "terrain-alignment")]
    TerrainAlignment {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long)]
        strict: bool,
    },
    /// Locations gates G2-G7
    Locations {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Town-label gates over core importance_declutter
    #[command(name = "town-labels")]
    TownLabels {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long, default_value_t = -2.0, allow_hyphen_values = true)]
        zoom: f64,
    },
    /// Road-name gates over core road_labels
    #[command(name = "road-names")]
    RoadNames {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long, default_value_t = 0.0)]
        zoom: f64,
    },
    /// Glyph coverage gate GL-G1..G6
    #[command(name = "map-glyphs")]
    MapGlyphs,
    /// map-object enum single-source (GAP-M5)
    #[command(name = "map-object-enums")]
    MapObjectEnums,
    /// type-inventory invariants I1-I7
    #[command(name = "type-inventory")]
    TypeInventory,
    /// terrain manifest schema + terrains contract cross-check
    #[command(name = "terrain-manifest")]
    TerrainManifest {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Flatten mission ORBAT roles into slots[] (tool)
    #[command(name = "flatten-orbat-slots")]
    FlattenOrbatSlots {
        path: String,
        #[arg(long)]
        in_place: bool,
    },
}
