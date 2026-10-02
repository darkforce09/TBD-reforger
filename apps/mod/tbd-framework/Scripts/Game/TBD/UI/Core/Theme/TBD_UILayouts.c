/**
 * @file TBD_UILayouts.c
 * @brief Every layout and texture resource the UI framework owns, named once, and the create helpers.
 *
 * Role: holds one `"{GUID}path"` constant per layout and texture and creates layouts with a
 * bare-path retry, a full-width stretch, a handler lookup or a rounded-shape mount.
 * Position: read by every screen, component and HUD under UI and Session; the GUIDs come from the
 * `.meta` files Workbench writes.
 * State: the set of texture misses already logged, for the life of the script VM on the client.
 * Invariants: a resource is named only here, so a changed GUID or a moved file is a one-line edit;
 * `Create` returns null without a workspace or for an unresolvable layout, and every caller
 * handles null; a new GUID block is checked against the ledger below.
 */

//! Resource registry and layout creation helpers.
//!
//! Enfusion resources are addressed as `"{GUID}relative/path.layout"`; the GUID is the primary key
//! and lives in the file's `.meta`, the path is the fallback.
//!
//! `UI/layouts/` mirrors the 7-domain architecture. See
//! `documentation/mod/tbd-framework/UI/README.md`.
//!   Common/      shared component library                     blocks 07, 10-14, 16-18, 24-2E, 36, 38-3D
//!   Hud/         ObjectiveHud                                  0A
//!   Session/     Shared 1B-1C - MissionSelector 0B, 1D-23 - Lobby 0C, 2F-37 -
//!                Briefing 0D, 3E-42, 46-49 - Shared 1B-1C, 43-45 - Spectator 1A - Admin 19 - PostGame 08/09
//! GUIDs are `7BD1A7000000XXnn`: `XX` = block, `nn` = 00 root widget, 01 the `.meta` resource id,
//! 02+ child widgets.
//!
//!   07 ScreenShell + ListRow          10 Panel            11 Chip             12 SearchBox
//!   13 NavItem + TabStrip             14 Button           16 KeyValueRow      17 Dropdown + Menu
//!   24 Columns2                       18 InsetText        25 PanelFill        1B SessionTopBar    1C SessionBottomBar
//!   26 Rounded12   27 Rounded8   28 Rounded6   29 Rounded10   2A Rounded11   2B Rounded7   2C Rounded5   2D Rounded9
//!   2E ScrollBar   38 Columns3   39 Columns4   36 StatCell
//!   3A Section     3B NumberedCard   3C Caption   3D ScrollList
//!   0D Briefing shell   3E FreqRow   3F OrbatPage   40 AssetPreview   41 UniformCard   42 MarkersPanel
//!   46 PrimaryNav   47 PrimaryNavItem   48 TopicNavItem   49 TopicNav
//!   43 PlayersPanel     44 PlayerLane   45 PlayerRow (Session/Shared)
//!   0C Lobby shell   2F LobbyFactionList   30 LobbyFactionRow   31 LobbyRoster   32 LobbySquadCard
//!   33 LobbySlotRow  34 KitInspector       35 KitPreview        37 KitWeaponCard
//!   0B MissionSelector shell          1D TerrainSelector  1E ScenarioBrowser  1F MissionInspector
//!   20 ModGridItem                    21 FactionColumn    22 TerrainRow       23 MissionCard
//!   0E/0F lobby shell children        A0 lobby header     B0 misc
//!
//!   * Scripts need no rdb entry: script discovery is a directory scan, so moved or new `.c` files
//!     compile without Workbench.
//!   * Non-script resources are indexed by the rdb: a `.layout` or `.conf` at a new path is
//!     invisible until the project is opened in Workbench and `resourceDatabase.rdb` is regenerated.
//!     After moving layouts: cold-restart Workbench, open `addon.gproj`, commit the rdb.
//!   * `Create()` falls back to the bare path when a GUID does not resolve, which removes one
//!     class of first-run failure but does not replace the rdb pass.
class TBD_UILayouts
{
	static const ResourceName SCREEN_SHELL = "{7BD1A70000000701}UI/layouts/Common/TBD_ScreenShell.layout"; //!< The chrome every TBD screen sits in: backdrop, header, content frame, one primary action.
	static const ResourceName LIST_ROW     = "{7BD1A70000000702}UI/layouts/Common/TBD_ListRow.layout"; //!< One pooled row of a TBD_ListBox.

