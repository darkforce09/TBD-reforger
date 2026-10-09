//! `--migrate-host-agent-name`: the decision table, the script text, and the script run under a
//! local bash against a scratch home with a stand-in `systemctl`.
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

#[test]
fn the_decision_table_moves_only_a_lone_retired_name_and_refuses_both() {
    assert_eq!(rename_decision(true, false), RenameDecision::Move);
    assert_eq!(
        rename_decision(false, true),
        RenameDecision::AlreadyMigrated
    );
    assert_eq!(rename_decision(false, false), RenameDecision::NothingToMove);
    assert_eq!(rename_decision(true, true), RenameDecision::Refuse);
}

/// The script's two `case` blocks are the decision table, one arm per state, for the
/// configuration folder and the binary.
#[test]
fn the_script_applies_the_decision_table_to_the_folder_and_the_binary() {
    let p = migration_payload();
    assert!(p.contains(
        "case \"$(present \"$RETIRED_CONFIGURATION\"):$(present \"$CURRENT_CONFIGURATION\")\" in\n\
         \x20 yes:no) move_configuration=yes ;;\n\
         \x20 no:yes) move_configuration=no; echo \"  the configuration folder is already at ~/.config/game_server_host_agent\" ;;\n\
         \x20 no:no) move_configuration=no ;;\n\
         \x20 yes:yes) refuse \"both ~/.config/fleet_host_agent and ~/.config/game_server_host_agent exist; \
         keep the configuration folder the running agents use, move the other out of the way\" ;;\n\
         esac\n"
    ), "{p}");
    assert!(
        p.contains(
            "case \"$(present \"$RETIRED_BINARY\"):$(present \"$CURRENT_BINARY\")\" in\n\
         \x20 yes:no) move_binary=yes ;;\n"
        ),
        "{p}"
    );
    assert!(p.contains(
        "  yes:yes) refuse \"both ~/.local/bin/fleet_host_agent and ~/.local/bin/game_server_host_agent exist; \
         keep the binary the running agents use, move the other out of the way\" ;;\n"
    ), "{p}");
}

