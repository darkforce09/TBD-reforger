/**
 * @file ChimeraMenuPreset.c
 * @brief Every menu preset the TBD framework adds to `ChimeraMenuPreset`.
 *
 * Role: the one place TBD screens get their preset names.  Position: each member is bound to a
 * layout and a screen class by a `MenuPreset` block in `Configs/System/chimeraMenus.conf`, and
 * opened by name through `GetGame().GetMenuManager().OpenMenu`.
 * State: none.  Invariants: member names are frozen, because configs reference them by name; a
 * preset opens only once the addon's `resourceDatabase.rdb` lists `chimeraMenus.conf`, which only
 * a Workbench pass regenerates. Until then the engine logs
 * `GUI (E): Menu preset '<name>' not found!` at every startup, which makes that line the exact
 * check (grep "Menu preset" in the profile error log). No script API registers a preset
 * at runtime: `MenuManager.RegisterPreset`, `OpenMenuByLayout`, `GetMenuPresets` and `FindPreset`
 * do not exist.
 */

//! TBD menu presets, one per screen.
modded enum ChimeraMenuPreset
{
	TBD_UIShell, //!< TBD_ShellScreen with no content: the end-to-end proof of the menu stack
	TBD_UIMissionSelector, //!< TBD_MissionSelectorScreen: pick the mission to deploy
	TBD_UIBriefing, //!< TBD_BriefingScreen: the faction briefing
	TBD_UILobby, //!< TBD_LobbyScreen: slot selection
	TBD_UIAdmin, //!< TBD_AdminScreen: the admin console over the audit trail
	TBD_Spectator //!< TBD_SpectatorScreen: the spectator roster on the shared shell layout
}