	// Pre-game primitives. Contracts: `UI/layouts/Common/README.md`.
	static const ResourceName PANEL          = "{7BD1A70000001001}UI/layouts/Common/TBD_Panel.layout"; //!< Glass card, CONTENT-SIZED (stacks in a vertical list). `TBD_PanelComponent`.
	static const ResourceName PANEL_FILL     = "{7BD1A70000002501}UI/layouts/Common/TBD_PanelFill.layout"; //!< Same card, FRAME-ANCHORED (fills the dock it is mounted into; body takes the remaining height). Same widget names, same handler. Use it for columns, PANEL for cards.
	static const ResourceName CHIP           = "{7BD1A70000001101}UI/layouts/Common/TBD_Chip.layout"; //!< Tinted mono pill. `TBD_ChipComponent`.
	static const ResourceName SEARCH_BOX     = "{7BD1A70000001201}UI/layouts/Common/TBD_SearchBox.layout"; //!< Icon + EditBox + clear. `TBD_SearchBoxComponent`.
	static const ResourceName NAV_ITEM       = "{7BD1A70000001301}UI/layouts/Common/TBD_NavItem.layout"; //!< One tab of a strip (icon, label, badge). `TBD_NavItemComponent`.
	static const ResourceName TAB_STRIP      = "{7BD1A70000001302}UI/layouts/Common/TBD_TabStrip.layout"; //!< Segmented control that instantiates NAV_ITEM per entry. `TBD_TabStripComponent`.
	static const ResourceName BUTTON         = "{7BD1A70000001401}UI/layouts/Common/TBD_Button.layout"; //!< Runtime-instantiable button (Background, Border, Label). `TBD_UIButton`.
	static const ResourceName KEY_VALUE_ROW  = "{7BD1A70000001601}UI/layouts/Common/TBD_KeyValueRow.layout"; //!< Label left, mono value right. `TBD_KeyValueRowComponent`.
	static const ResourceName DROPDOWN       = "{7BD1A70000001701}UI/layouts/Common/TBD_Dropdown.layout"; //!< Popover trigger pill. `TBD_DropdownComponent`.
	static const ResourceName DROPDOWN_MENU  = "{7BD1A70000001702}UI/layouts/Common/TBD_DropdownMenu.layout"; //!< Popover menu (scrim + list) mounted into a screen's OverlayDock by the dropdown.
	static const ResourceName COLUMNS_2      = "{7BD1A70000002401}UI/layouts/Common/TBD_Columns2.layout"; //!< Two equal columns (`ColumnA`, `ColumnB`) for grids of cards / rows.
	static const ResourceName COLUMNS_3      = "{7BD1A70000003801}UI/layouts/Common/TBD_Columns3.layout"; //!< three equal columns `ColumnA..C` (kit inspector grids)
	static const ResourceName COLUMNS_4      = "{7BD1A70000003901}UI/layouts/Common/TBD_Columns4.layout"; //!< four equal columns `ColumnA..D` (kit inspector grids)
	static const ResourceName INSET_TEXT     = "{7BD1A70000001801}UI/layouts/Common/TBD_InsetText.layout"; //!< Wrapped paragraph in an inset box (`Body`). Summary, lore, rules.
	static const ResourceName SECTION        = "{7BD1A70000003A01}UI/layouts/Common/TBD_Section.layout"; //!< Collapsible card (header button + body). `TBD_SectionComponent`.
	static const ResourceName NUMBERED_CARD  = "{7BD1A70000003B01}UI/layouts/Common/TBD_NumberedCard.layout"; //!< `(n) Title [chip]` card with paragraph, body dock and right-aligned footer. `TBD_NumberedCardComponent`.
	static const ResourceName CAPTION        = "{7BD1A70000003C01}UI/layouts/Common/TBD_Caption.layout"; //!< Uppercase mono section label + trailing note. `TBD_Caption.Mount`.
	static const ResourceName SCROLL_LIST    = "{7BD1A70000003D01}UI/layouts/Common/TBD_ScrollList.layout"; //!< Scrolling vertical list with the TBD scrollbar (the clip recipe). `TBD_ScrollList.Mount`.
	static const ResourceName STAT_CELL      = "{7BD1A70000003601}UI/layouts/Common/TBD_StatCell.layout"; //!< Label / value / count cell (kit inspector grids, objective stats, asset inventories).
	// Rounded rectangle, radius baked per file (Enfusion has no corner radius and no nine-patch scaling).
	// Seven images: centre, two strips, four clipped quarter-discs (see CORNER_DISC). Mounted into
	// a `Border` / `Background` frame dock by `MountRounded`; the dock is painted and
	// TBD_UITheme.Tint walks the shape (frame colour does not inherit). Eight hand-kept copies that
	// differ only in radius: even = border, odd = the 1 px-inset fill under the border of the even
	// radius above (concentric arcs; the same radius twice reads as bracket arcs at every corner).
	static const ResourceName ROUNDED_12     = "{7BD1A70000002601}UI/layouts/Common/TBD_Rounded12.layout"; //!< radius 12 border (panels)
	static const ResourceName ROUNDED_8      = "{7BD1A70000002701}UI/layouts/Common/TBD_Rounded8.layout"; //!< radius 8 border (rows); MountRounded's default
	static const ResourceName ROUNDED_6      = "{7BD1A70000002801}UI/layouts/Common/TBD_Rounded6.layout"; //!< radius 6 border (tags)
	static const ResourceName ROUNDED_10     = "{7BD1A70000002901}UI/layouts/Common/TBD_Rounded10.layout"; //!< radius 10 border (pills)
	static const ResourceName ROUNDED_11     = "{7BD1A70000002A01}UI/layouts/Common/TBD_Rounded11.layout"; //!< radius 11 fill under a radius 12 border
	static const ResourceName ROUNDED_7      = "{7BD1A70000002B01}UI/layouts/Common/TBD_Rounded7.layout"; //!< radius 7 fill under a radius 8 border
	static const ResourceName ROUNDED_5      = "{7BD1A70000002C01}UI/layouts/Common/TBD_Rounded5.layout"; //!< radius 5 fill under a radius 6 border
	static const ResourceName ROUNDED_9      = "{7BD1A70000002D01}UI/layouts/Common/TBD_Rounded9.layout"; //!< radius 9 fill under a radius 10 border

