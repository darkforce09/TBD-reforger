//! The `world` binary: the world-export pipeline and its verification gates.

fn main() -> std::process::ExitCode {
    world_export_pipeline::entrypoint()
}