/// The script names every path in both spellings, refuses before any change, moves without
/// overwriting, removes only the retired template, and installs the committed current template.
#[test]
fn the_script_text_follows_the_five_steps_in_order() {
    let p = migration_payload();
    assert!(p.starts_with("set -euo pipefail\n"), "{p}");
    for line in [
        "RETIRED_BINARY=\"$HOME/.local/bin/fleet_host_agent\"\n",
        "CURRENT_BINARY=\"$HOME/.local/bin/game_server_host_agent\"\n",
        "RETIRED_CONFIGURATION=\"$HOME/.config/fleet_host_agent\"\n",
        "CURRENT_CONFIGURATION=\"$HOME/.config/game_server_host_agent\"\n",
        "RETIRED_TEMPLATE=\"$UNITS/fleet_host_agent@.service\"\n",
        "CURRENT_TEMPLATE=\"$UNITS/game_server_host_agent@.service\"\n",
        "for n in 1 2 3 4 5; do\n  unit=\"fleet_host_agent@$n.service\"\n",
        "  exit 3\n",
    ] {
        assert!(p.contains(line), "missing {line:?} in {p}");
    }
    let order = [
        "refuse \"both ~/.config/fleet_host_agent",
        "refuse \"both ~/.local/bin/fleet_host_agent",
        "nothing to migrate: the host carries no fleet_host_agent name",
        "  systemctl --user disable --now \"fleet_host_agent@$n.service\"\n",
        "  mv -T \"$RETIRED_BINARY\" \"$CURRENT_BINARY\"\n",
        "  mv -T \"$RETIRED_CONFIGURATION\" \"$CURRENT_CONFIGURATION\"\n",
        "  rm -f -- \"$RETIRED_TEMPLATE\"\n",
        &format!("cat > \"$CURRENT_TEMPLATE\" <<'UNITEOF'\n{HOST_AGENT_TEMPLATE}UNITEOF\n"),
        "systemctl --user daemon-reload\n",
        "  systemctl --user enable --now \"game_server_host_agent@$n.service\"\n",
        "    echo \"FAIL: $unit is '$state', not active.\" >&2\n",
    ];
    let positions: Vec<usize> = order
        .iter()
        .map(|needle| {
            p.find(needle)
                .unwrap_or_else(|| panic!("missing {needle:?} in {p}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{positions:?}"
    );
    // The one removal is the retired template; nothing else is deleted.
    assert_eq!(p.matches("rm ").count(), 1, "{p}");
}

/// The dry run prints what the migration does, then the exact script, line for line.
#[test]
fn the_dry_run_prints_the_exact_script() {
    let lines = migration_plan_lines("deploy@192.0.2.10");
    assert!(lines[0].starts_with(
        "[dry-run] migrate the host agent names: stop and disable every fleet_host_agent@N.service; \
         move ~/.local/bin/fleet_host_agent to ~/.local/bin/game_server_host_agent and \
         ~/.config/fleet_host_agent/ to ~/.config/game_server_host_agent/; remove \
         fleet_host_agent@.service; install game_server_host_agent@.service; enable and start \
         game_server_host_agent@N.service for the same N;"
    ), "{}", lines[0]);
    assert_eq!(
        lines[1],
        "[dry-run] ssh deploy@192.0.2.10 bash -s, with this script on stdin:"
    );
    assert_eq!(lines[2..].join("\n") + "\n", migration_payload());
}

/// Without the flag, a deploy refuses while any retired name is left on the host.
#[test]
fn the_deploy_check_names_every_retired_path() {
    let p = retired_names_absent_payload();
    assert!(
        p.contains(
            "for path in \"$HOME/.config/systemd/user/fleet_host_agent@.service\" \
         \"$HOME/.config/fleet_host_agent\" \"$HOME/.local/bin/fleet_host_agent\"; do\n"
        ),
        "{p}"
    );
    assert!(p.contains("Run cargo xtask deploy staging --migrate-host-agent-name first"));
    assert!(p.contains("  exit 1\n"));
    assert!(retired_names_absent_plan_line().contains("--migrate-host-agent-name moves them"));
}

// ── The script under a local bash ───────────────────────────────────────────────────────────

/// A scratch home with a stand-in `systemctl` that keeps unit state in files, and stand-ins for
/// `sleep` and `journalctl`, so nothing on this machine is touched.
struct ScratchHost {
    home: PathBuf,
}

const SYSTEMCTL_STAND_IN: &str = r#"#!/bin/sh
state="$HOME/systemd-state"
echo "$*" >> "$state/calls"
[ "$1" = --user ] && shift
case "$1" in
  show-environment) exit 0 ;;
  is-enabled) if [ -e "$state/enabled/$2" ]; then echo enabled; else echo disabled; exit 1; fi ;;
  is-active) if [ -e "$state/active/$2" ]; then echo active; else echo inactive; exit 3; fi ;;
  disable)
    [ -e "$state/fail-disable" ] && { echo "Failed to disable $3" >&2; exit 1; }
    rm -f "$state/enabled/$3" "$state/active/$3" ;;
  enable)
    template="${3%%@*}@.service"
    [ -f "$HOME/.config/systemd/user/$template" ] || { echo "Unit file $3 does not exist." >&2; exit 1; }
    touch "$state/enabled/$3" "$state/active/$3" ;;
  daemon-reload) ;;
  *) echo "unexpected systemctl $*" >&2; exit 1 ;;
esac
"#;