	static const ResourceName CORNER_DISC    = "{1F2DC726318EC5AF}UI/Textures/TBD/TBD_Disc_UI.edds"; //!< Filled white disc the corner quarters are cut from: `UI/Textures/TBD/TBD_Disc_UI.png` (128 px, anti-aliased, ours; vanilla `circleFull.edds` is a ring); the GUID is its `.meta` id. Referenced by the eight shape layouts.
	static const ResourceName CORNER_DISC_INV = "{8DF41982E2A2BBA5}UI/Textures/TBD/TBD_DiscInv_UI.edds"; //!< Inverse of CORNER_DISC: an opaque square with a transparent disc. Clipped quarters of it, painted in the colour BEHIND a surface, round the corners of a photo (inspector hero).
	static const ResourceName SCROLL_BAR      = "{7BD1A70000002E01}UI/layouts/Common/TBD_ScrollBar.layout"; //!< 4 px track + thumb driven by TBD_UIScrollBar; mounted into a list's `ScrollBarDock`.

	// GUIDs come from the `.meta` Workbench writes on import; a new PNG starts as a bare path and is
	// pinned here after its import (the rdb resolves the bare path meanwhile).
	static const ResourceName FADE_DOWN       = "{5DB4948B38B2380A}UI/Textures/TBD/TBD_FadeDown_UI.edds"; //!< Vertical fade, alpha 0 (top) -> 1 (bottom); tinted to the colour it fades into.
	static const ResourceName HERO_EVERON     = "{9D92B49A28C50256}UI/Textures/TBD/TBD_Hero_Everon_UI.edds"; //!< Everon satellite band (map-assets full.webp, crop 4096x560 at y 1600, scaled 1024x140).
	static const ResourceName HERO_TOPO       = "{B546577F58DCE62A}UI/Textures/TBD/TBD_HeroTopo_UI.edds"; //!< The Stitch hero art (40 px grid, contours, dashed circle) for terrains without imagery.

