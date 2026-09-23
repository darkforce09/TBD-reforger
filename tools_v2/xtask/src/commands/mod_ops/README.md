# Mod Ops

Workbench compilation and logs, world boot, dedicated server lifecycle, publishing, dependencies, and playtest operations. CLI declarations and dispatch remain in this command domain.

Source modules: `cli`, `compile`, `compile_host`, `development_bootstrap`, `development_server`, `dispatch`, `game_runtime_api_smoke` (the game-runtime routes called with a server's `mod_runtime` credential), `mission_test`, `mod`, `playtest_server`, `wave_execution`, `website_api_client` (the website API client the playtest, world boot, mission test and smoke drive the platform with: development login, mission publication, fleet provisioning, deployments and the mod's artifact cache), `world_boot`, `world_boot_verdict`.
