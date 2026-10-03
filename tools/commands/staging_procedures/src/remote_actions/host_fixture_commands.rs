//! The `staging-fixtures` host tool's command lines.
//!
//! **Role:** builds each `staging-fixtures` subcommand the harness runs on the host: fleet
//! provisioning, credential rotation, the load population and fixture events, membership aging,
//! and the bot's member reads and bucket spend.
//!
//! **Position:** called by `staging_dispatch.rs` for the confirmed actions, by the procedures' host
//! action steps, and by `remote_observers/discord_member_reader.rs`; the tool itself lives in
//! `tools/staging/staging_fixtures/src/` and is built in the host checkout.
//!
//! **Signals & state:** none; pure builders.
//!
//! **Invariants:** the tool runs from the host checkout root, where its API env file default
//! resolves; every invocation names `--confirm-database tbd_reforger` and `--apply` (the tool
//! otherwise only prints its plan), and its [`CommandPurpose`] says whether it changes the host;
//! every argument is one single-quoted shell word, except a bucket spend's start named by a host
//! script variable ([`SpendStart::HostVariable`]), which is one double-quoted expansion of a
//! shell identifier; no secret is an argument (the tool reads the bot token and writes
//! credentials on the host).

use crate::error::{Result, ensure};

use crate::remote_observers::remote_command::{
    CommandPurpose, RemoteCommand, shell_quote, shell_words,
};
use crate::staging_settings::{STAGING_DATABASE, StagingSettings};

/// The tool's path under the host checkout.
pub(crate) const HOST_TOOL: &str = "target/release/staging-fixtures";

/// Which executor's machine credential a rotation replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CredentialExecutor {
    /// The host agent's credential.
    #[value(name = "host_agent")]
    HostAgent,
    /// The game runtime's credential.
    #[value(name = "mod_runtime")]
    ModRuntime,
}

impl CredentialExecutor {
    /// The tool's spelling.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::HostAgent => "host_agent",
            Self::ModRuntime => "mod_runtime",
        }
    }
}

/// The half of a rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RotationStage {
    /// Issue the new credential into the `.staged` file.
    Stage,
    /// Move the staged credential over the live one.
    Promote,
}

/// Which guild a member read asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuildScope {
    Main,
    Partner,
}

/// `staging-fixtures <subcommand> <arguments> --confirm-database tbd_reforger --apply`, run from
/// the host checkout.
pub(crate) fn invocation(
    settings: &StagingSettings,
    subcommand: &str,
    arguments: &[String],
    purpose: CommandPurpose,
) -> RemoteCommand {
    invocation_of_words(settings, subcommand, &shell_words(arguments), purpose)
}

/// [`invocation`] over arguments already written as shell words.
fn invocation_of_words(
    settings: &StagingSettings,
    subcommand: &str,
    argument_words: &str,
    purpose: CommandPurpose,
) -> RemoteCommand {
    let mut command_line = format!(
        "cd {} && {}",
        shell_quote(&settings.checkout),
        shell_words(&[format!("./{HOST_TOOL}"), subcommand.to_string()])
    );
    if !argument_words.is_empty() {
        command_line.push(' ');
        command_line.push_str(argument_words);
    }
    command_line.push(' ');
    command_line.push_str(&shell_words(&[
        "--confirm-database",
        STAGING_DATABASE,
        "--apply",
    ]));
    RemoteCommand {
        observer: "host tool",
        purpose,
        command_line,
        stdin: None,
    }
}

/// Registers the fleet's servers and writes their credentials under the fleet root.
pub(crate) fn provision_fleet(settings: &StagingSettings) -> Result<RemoteCommand> {
    let address = settings.server_address()?;
    let arguments = vec![
        "--instances".to_string(),
        settings.fleet.instance_count.to_string(),
        "--actor".into(),
        settings.operator()?.to_string(),
        "--ip".into(),
        address,
        "--secrets-root".into(),
        settings.fleet_root(),
        "--game-port-base".into(),
        settings.fleet.game_port_base.to_string(),
    ];
    Ok(invocation(
        settings,
        "provision-fleet",
        &arguments,
        CommandPurpose::Change,
    ))
}

/// One half of the rotation of `instance`'s `executor` credential.
pub(crate) fn rotate_credential(
    settings: &StagingSettings,
    instance: u16,
    executor: CredentialExecutor,
    stage: RotationStage,
) -> Result<RemoteCommand> {
    ensure!(
        (1..=settings.fleet.instance_count).contains(&instance),
        "instance {instance} is not one of the fleet's 1..={}",
        settings.fleet.instance_count
    );
    let mut arguments = vec![
        "--instance".to_string(),
        instance.to_string(),
        "--executor".into(),
        executor.as_str().into(),
        "--secrets-root".into(),
        settings.fleet_root(),
    ];
    match stage {
        RotationStage::Stage => {
            arguments.extend(["--stage".into(), "--actor".into()]);
            arguments.push(settings.operator()?.to_string());
        }
        RotationStage::Promote => arguments.push("--promote".into()),
    }
    Ok(invocation(
        settings,
        "rotate-credential",
        &arguments,
        CommandPurpose::Change,
    ))
}