	static const ResourceName SESSION_TOP_BAR    = "{7BD1A70000001B01}UI/layouts/Session/Shared/TBD_SessionTopBar.layout"; //!< Title - Scenario Browser / Lobby / Briefing strip - identity - player count. `TBD_SessionTopBar`.
	static const ResourceName SESSION_BOTTOM_BAR = "{7BD1A70000001C01}UI/layouts/Session/Shared/TBD_SessionBottomBar.layout"; //!< Left / right action docks filled with BUTTON at runtime. `TBD_SessionBottomBar`.

	static const ResourceName MISSION_SELECTOR              = "{7BD1A70000000B01}UI/layouts/Session/MissionSelector/TBD_MissionSelector.layout"; //!< Dock shell only (TopDock, LeftDock, CenterDock, RightDock, BottomDock, OverlayDock). Its GUID and path are the ones `chimeraMenus.conf` and the rdb row name.
	static const ResourceName MISSION_SELECTOR_TERRAINS     = "{7BD1A70000001D01}UI/layouts/Session/MissionSelector/TBD_TerrainSelector.layout"; //!< Body of the TERRAINS panel: search dock + scrolling list.
	static const ResourceName MISSION_SELECTOR_BROWSER      = "{7BD1A70000001E01}UI/layouts/Session/MissionSelector/TBD_ScenarioBrowser.layout"; //!< Body of the <TERRAIN> MISSIONS panel: search + modes dropdown + scrolling cards.
	static const ResourceName MISSION_SELECTOR_INSPECTOR    = "{7BD1A70000001F01}UI/layouts/Session/MissionSelector/TBD_MissionInspector.layout"; //!< Right column: hero banner + scrolling card stack.
	static const ResourceName MISSION_SELECTOR_TERRAIN_ROW  = "{7BD1A70000002201}UI/layouts/Session/MissionSelector/TBD_TerrainRow.layout"; //!< One pooled terrain row. `TBD_TerrainRowComponent`.
	static const ResourceName MISSION_SELECTOR_MISSION_CARD = "{7BD1A70000002301}UI/layouts/Session/MissionSelector/TBD_MissionCard.layout"; //!< One pooled mission card. `TBD_MissionCardComponent`.
	static const ResourceName MISSION_SELECTOR_MOD_ITEM     = "{7BD1A70000002001}UI/layouts/Session/MissionSelector/TBD_ModGridItem.layout"; //!< One mod in the REQUIRED MODSET grid.
	static const ResourceName MISSION_SELECTOR_FACTION_COL  = "{7BD1A70000002101}UI/layouts/Session/MissionSelector/TBD_FactionColumn.layout"; //!< One faction column (ORBAT overview / objectives).

	static const ResourceName LOBBY_SCREEN        = "{7BD1A70000000C01}UI/layouts/Session/Lobby/TBD_LobbyScreen.layout"; //!< Dock shell only (TopDock, LeftDock 320, CenterDock 500, RightDock, BottomDock, OverlayDock). Its GUID and path are the ones `chimeraMenus.conf` and the rdb row name.
	static const ResourceName LOBBY_FACTION_LIST  = "{7BD1A70000002F01}UI/layouts/Session/Lobby/TBD_LobbyFactionList.layout"; //!< Body of the FACTIONS panel: faction rows, spectators row, a dock for the voice panel.
	static const ResourceName LOBBY_FACTION_ROW   = "{7BD1A70000003001}UI/layouts/Session/Lobby/TBD_LobbyFactionRow.layout"; //!< One faction / spectators row. `TBD_LobbyFactionRowComponent`.
	static const ResourceName LOBBY_ROSTER        = "{7BD1A70000003101}UI/layouts/Session/Lobby/TBD_LobbyRoster.layout"; //!< Body of the ROLES panel: scrolling squad cards.
	static const ResourceName LOBBY_SQUAD_CARD    = "{7BD1A70000003201}UI/layouts/Session/Lobby/TBD_LobbySquadCard.layout"; //!< One collapsible squad card. `TBD_LobbySquadCardComponent`.
	static const ResourceName LOBBY_SLOT_ROW      = "{7BD1A70000003301}UI/layouts/Session/Lobby/TBD_LobbySlotRow.layout"; //!< One seat inside a squad card. `TBD_LobbySlotRowComponent`.
	static const ResourceName LOBBY_KIT_INSPECTOR = "{7BD1A70000003401}UI/layouts/Session/Lobby/TBD_KitInspector.layout"; //!< Right column: KIT INSPECTOR title band + scrolling section cards.
	static const ResourceName LOBBY_KIT_PREVIEW   = "{7BD1A70000003501}UI/layouts/Session/Lobby/TBD_KitPreview.layout"; //!< The preview card (empty frame until the visual-preview pass).
	static const ResourceName LOBBY_KIT_WEAPON    = "{7BD1A70000003701}UI/layouts/Session/Lobby/TBD_KitWeaponCard.layout"; //!< One WEAPON SLOT card: name, mounted attachments, ammunition.

