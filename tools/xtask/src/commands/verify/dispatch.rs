use super::cli::{DocumentationGateArgs, VerifyCmd};
use anyhow::Result;
use documentation_checks::{GateRequest, UntrackedFiles};
use repository_checks::architecture::workspace_laws;
use repository_layout::prelude::find_repository_root;

pub(crate) fn run(cmd: VerifyCmd) -> Result<u8> {
    {
        let code = match cmd {
            VerifyCmd::ApiReadiness { evidence, execute } => {
                api_readiness_checks::verify(&find_repository_root()?, &evidence, execute)?
            }
            VerifyCmd::FileLength => {
                repository_checks::language_bans::node_and_file_limits::verify_file_length()?
            }
            VerifyCmd::BlasManifest => {
                map_asset_verification::blas_manifest::verify_blas_manifest(&find_repository_root()?)?
            }
            VerifyCmd::NoNode => {
                repository_checks::language_bans::node_and_file_limits::verify_no_node()?
            }
            VerifyCmd::NoShell => {
                repository_checks::language_bans::shell_scripts::verify_no_shell()?
            }
            VerifyCmd::CiShell => ci_task_catalog::workflow_checks::workflow_shell::verify_ci_shell()?,
            VerifyCmd::NoSelectStar => {
                database_operations::database_checks::sql_deserialization::verify_no_select_star(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::PlayerIdentityComments => {
                mod_script_checks::player_identity_comments::verify_player_identity_comments(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::ResultsReporterIdentityComments => {
                mod_script_checks::results_reporter_identity_comments::verify_results_reporter_identity_comments(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::ObjectRegistryAliases => {
                repository_checks::registry::object_registry_aliases::verify_object_registry_aliases(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::WikiSeeds => {
                database_operations::database_checks::wiki_seeds::verify_wiki_seeds(&find_repository_root()?)?
            }
            VerifyCmd::NoCrfLeak => {
                repository_checks::licensing::upstream_code_leaks::verify_crf_leak(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::EditorOrbatCoherency => {
                repository_checks::architecture::editor_orbat_coherency::verify_editor_orbat_coherency(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::RouteTags => {
                repository_checks::architecture::route_tags::verify_route_tags(
                    &find_repository_root()?
                )?
            }
            VerifyCmd::UiLayouts => {
                mod_script_checks::ui_layouts::verify_ui_layouts(&find_repository_root()?)?
            }
            VerifyCmd::DestroyTargetDiagnostics => {
                mod_script_checks::destroy_target_diagnostics::verify_destroy_target_diagnostics(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::EnfusionComments { paths } => {
                mod_script_checks::enfusion_comments::verify_enfusion_comments(
                    &find_repository_root()?,
                    &paths,
                )
            }
            VerifyCmd::StagingComposePaths => {
                deployment::deployment_checks::staging_compose_paths::verify_staging_compose_paths(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::FactionLibrarySeeds => database_operations::database_checks::faction_library_seeds::verify_faction_library_seeds(
                &find_repository_root()?,
            )?,
            VerifyCmd::NoPython => {
                repository_checks::language_bans::python_scripts::verify_no_python()?
            }
            VerifyCmd::MissionRestSizeLimits => {
                mod_script_checks::mission_rest_size_limits::verify_mission_rest_size_limits(
                    &find_repository_root()?,
                )?
            }
            VerifyCmd::CiSchemaParity => {
                ci_task_catalog::workflow_checks::schema_parity::verify_ci_schema_parity(&find_repository_root()?)?
            }
            VerifyCmd::CrateTiers => workspace_laws::verify_crate_tiers()?,
            VerifyCmd::CrateAnatomy => workspace_laws::verify_crate_anatomy()?,
            VerifyCmd::TestFileReachability => workspace_laws::verify_test_file_reachability()?,
            VerifyCmd::FrontendLayering => workspace_laws::verify_frontend_layering()?,
            VerifyCmd::TailwindSources => workspace_laws::verify_tailwind_sources()?,
            VerifyCmd::ReadmeCoverage { arguments } => {
                documentation_checks::readme_coverage::verify_readme_coverage(
                    &find_repository_root()?,
                    &documentation_gate_request(arguments),
                )
            }
            VerifyCmd::MarkdownPlacement { arguments } => {
                documentation_checks::markdown_placement::verify_markdown_placement(
                    &find_repository_root()?,
                    &documentation_gate_request(arguments),
                )
            }
            VerifyCmd::LinkCheck { report, arguments } => {
                use documentation_checks::link_check::{
                    BreakListing, verify_link_check,
                };
                let listing = if report {
                    BreakListing::Every
                } else {
                    BreakListing::First
                };
                verify_link_check(
                    &find_repository_root()?,
                    &documentation_gate_request(arguments),
                    listing,
                    &crate::cli::command_vocabulary::documentation_command_vocabulary(),
                )
            }
        };
        Ok(code)
    }
}

/// The request every documentation gate judges, from the arguments every documentation verb
/// takes.
fn documentation_gate_request(arguments: DocumentationGateArgs) -> GateRequest {
    GateRequest {
        paths: arguments.paths,
        untracked: if arguments.with_untracked {
            UntrackedFiles::Included
        } else {
            UntrackedFiles::Invisible
        },
    }
}