impl ScratchHost {
    fn new(name: &str) -> Self {
        let home = std::env::temp_dir().join(format!(
            "tbd-host-agent-rename-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&home);
        for folder in [
            "bin",
            "systemd-state/enabled",
            "systemd-state/active",
            ".config/systemd/user",
            ".local/bin",
        ] {
            std::fs::create_dir_all(home.join(folder)).unwrap();
        }
        std::fs::write(home.join("systemd-state/calls"), "").unwrap();
        for (tool, body) in [
            ("systemctl", SYSTEMCTL_STAND_IN),
            ("sleep", "#!/bin/sh\nexit 0\n"),
            ("journalctl", "#!/bin/sh\nexit 0\n"),
        ] {
            let path = home.join("bin").join(tool);
            std::fs::write(&path, body).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self { home }
    }

    fn path(&self, under_home: &str) -> PathBuf {
        self.home.join(under_home)
    }

    /// A host as the retired names left it: the template, the binary, an owner-only configuration
    /// folder for each of `instances`, and their units enabled and running.
    fn with_retired_install(self, instances: &[u16]) -> Self {
        std::fs::write(
            self.path(".config/systemd/user")
                .join(RETIRED_UNIT_TEMPLATE),
            "retired template",
        )
        .unwrap();
        write_executable(&self.path(RETIRED_BINARY_UNDER_HOME), "retired binary");
        let folder = self.path(RETIRED_CONFIGURATION_UNDER_HOME);
        for n in instances {
            let instance = folder.join(format!("instance-{n}"));
            std::fs::create_dir_all(&instance).unwrap();
            let file = instance.join("agent.toml");
            std::fs::write(&file, format!("instance {n}")).unwrap();
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
            std::fs::set_permissions(&instance, std::fs::Permissions::from_mode(0o700)).unwrap();
            self.set_unit(&format!("{RETIRED_HOST_AGENT_NAME}@{n}.service"), true);
        }
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
        self
    }

    fn set_unit(&self, unit: &str, running: bool) {
        for kind in ["enabled", "active"] {
            let marker = self.path("systemd-state").join(kind).join(unit);
            if running {
                std::fs::write(marker, "").unwrap();
            } else {
                let _ = std::fs::remove_file(marker);
            }
        }
    }

    fn unit_is_running(&self, unit: &str) -> bool {
        ["enabled", "active"]
            .iter()
            .all(|kind| self.path("systemd-state").join(kind).join(unit).exists())
    }

    fn run(&self) -> Output {
        std::fs::write(self.path("systemd-state/calls"), "").unwrap();
        std::process::Command::new("bash")
            .arg("-c")
            .arg(migration_payload())
            .env("HOME", &self.home)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.home.join("bin").display()),
            )
            .output()
            .expect("bash runs")
    }

    /// The `systemctl` calls of the last run that change state.
    fn changing_calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.path("systemd-state/calls"))
            .unwrap()
            .lines()
            .filter(|call| {
                ![
                    "--user show-environment",
                    "--user is-enabled ",
                    "--user is-active ",
                ]
                .iter()
                .any(|query| call.starts_with(query))
            })
            .map(str::to_string)
            .collect()
    }

    /// Every file under the home with its mode, the stand-ins' state and tools left out.
    fn snapshot(&self) -> Vec<(String, u32)> {
        let mut files = Vec::new();
        collect(&self.home, &self.home, &mut files);
        files.retain(|(path, _)| !path.starts_with("bin") && !path.starts_with("systemd-state"));
        files.sort();
        files
    }
}

impl Drop for ScratchHost {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn write_executable(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn collect(root: &Path, at: &Path, files: &mut Vec<(String, u32)>) {
    for entry in std::fs::read_dir(at).unwrap() {
        let path = entry.unwrap().path();
        let mode = std::fs::symlink_metadata(&path)
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        let relative = path.strip_prefix(root).unwrap().display().to_string();
        files.push((relative, mode));
        if path.is_dir() {
            collect(root, &path, files);
        }
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Retired names only: every step runs, the folder keeps its files and their modes, the same
/// instances come back under the current units, and a second run changes nothing.
#[test]
fn old_only_migrates_every_step_and_a_second_run_is_a_no_op() {
    let host = ScratchHost::new("old-only").with_retired_install(&[1, 2]);
    let out = host.run();
    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));

    for n in [1, 2] {
        assert!(!host.unit_is_running(&format!("fleet_host_agent@{n}.service")));
        assert!(host.unit_is_running(&format!("game_server_host_agent@{n}.service")));
        let file = host
            .path(HOST_AGENT_CONFIGURATION_UNDER_HOME)
            .join(format!("instance-{n}/agent.toml"));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            format!("instance {n}")
        );
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&file), 0o600, "the file keeps its owner-only mode");
        assert_eq!(mode(file.parent().unwrap()), 0o700);
    }
    assert!(!host.unit_is_running("game_server_host_agent@3.service"));
    assert!(!host.path(RETIRED_CONFIGURATION_UNDER_HOME).exists());
    assert!(!host.path(RETIRED_BINARY_UNDER_HOME).exists());
    assert_eq!(
        std::fs::read_to_string(host.path(HOST_AGENT_BINARY_UNDER_HOME)).unwrap(),
        "retired binary",
        "the binary moved, not replaced"
    );
    let units = host.path(".config/systemd/user");
    assert!(!units.join(RETIRED_UNIT_TEMPLATE).exists());
    assert_eq!(
        std::fs::read_to_string(units.join(HOST_AGENT_UNIT_TEMPLATE)).unwrap(),
        HOST_AGENT_TEMPLATE
    );
    assert_eq!(
        host.changing_calls(),
        [
            "--user disable --now fleet_host_agent@1.service",
            "--user disable --now fleet_host_agent@2.service",
            "--user daemon-reload",
            "--user enable --now game_server_host_agent@1.service",
            "--user enable --now game_server_host_agent@2.service",
        ]
    );

    let before = host.snapshot();
    let again = host.run();
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(
        stdout(&again).contains("nothing to migrate"),
        "{}",
        stdout(&again)
    );
    assert!(
        host.changing_calls().is_empty(),
        "{:?}",
        host.changing_calls()
    );
    assert_eq!(host.snapshot(), before, "the second run changed the host");
}

