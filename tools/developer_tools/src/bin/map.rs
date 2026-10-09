//! The `map` binary: satellite, cartographic, label, water and glyph map assets.

fn main() -> std::process::ExitCode {
    map_raster_pipeline::entrypoint()
}
