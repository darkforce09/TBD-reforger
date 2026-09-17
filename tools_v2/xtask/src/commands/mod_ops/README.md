# Game Mod Operations (`xtask/src/commands/mod_ops`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Operational tasks for compiling, launching, and diagnosing the Enfusion mod.

Consolidates operational tasks previously misnamed as gates:
- **`compile.rs`** (formerly `gate_mod_compile.rs`): Headless compile check harness.
- **`dev_server.rs`** (formerly `gate_run_dev_server.rs`): Reforger development server launcher.
- **`playtest.rs`** (formerly `playtest_server.rs`): Dedicated playtest server orchestrator.
- **`dev_bootstrap.rs`** (formerly `gate_tbd_dev_bootstrap.rs`): Full development environment bootstrap.
- **`remote_logs.rs`** (formerly `gate_remote_log_grep.rs`): Game-server log streamer over SSH.
- **`test_mission.rs`** (formerly `gate_test_mission.rs`): Test mission switcher.
- **`test_phase1_api.rs`** (formerly `gate_test_phase1_api.rs`): Game REST API smoke tester.
- **`seed_announcement.rs`** (formerly `gate_seed_milestone_announcement.rs`): Announcement seeder.
- **`bootstrap_staging.rs`** (formerly `gate_bootstrap_staging_server.rs`): Remote staging directory provisioner.