/// Current names only (a migrated or freshly installed host): nothing changes.
#[test]
fn new_only_changes_nothing() {
    let host = ScratchHost::new("new-only");
    let instance = host
        .path(HOST_AGENT_CONFIGURATION_UNDER_HOME)
        .join("instance-1");
    std::fs::create_dir_all(&instance).unwrap();
    std::fs::write(instance.join("agent.toml"), "current").unwrap();
    write_executable(&host.path(HOST_AGENT_BINARY_UNDER_HOME), "current binary");
    host.set_unit("game_server_host_agent@1.service", true);
    let before = host.snapshot();
    let out = host.run();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("is already at ~/.config/game_server_host_agent"));
    assert!(stdout(&out).contains("nothing to migrate"));
    assert!(
        host.changing_calls().is_empty(),
        "{:?}",
        host.changing_calls()
    );
    assert_eq!(host.snapshot(), before);
}

/// Both configuration folders: refused with exit 3 before any step, the operator's decision named.
#[test]
fn both_folders_refuse_before_anything_changes() {
    let host = ScratchHost::new("both").with_retired_install(&[1]);
    std::fs::create_dir_all(
        host.path(HOST_AGENT_CONFIGURATION_UNDER_HOME)
            .join("instance-1"),
    )
    .unwrap();
    let before = host.snapshot();
    let out = host.run();
    assert_eq!(out.status.code(), Some(REFUSED), "{}", stderr(&out));
    assert!(
        stderr(&out).contains(
            "REFUSED: both ~/.config/fleet_host_agent and ~/.config/game_server_host_agent exist"
        ),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("Nothing on the host was changed"));
    assert!(
        host.changing_calls().is_empty(),
        "{:?}",
        host.changing_calls()
    );
    assert!(host.unit_is_running("fleet_host_agent@1.service"));
    assert_eq!(host.snapshot(), before);
}

/// Both binaries: refused the same way.
#[test]
fn both_binaries_refuse_before_anything_changes() {
    let host = ScratchHost::new("both-binaries").with_retired_install(&[1]);
    write_executable(&host.path(HOST_AGENT_BINARY_UNDER_HOME), "current binary");
    let before = host.snapshot();
    let out = host.run();
    assert_eq!(out.status.code(), Some(REFUSED), "{}", stderr(&out));
    assert!(stderr(&out).contains("both ~/.local/bin/fleet_host_agent and"));
    assert!(host.changing_calls().is_empty());
    assert_eq!(host.snapshot(), before);
}

/// Neither name (a host the fleet never reached): nothing to migrate, nothing written.
#[test]
fn neither_changes_nothing() {
    let host = ScratchHost::new("neither");
    let before = host.snapshot();
    let out = host.run();
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("nothing to migrate"));
    assert!(host.changing_calls().is_empty());
    assert_eq!(host.snapshot(), before);
}

/// A step that fails stops the run with the failure named, before anything is moved or removed.
#[test]
fn a_failing_step_stops_before_anything_is_moved_or_removed() {
    let host = ScratchHost::new("failing").with_retired_install(&[1]);
    std::fs::write(host.path("systemd-state/fail-disable"), "").unwrap();
    let before = host.snapshot();
    let out = host.run();
    assert!(!out.status.success());
    assert_ne!(out.status.code(), Some(REFUSED));
    assert!(
        stderr(&out).contains("FAIL: the host agent name migration stopped part way"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        host.snapshot(),
        before,
        "a failed step moved or removed something"
    );
}
