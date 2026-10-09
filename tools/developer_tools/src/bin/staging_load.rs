//! The `staging-load` binary: the staging member load, a plan in and its report out.

fn main() -> std::process::ExitCode {
    staging_load_generator::entrypoint()
}
