use super::cli::VerifyCmd;
use crate::core::repository_root::find_repo_root;
use crate::*;
use anyhow::Result;

pub(crate) fn run(cmd: VerifyCmd) -> Result<u8> {
    {
        let code = match cmd {
            VerifyCmd::FileLength => {
                crate::verifications::language_bans::node_and_file_limits::verify_file_length()?
            }
            VerifyCmd::BlasManifest => {
                verifications::map_assets::verify_blas_manifest(&find_repo_root()?)?
            }
            VerifyCmd::NoNode => {
                crate::verifications::language_bans::node_and_file_limits::verify_no_node()?
            }
            VerifyCmd::NoShell => {
                crate::verifications::language_bans::shell_scripts::verify_no_shell()?
            }
            VerifyCmd::CiShell => crate::verifications::ci::workflow_shell::verify_ci_shell()?,
            VerifyCmd::NoSelectStar => {
                crate::verifications::database::sql_deserialization::verify_no_select_star(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T452 => {
                crate::verifications::mod_scripts::player_identity_comments::verify_t452(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T296 => {
                crate::verifications::mod_scripts::results_reporter_identity_comments::verify_t296(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T439 => {
                crate::verifications::registry::object_registry_aliases::verify_t439(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T444 => {
                crate::verifications::database::wiki_seeds::verify_t444(&find_repo_root()?)?
            }
            VerifyCmd::NoCrfLeak => {
                crate::verifications::licensing::upstream_code_leaks::verify_crf_leak(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T180 => {
                crate::verifications::architecture::editor_orbat_coherency::verify_t180(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::RouteTags => {
                crate::verifications::architecture::route_tags::verify_route_tags(
                    &find_repo_root()?
                )?
            }
            VerifyCmd::UiLayouts => {
                crate::verifications::mod_scripts::ui_layouts::verify_ui_layouts(&find_repo_root()?)?
            }
            VerifyCmd::T437 => {
                crate::verifications::mod_scripts::destroy_target_diagnostics::verify_t437(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T438 => {
                crate::verifications::deployment::staging_compose_paths::verify_t438(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T440 => crate::verifications::database::faction_library_seeds::verify_t440(
                &find_repo_root()?,
            )?,
            VerifyCmd::NoPython => {
                crate::verifications::language_bans::python_scripts::verify_no_python()?
            }
            VerifyCmd::T456 => {
                crate::verifications::mod_scripts::mission_rest_size_limits::verify_t456(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::T468 => {
                crate::verifications::ci::schema_parity::verify_t468(&find_repo_root()?)?
            }
            VerifyCmd::EngineLayers => {
                crate::verifications::architecture::engine_layer_boundaries::verify_engine_layers(
                    &find_repo_root()?,
                )?
            }
        };
        Ok(code)
    }
}