	static const ResourceName BRIEFING_SCREEN    = "{7BD1A70000000D01}UI/layouts/Session/Briefing/TBD_BriefingScreen.layout"; //!< briefing dock shell; its GUID and path are the ones `chimeraMenus.conf` and the rdb row name
	// Briefing layouts. Contracts: `UI/layouts/Session/Briefing/README.md`.
	static const ResourceName BRIEFING_FREQ_ROW      = "{7BD1A70000003E01}UI/layouts/Session/Briefing/TBD_FreqRow.layout"; //!< One radio net (name - MHz chip - aux channels).
	static const ResourceName BRIEFING_ORBAT_PAGE    = "{7BD1A70000003F01}UI/layouts/Session/Briefing/TBD_OrbatPage.layout"; //!< ORBAT page: `RosterDock` 500 + `KitDock` (the lobby roster + kit inspector, read-only).
	static const ResourceName BRIEFING_ASSET_PREVIEW = "{7BD1A70000004001}UI/layouts/Session/Briefing/TBD_AssetPreview.layout"; //!< Vehicle Info render box (`Preview` ItemPreviewWidget, 168 tall).
	static const ResourceName BRIEFING_UNIFORM_CARD  = "{7BD1A70000004101}UI/layouts/Session/Briefing/TBD_UniformCard.layout"; //!< One faction uniform card (name - doll - chips - camo).
	static const ResourceName BRIEFING_MARKERS_PANEL = "{7BD1A70000004201}UI/layouts/Session/Briefing/TBD_MarkersPanel.layout"; //!< Markers panel (plan dropdown + Load Plan).
	static const ResourceName BRIEFING_PRIMARY_NAV      = "{7BD1A70000004601}UI/layouts/Session/Briefing/TBD_PrimaryNav.layout"; //!< primary navigation glass panel (Map, Briefing, Players, Markers); `TBD_BriefingPrimaryNav`
	static const ResourceName BRIEFING_PRIMARY_NAV_ITEM = "{7BD1A70000004701}UI/layouts/Session/Briefing/TBD_PrimaryNavItem.layout"; //!< one primary navigation item
	static const ResourceName BRIEFING_TOPIC_NAV      = "{7BD1A70000004901}UI/layouts/Session/Briefing/TBD_TopicNav.layout"; //!< topic navigation glass panel (10 topics, 3 groups); `TBD_BriefingTopicNav`
	static const ResourceName BRIEFING_TOPIC_NAV_ITEM = "{7BD1A70000004801}UI/layouts/Session/Briefing/TBD_TopicNavItem.layout"; //!< one topic navigation item

	// Players panel: a briefing mode beside the primary nav, `TBD_PlayersPanel`.
	static const ResourceName PLAYERS_PANEL  = "{7BD1A70000004301}UI/layouts/Session/Shared/TBD_PlayersPanel.layout"; //!< players panel: one column per faction
	static const ResourceName PLAYERS_LANE   = "{7BD1A70000004401}UI/layouts/Session/Shared/TBD_PlayerLane.layout"; //!< one faction column of the players panel
	static const ResourceName PLAYERS_ROW    = "{7BD1A70000004501}UI/layouts/Session/Shared/TBD_PlayerRow.layout"; //!< one player row of a faction column

	static const ResourceName OBJECTIVE_HUD      = "{7BD1A70000000A01}UI/layouts/Hud/TBD_ObjectiveHud.layout"; //!< Objective list and capture bar. `TBD_ObjectiveHud`.

