//! Test support shared by the harness's tests and the procedures' tests: a scripted host that
//! answers by command fragment and simulated time, the staging settings of a fictional host, and
//! a scratch run folder.
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Result, bail};

use crate::procedure_runner::fake_clock::FakeClock;
use crate::remote_observers::remote_command::{CommandOutput, HostCommandRunner, RemoteCommand};
use crate::staging_settings::StagingSettings;
use deploy_settings::DeployEnvironment;
use time_source::Clock as _;

/// The deploy settings of a fictional staging host with a five-instance fleet and its relay.
pub(crate) const TEST_DEPLOY_ENV: &str = "TBD_SSH_HOST=deploy@192.0.2.10\n\
TBD_SSH_PASS=ssh-password-canary\n\
TBD_BACKEND_URL=http://127.0.0.1:8080\n\
TBD_FLEET_INSTANCES=5\n\
TBD_FLEET_GAME_PORT_BASE=2000\n\
TBD_FLEET_RCON_PORT_BASE=19998\n\
TBD_FLEET_RELAY_INSTANCE=5\n\
TBD_FLEET_RELAY_PORT=18085\n\
TBD_STAGING_DB_CONTAINER=tbd_staging_db\n\
TBD_STAGING_OPERATOR_DISCORD_ID=123456789012345678\n\
TBD_STAGING_PARTNER_GUILD_ID=223456789012345678\n\
TBD_LOAD_SOURCE_ADDRESSES=192.0.2.117, 192.0.2.240\n";

/// [`TEST_DEPLOY_ENV`] as settings.
pub(crate) fn test_settings() -> StagingSettings {
    let environment = DeployEnvironment::from_text(
        std::path::Path::new("/deploy.env"),
        Some(TEST_DEPLOY_ENV),
        Vec::new(),
    )
    .unwrap();
    StagingSettings::from_environment(&environment).unwrap()
}

/// A fresh folder under the system temporary folder.
pub(crate) fn scratch_folder(purpose: &str) -> PathBuf {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let folder = std::env::temp_dir().join(format!(
        "xtask-staging-{purpose}-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// One recorded answer: from `from_unix_ms` on, a command whose command line or stdin holds
/// `fragment` answers `output`.
struct RecordedAnswer {
    fragment: String,
    from_unix_ms: u64,
    output: CommandOutput,
}

/// A host that answers from recordings, by the simulated time of a [`FakeClock`].
pub(crate) struct ScriptedHost {
    clock: FakeClock,
    answers: Vec<RecordedAnswer>,
    /// Every command it was asked to run, in order.
    pub calls: Vec<RemoteCommand>,
}

impl ScriptedHost {
    /// A host with no recordings yet.
    pub(crate) fn new(clock: &FakeClock) -> Self {
        Self {
            clock: clock.clone(),
            answers: Vec::new(),
            calls: Vec::new(),
        }
    }

    /// From `from_unix_ms` on, commands holding `fragment` exit `exit_code` with `stdout`; the
    /// latest-starting matching recording wins.
    pub(crate) fn answer(
        mut self,
        fragment: &str,
        from_unix_ms: u64,
        exit_code: i32,
        stdout: &str,
    ) -> Self {
        self.answers.push(RecordedAnswer {
            fragment: fragment.to_string(),
            from_unix_ms,
            output: CommandOutput {
                exit_code,
                stdout: stdout.to_string(),
                stderr: String::new(),
            },
        });
        self
    }
}

impl HostCommandRunner for ScriptedHost {
    fn run(&mut self, command: &RemoteCommand) -> Result<CommandOutput> {
        self.calls.push(command.clone());
        let now = self.clock.now_unix_ms();
        let holds = |fragment: &str| {
            command.command_line.contains(fragment)
                || command
                    .stdin
                    .as_deref()
                    .is_some_and(|stdin| stdin.contains(fragment))
        };
        match self
            .answers
            .iter()
            .filter(|answer| answer.from_unix_ms <= now && holds(&answer.fragment))
            .max_by_key(|answer| answer.from_unix_ms)
        {
            Some(answer) => Ok(answer.output.clone()),
            None => bail!("no recorded answer for {} at {now}", command.observer),
        }
    }
}