/// What seeding the load fixtures names: the population and the mission its events attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoadSeeding<'a> {
    /// Synthetic accounts to create.
    pub(crate) accounts: u32,
    /// The Discord id of the first account.
    pub(crate) id_base: &'a str,
    /// The Discord role name every account holds.
    pub(crate) discord_role: &'a str,
    /// Where the tool writes the accounts' refresh tokens on the host.
    pub(crate) account_file: &'a str,
    /// The live mission the fixture events attach.
    pub(crate) mission_id: &'a str,
}

/// Seeds the synthetic population and then the fixture events (`Some`), or cleans the fixture
/// events and then the population (`None`): the order the foreign keys fix.
pub(crate) fn load_fixtures(
    settings: &StagingSettings,
    seeding: Option<&LoadSeeding<'_>>,
) -> Vec<RemoteCommand> {
    let change = |subcommand: &str, arguments: Vec<String>| {
        invocation(settings, subcommand, &arguments, CommandPurpose::Change)
    };
    match seeding {
        Some(seeding) => vec![
            change(
                "seed-load-population",
                vec![
                    "--accounts".to_string(),
                    seeding.accounts.to_string(),
                    "--role".into(),
                    seeding.discord_role.into(),
                    "--account-file".into(),
                    seeding.account_file.into(),
                    "--id-base".into(),
                    seeding.id_base.into(),
                ],
            ),
            change(
                "seed-load-fixture-events",
                vec!["--mission".to_string(), seeding.mission_id.into()],
            ),
        ],
        None => vec![
            change("clean-load-fixture-events", Vec::new()),
            change("clean-load-population", Vec::new()),
        ],
    }
}

/// Ages `discord_id`'s membership snapshot by `hours`: a staged precondition.
pub(crate) fn age_membership_snapshot(
    settings: &StagingSettings,
    discord_id: &str,
    hours: u32,
) -> RemoteCommand {
    let arguments = vec![
        "--discord-id".to_string(),
        discord_id.to_string(),
        "--hours".into(),
        hours.to_string(),
    ];
    invocation(
        settings,
        "age-membership-snapshot",
        &arguments,
        CommandPurpose::Change,
    )
}

/// The tool's target flags for the operator's member record in `guild`: `--discord-id <operator>
/// --guild main|partner [--partner-guild-id <partner guild>]`. A key `deploy.env` lacks leaves its
/// flag out, so the tool refuses the command naming that flag and never reads another member.
pub(crate) fn operator_member_target(settings: &StagingSettings, guild: GuildScope) -> Vec<String> {
    let mut arguments = Vec::new();
    if let Some(operator) = &settings.operator_discord_id {
        arguments.extend(["--discord-id".to_string(), operator.clone()]);
    }
    arguments.push("--guild".into());
    match guild {
        GuildScope::Main => arguments.push("main".into()),
        GuildScope::Partner => {
            arguments.push("partner".into());
            if let Some(partner) = &settings.partner_guild_id {
                arguments.extend(["--partner-guild-id".to_string(), partner.clone()]);
            }
        }
    }
    arguments
}

/// The bot's read of the operator's member record in `guild`.
pub(crate) fn observe_discord_member(
    settings: &StagingSettings,
    guild: GuildScope,
) -> RemoteCommand {
    invocation(
        settings,
        "observe-discord-member",
        &operator_member_target(settings, guild),
        CommandPurpose::Read,
    )
}

/// When a bucket spend starts, as `--start-at-unix-ms` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpendStart {
    /// A Unix time in milliseconds, known when the command is built.
    UnixMillis(u64),
    /// A variable of the host script the command runs in, holding the Unix time in milliseconds
    /// the script computed first; the host's shell expands it.
    HostVariable(&'static str),
}

impl From<u64> for SpendStart {
    fn from(unix_ms: u64) -> Self {
        Self::UnixMillis(unix_ms)
    }
}

impl SpendStart {
    /// The start as one shell word: the quoted time, or the quoted expansion of the variable,
    /// which must be a shell identifier.
    fn shell_word(self) -> Result<String> {
        match self {
            Self::UnixMillis(unix_ms) => Ok(shell_quote(&unix_ms.to_string())),
            Self::HostVariable(name) => {
                let mut characters = name.chars();
                ensure!(
                    characters
                        .next()
                        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
                        && characters.all(|rest| rest == '_' || rest.is_ascii_alphanumeric()),
                    "`{name}` is not a shell variable name"
                );
                Ok(format!("\"${name}\""))
            }
        }
    }
}

/// Spends the Get Guild Member bucket from `start`, holding `hold_seconds`, with at most
/// `max_requests` (at most 50) requests.
pub(crate) fn spend_discord_member_bucket(
    settings: &StagingSettings,
    start: impl Into<SpendStart>,
    hold_seconds: u32,
    max_requests: u32,
) -> Result<RemoteCommand> {
    ensure!(
        (1..=50).contains(&max_requests),
        "a bucket spend makes 1 to 50 requests, not {max_requests}"
    );
    let target = shell_words(&operator_member_target(settings, GuildScope::Main));
    let limits = shell_words(&[
        "--hold-seconds".to_string(),
        hold_seconds.to_string(),
        "--max-requests".into(),
        max_requests.to_string(),
    ]);
    let arguments = format!(
        "{target} {} {} {limits}",
        shell_quote("--start-at-unix-ms"),
        start.into().shell_word()?
    );
    Ok(invocation_of_words(
        settings,
        "spend-discord-member-bucket",
        &arguments,
        CommandPurpose::Change,
    ))
}
