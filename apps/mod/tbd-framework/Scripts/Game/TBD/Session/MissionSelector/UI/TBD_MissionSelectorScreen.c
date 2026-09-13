//! Pre-game rebuild (2026-09-12) — the Mission Selector, first Dock & Sub-Layout screen.
//!
//! ```
//!   TopDock     TBD_SessionTopBar        "SCENARIO BROWSER" · tabs · Mission Maker ADMIN · 👥 1
//!   LeftDock    TBD_PanelFill + TBD_TerrainSelectorPanel   TERRAINS
//!   CenterDock  TBD_PanelFill + TBD_ScenarioBrowserPanel   <TERRAIN> MISSIONS
//!   RightDock   TBD_MissionInspector + TBD_MissionInspectorPanel
//!   BottomDock  TBD_SessionBottomBar     [ Select Scenario ]
//!   OverlayDock popovers (Modes, version)
//! ```
//!
//! The screen owns wiring and nothing else: terrain -> browser -> inspector is three invokers.
//! Data comes from `TBD_MissionCatalog.Get()` (mock today, client cache later); this file never
//! touches a widget by name outside the docks.
//!
//! Opened through `TBD_MenuStack` (preset `TBD_UIMissionSelector`, bound in
//! `Configs/System/chimeraMenus.conf` to `TBD_MissionSelector.layout` — the shell that replaced
//! the 2736-line monolith at the same GUID). `TBD_MissionBrowser` toggles it on F6 / F9.
class TBD_MissionSelectorScreen : TBD_DockScreen
{
	protected ref TBD_TerrainSelectorPanel m_Terrains;
	protected ref TBD_ScenarioBrowserPanel m_Browser;
	protected ref TBD_MissionInspectorPanel m_Inspector;
	protected TBD_MissionCatalog m_Catalog;

	static const string ACTION_SELECT = "select_scenario";

	//------------------------------------------------------------------------------------------------
	//! F6 / F9: raise or drop the screen through the stack.
	static void Toggle()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIMissionSelector))
		{
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UIMissionSelector);
			return;
		}

		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIMissionSelector);
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		m_Catalog = TBD_MissionCatalog.Get();
		super.OnScreenOpen(); // bars are up after this; they read GetSessionIdentity()

		Widget overlay = GetOverlayDock();

		m_Terrains = new TBD_TerrainSelectorPanel();
		if (m_Terrains.Build(Mount("LeftDock", TBD_UILayouts.PANEL_FILL), m_Catalog))
			m_Terrains.GetOnSelected().Insert(OnTerrainSelected);

		m_Browser = new TBD_ScenarioBrowserPanel();
		if (m_Browser.Build(Mount("CenterDock", TBD_UILayouts.PANEL_FILL), overlay, m_Catalog))
			m_Browser.GetOnSelected().Insert(OnMissionSelected);

		m_Inspector = new TBD_MissionInspectorPanel();
		if (m_Inspector.Build(Mount("RightDock", TBD_UILayouts.MISSION_SELECTOR_INSPECTOR), overlay, m_Catalog))
			m_Inspector.GetOnVersionChanged().Insert(OnVersionChanged);

		if (m_BottomBar)
			m_BottomBar.AddAction(ACTION_SELECT, "Select Scenario", true);

		// First terrain in catalog order; the cascade fills the browser and the inspector.
		m_Terrains.Select(m_Terrains.GetFirstKey(), true);

		Print("[TBD][selector] Mission Selector opened (dock shell).");
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		if (m_Terrains)
		{
			m_Terrains.GetOnSelected().Remove(OnTerrainSelected);
			m_Terrains.Destroy();
		}

		if (m_Browser)
		{
			m_Browser.GetOnSelected().Remove(OnMissionSelected);
			m_Browser.Destroy();
		}

		if (m_Inspector)
		{
			m_Inspector.GetOnVersionChanged().Remove(OnVersionChanged);
			m_Inspector.Destroy();
		}

		m_Terrains = null;
		m_Browser = null;
		m_Inspector = null;

		Print("[TBD][selector] Mission Selector closed.");
		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	//! Focus lands on the selected terrain row: the next click is "pick a map" or "pick a card".
	override void FocusDefault()
	{
		if (m_Terrains && m_Terrains.FocusSelected())
			return;

		super.FocusDefault();
	}

	// ── TBD_DockScreen hooks ────────────────────────────────────────────────────────────────

	override protected string GetScreenTitle()
	{
		return "Scenario Browser";
	}

	override protected int GetSessionTab()
	{
		return TBD_ESessionTab.SCENARIO_BROWSER;
	}

	override protected TBD_SessionIdentity GetSessionIdentity()
	{
		if (!m_Catalog)
			return null;

		return m_Catalog.GetIdentity();
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnBottomAction(TBD_SessionBottomBar bar, string id)
	{
		if (id != ACTION_SELECT)
			return;

		TBD_MissionSummary mission = SelectedMission();
		if (!mission)
		{
			Print("[TBD][selector] Select Scenario pressed with nothing selected.", LogLevel.WARNING);
			return;
		}

		TBD_SessionSelection.Set(mission, m_Inspector.GetSelectedVersionLabel());
		// Mock pass: the intent is logged. The wire step hands this to TBD_MissionBrowser's load
		// request; nothing in this screen changes when it does.
		Print(string.Format("[TBD][selector] SELECT SCENARIO %1 (%2) on %3, version %4", mission.m_sTitle, mission.m_sId, mission.m_sTerrainKey, m_Inspector.GetSelectedVersionLabel()));
	}

	// ── Wiring ──────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnTerrainSelected(TBD_TerrainSelectorPanel panel, string terrainKey)
	{
		if (m_Browser)
			m_Browser.SetTerrain(terrainKey);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnMissionSelected(TBD_ScenarioBrowserPanel panel, string missionId)
	{
		TBD_MissionSummary mission = m_Catalog.FindMission(missionId);

		if (m_Inspector)
			m_Inspector.Show(mission);

		if (m_BottomBar)
			m_BottomBar.SetActionEnabled(ACTION_SELECT, mission != null);
	}

	//------------------------------------------------------------------------------------------------
	protected void OnVersionChanged(TBD_MissionInspectorPanel panel, int versionIndex)
	{
		Print(string.Format("[TBD][selector] version -> %1", panel.GetSelectedVersionLabel()));
	}

	//------------------------------------------------------------------------------------------------
	protected TBD_MissionSummary SelectedMission()
	{
		if (!m_Browser || !m_Catalog)
			return null;

		return m_Catalog.FindMission(m_Browser.GetSelectedId());
	}
}

modded enum ChimeraMenuPreset
{
	TBD_UIMissionSelector
}