	static const ResourceName END_SCREEN     = "{7BD1A70000000801}UI/layouts/Session/PostGame/TBD_EndScreen.layout"; //!< END stage banner: winning faction + reason.
	static const ResourceName DEBRIEF_SCREEN = "{7BD1A70000000901}UI/layouts/Session/PostGame/TBD_DebriefScreen.layout"; //!< DEBRIEF stage scoreboard.

	//! Put one of OUR textures (the constants above) on an image widget. False = not found (not
	//! imported yet, rdb stale): the widget is hidden so no white quad shows, and the miss is logged
	//! once per texture.
	static bool LoadTexture(ImageWidget w, ResourceName texture)
	{
		if (!w)
			return false;

		if (!texture.IsEmpty() && w.LoadImageTexture(0, texture))
		{
			w.SetImage(0);
			w.SetVisible(true);
			return true;
		}

		w.SetVisible(false);
		if (!s_aMissingTextures)
			s_aMissingTextures = {};

		if (!s_aMissingTextures.Contains(texture))
		{
			s_aMissingTextures.Insert(texture);
			Print(string.Format("[TBD][ui] texture not found: %1 (import it in Workbench, then pin its GUID in TBD_UILayouts)", texture), LogLevel.WARNING);
		}

		return false;
	}

	protected static ref array<string> s_aMissingTextures; //!< textures already logged as missing; created on the first miss

	//! Instantiate a layout under `parent`, retrying without the GUID prefix if the GUID does not
	//! resolve. Returns null without a workspace or for an unresolvable layout; every caller
	//! handles null.
	static Widget Create(ResourceName layout, Widget parent)
	{
		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return null; // no workspace: nothing to draw on

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

	//! Fill a `Border` / `Background` frame dock with the rounded shape nearest `radius`
	//! (TBD_UITheme.RADIUS_*). The dock keeps its name and is what the handler paints; the seven
	//! images inside inherit that colour. A dock that is not a FrameWidget (a layout that was not
	//! converted) is left alone and keeps rendering square, so the call is always safe. Any
	//! previous shape in the dock is removed first, so a chip can switch tag <-> pill.
	static Widget MountRounded(Widget dock, int radius)
	{
		if (!dock || !FrameWidget.Cast(dock))
			return null;

		Clear(dock);

		ResourceName layout;
		switch (radius)
		{
			case 12: layout = ROUNDED_12; break;
			case 11: layout = ROUNDED_11; break;
			case 10: layout = ROUNDED_10; break;
			case 9:  layout = ROUNDED_9;  break;
			case 7:  layout = ROUNDED_7;  break;
			case 6:  layout = ROUNDED_6;  break;
			case 5:  layout = ROUNDED_5;  break;
			default: layout = ROUNDED_8;  break;
		}

		return Create(layout, dock);
	}

	//! Create() and pin the new root to the full width of a layout-widget parent. Every row, card
	//! and chip that lands in a Vertical/HorizontalLayout goes through here, because a root slot
	//! exists only once the widget has a parent, so the layout file alone cannot set the stretch.
	static Widget CreateStretched(ResourceName layout, Widget parent)
	{
		Widget created = Create(layout, parent);
		if (created)
			AlignableSlot.SetHorizontalAlign(created, LayoutHorizontalAlign.Stretch);

		return created;
	}

	//! Create() and pull one of our handlers off the new root in a single step. Null when either
	//! half fails; the half-built widget is removed so a screen never keeps an unbound stub.
	static ScriptedWidgetComponent CreateHandler(ResourceName layout, Widget parent, typename handler)
	{
		Widget created = Create(layout, parent);
		if (!created)
			return null;

		ScriptedWidgetComponent found = ScriptedWidgetComponent.Cast(created.FindHandler(handler));
		if (!found)
		{
			Print(string.Format("[TBD][ui] layout %1 carries no %2 handler", StripGuid(layout), handler), LogLevel.ERROR);
			created.RemoveFromHierarchy();
			return null;
		}

		return found;
	}

	//! Remove every child of `parent`. Panels that rebuild small lists on selection use this instead
	//! of a pool; lists that refresh on replication must pool (see TBD_ListBox).
	static void Clear(Widget parent)
	{
		if (!parent)
			return;

		Widget child = parent.GetChildren();
		while (child)
		{
			Widget next = child.GetSibling();
			child.RemoveFromHierarchy();
			child = next;
		}
	}

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
