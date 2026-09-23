use super::cli::VerifyCmd;
use crate::core::repository_root::find_repo_root;
use crate::*;
use anyhow::Result;

pub(crate) fn run(cmd: VerifyCmd) -> Result<u8> {
    {
        let code = match cmd {
            VerifyCmd::ApiReadiness { evidence, execute } => {
                crate::verifications::api_readiness::verify(&find_repo_root()?, &evidence, execute)?
            }
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
            VerifyCmd::PlayerIdentityComments => {
                crate::verifications::mod_scripts::player_identity_comments::verify_player_identity_comments(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::ResultsReporterIdentityComments => {
                crate::verifications::mod_scripts::results_reporter_identity_comments::verify_results_reporter_identity_comments(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::ObjectRegistryAliases => {
                crate::verifications::registry::object_registry_aliases::verify_object_registry_aliases(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::WikiSeeds => {
                crate::verifications::database::wiki_seeds::verify_wiki_seeds(&find_repo_root()?)?
            }
            VerifyCmd::NoCrfLeak => {
                crate::verifications::licensing::upstream_code_leaks::verify_crf_leak(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::EditorOrbatCoherency => {
                crate::verifications::architecture::editor_orbat_coherency::verify_editor_orbat_coherency(
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
            VerifyCmd::DestroyTargetDiagnostics => {
                crate::verifications::mod_scripts::destroy_target_diagnostics::verify_destroy_target_diagnostics(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::StagingComposePaths => {
                crate::verifications::deployment::staging_compose_paths::verify_staging_compose_paths(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::FactionLibrarySeeds => crate::verifications::database::faction_library_seeds::verify_faction_library_seeds(
                &find_repo_root()?,
            )?,
            VerifyCmd::NoPython => {
                crate::verifications::language_bans::python_scripts::verify_no_python()?
            }
            VerifyCmd::MissionRestSizeLimits => {
                crate::verifications::mod_scripts::mission_rest_size_limits::verify_mission_rest_size_limits(
                    &find_repo_root()?,
                )?
            }
            VerifyCmd::CiSchemaParity => {
                crate::verifications::ci::schema_parity::verify_ci_schema_parity(&find_repo_root()?)?
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
