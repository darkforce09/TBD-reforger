//! T-181.7 - every `.layout` the UI framework owns, named once.
//!
//! Enfusion resources are addressed as `"{GUID}relative/path.layout"`. The GUID is the primary
//! key and lives in the file's `.meta`; the path is the fallback. Keeping both in one place means
//! that when Workbench next rewrites `resourceDatabase.rdb`, a changed GUID or a moved file is a
//! one-line edit here rather than a hunt through screens.
//!
//! -- Tree (UI reorg 2026-09-12) ----------------------------------------------------------------
//! `UI/layouts/` mirrors `Scripts/Game/TBD/UI/`. See `docs/mod/ui/UI_STRUCTURE.md`.
//!   Core/        framework chrome (shell, list row)                  block 07
//!   Common/      shared rows / chips / chrome sub-layouts            block 1B
//!   PreGame/     Shared 16 · MissionSelector 0B · Lobby 0C (LoadoutPreview 15) · Briefing 0D (panels 17)
//!   InGameMenu/  Pause 18 · Admin 19
//!   Hud/         ObjectiveHud 0A · Spectator 1A
//!   PostGame/    EndScreen 08 · DebriefScreen 09
//! GUIDs are `7BD1A7000000XXnn`: `XX` = block, `nn` = 00 root widget, 01 the `.meta` resource id,
//! 02+ child widgets. Older screens spilled past their block (Lobby into 0E, Briefing into 0EA0+,
//! MissionSelector into A0/B0, 0F) - so before taking a block for a new screen run
//! `grep -rho '{7BD1A7000000XX' UI Scripts` and pick one with zero hits.
//!
//! -- Resource visibility (measured on the headless server) ----------------------------------
//!   * **Scripts do not need an rdb entry.** Script discovery is a directory scan, so moved or
//!     new `.c` files compile without Workbench.
//!   * **Non-script resources ARE indexed by the rdb.** A `.layout` / `.conf` at a new path is
//!     invisible until the project is opened in Workbench and `resourceDatabase.rdb` is rewritten.
//!     After moving layouts: cold-restart Workbench, open `addon.gproj`, commit the rdb.
//!   * `Create()` falls back to the bare path when a GUID does not resolve, which removes one
//!     class of first-run failure but does not replace the rdb pass.
class TBD_UILayouts
{
	// -- Core -------------------------------------------------------------------------------------
	//! The chrome every TBD screen sits in: backdrop, header, content frame, one primary action.
	static const ResourceName SCREEN_SHELL = "{7BD1A70000000701}UI/layouts/Core/TBD_ScreenShell.layout";
	//! One pooled row of a TBD_ListBox.
	static const ResourceName LIST_ROW     = "{7BD1A70000000702}UI/layouts/Core/TBD_ListRow.layout";

	// -- PreGame / MissionSelector -------------------------------------------------------------
	static const ResourceName MISSION_SELECTOR              = "{7BD1A70000000B01}UI/layouts/PreGame/MissionSelector/TBD_MissionSelector.layout";
	static const ResourceName MISSION_SELECTOR_TERRAIN_ROW  = "{7BD1A70000000B20}UI/layouts/PreGame/MissionSelector/TBD_TerrainRow.layout";
	static const ResourceName MISSION_SELECTOR_MISSION_CARD = "{7BD1A70000000B40}UI/layouts/PreGame/MissionSelector/TBD_MissionCard.layout";

	// -- PreGame / Lobby -----------------------------------------------------------------------
	static const ResourceName LOBBY_SCREEN       = "{7BD1A70000000C01}UI/layouts/PreGame/Lobby/TBD_LobbyScreen.layout";
	static const ResourceName LOBBY_SQUAD_CARD   = "{7BD1A70000000C02}UI/layouts/PreGame/Lobby/TBD_LobbySquadCard.layout";
	static const ResourceName LOBBY_SLOT_ROW     = "{7BD1A70000000C03}UI/layouts/PreGame/Lobby/TBD_LobbySlotRow.layout";
	static const ResourceName LOBBY_FACTION_ROW  = "{7BD1A70000000C04}UI/layouts/PreGame/Lobby/TBD_LobbyFactionRow.layout";
	static const ResourceName LOBBY_HEADER       = "{7BD1A70000000C05}UI/layouts/PreGame/Lobby/TBD_LobbyHeader.layout";
	static const ResourceName LOBBY_FOOTER       = "{7BD1A70000000C06}UI/layouts/PreGame/Lobby/TBD_LobbyFooter.layout";
	static const ResourceName LOBBY_SIDEBAR      = "{7BD1A70000000C07}UI/layouts/PreGame/Lobby/TBD_LobbySidebar.layout";
	static const ResourceName LOBBY_ROSTER       = "{7BD1A70000000C08}UI/layouts/PreGame/Lobby/TBD_LobbyCenterRoster.layout";
	static const ResourceName LOBBY_INSPECTOR    = "{7BD1A70000000C09}UI/layouts/PreGame/Lobby/TBD_LobbyInspector.layout";
	//! T-139 - kit icon grid beside the slot list (block 15; was 0A, which collided with the HUD).
	static const ResourceName LOADOUT_PREVIEW    = "{7BD1A70000001501}UI/layouts/PreGame/Lobby/TBD_LoadoutPreview.layout";

	// -- PreGame / Briefing --------------------------------------------------------------------
	static const ResourceName BRIEFING_SCREEN    = "{7BD1A70000000D01}UI/layouts/PreGame/Briefing/TBD_BriefingScreen.layout";

	// -- Hud -----------------------------------------------------------------------------------
	//! T-941.4 - objective list + capture bar.
	static const ResourceName OBJECTIVE_HUD      = "{7BD1A70000000A01}UI/layouts/Hud/TBD_ObjectiveHud.layout";

	// -- PostGame ------------------------------------------------------------------------------
	//! T-941.3 - END stage banner: winning faction + reason.
	static const ResourceName END_SCREEN     = "{7BD1A70000000801}UI/layouts/PostGame/TBD_EndScreen.layout";
	//! T-941.3 - DEBRIEF stage scoreboard.
	static const ResourceName DEBRIEF_SCREEN = "{7BD1A70000000901}UI/layouts/PostGame/TBD_DebriefScreen.layout";

	//------------------------------------------------------------------------------------------------
	//! Instantiate a layout under `parent`, retrying without the GUID prefix if the GUID does not
	//! resolve. Returns null on a dead workspace (server-side) or an unresolvable layout - every
	//! caller must handle null, because on a dedicated server there is no workspace at all.
	static Widget Create(ResourceName layout, Widget parent)
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return null; // headless / server - nothing to draw on

		if (layout.IsEmpty())
			return null;

		Widget created = workspace.CreateWidgets(layout, parent);
		if (created)
			return created;

		string bare = StripGuid(layout);
		if (bare == layout)
			return null; // no GUID prefix to strip; the layout is genuinely missing

		Print(string.Format("[TBD][ui] layout GUID did not resolve, retrying by path: %1", bare), LogLevel.WARNING);
		return workspace.CreateWidgets(bare, parent);
	}

	//------------------------------------------------------------------------------------------------
	//! `"{GUID}UI/x.layout"` -> `"UI/x.layout"`. Returns the input unchanged when there is no
	//! `{...}` prefix.
	static string StripGuid(string resource)
	{
		int close = resource.IndexOf("}");
		if (close < 0)
			return resource;

		return resource.Substring(close + 1, resource.Length() - close - 1);
	}
}
