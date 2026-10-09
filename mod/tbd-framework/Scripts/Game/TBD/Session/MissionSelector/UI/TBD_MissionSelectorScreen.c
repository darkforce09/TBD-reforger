/**
 * @file TBD_MissionSelectorScreen.c
 * @brief The Mission Selector screen: docks the terrains, missions and inspector columns and wires them.
 *
 * Role: mounts the three column controllers and chains terrain pick -> mission list -> inspector
 * through their invokers; Select Scenario records the pick in TBD_SessionSelection.  Position: opened through TBD_MenuStack as preset `TBD_UIMissionSelector`, bound in
 * `Configs/System/chimeraMenus.conf` to `TBD_MissionSelector.layout`; the `TBD_MissionSelector`
 * key (F9) on SCR_PlayerController calls Toggle; data comes from TBD_MissionCatalog.Get().
 * State: the three column controllers and the catalog, owned by the screen on the client.
 * Invariants: the screen touches no widget by name outside its docks; Select Scenario is enabled
 * only while a mission is selected.
 *
 *   TopDock     TBD_SessionTopBar        "SCENARIO BROWSER", tabs, identity
 *   LeftDock    TBD_PanelFill + TBD_TerrainSelectorPanel   TERRAINS
 *   CenterDock  TBD_PanelFill + TBD_ScenarioBrowserPanel   <TERRAIN> MISSIONS
 *   RightDock   TBD_MissionInspector + TBD_MissionInspectorPanel
 *   BottomDock  TBD_SessionBottomBar     [ Select Scenario ]
 *   OverlayDock popovers (Modes, version)
 */

//! The Mission Selector dock screen.
class TBD_MissionSelectorScreen : TBD_DockScreen
{
	protected ref TBD_TerrainSelectorPanel m_Terrains; //!< TERRAINS column
	protected ref TBD_ScenarioBrowserPanel m_Browser; //!< missions column
	protected ref TBD_MissionInspectorPanel m_Inspector; //!< inspector column
	protected TBD_MissionCatalog m_Catalog; //!< the catalog in force

	static const string ACTION_SELECT = "select_scenario"; //!< bottom-bar action id of Select Scenario

	//! Raise or drop the screen through the menu stack.
	static void Toggle()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UIMissionSelector))
		{
			TBD_MenuStack.Close(ChimeraMenuPreset.TBD_UIMissionSelector);
			return;
		}

		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIMissionSelector);
	}

	//! Mount the three columns, wire their invokers, add Select Scenario, and select the first
	//! terrain so the cascade fills the browser and the inspector.
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

	//! Unwire and destroy the three columns.
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

	//! Focus lands on the selected terrain row: the next click is "pick a map" or "pick a card".
	override void FocusDefault()
	{
		if (m_Terrains && m_Terrains.FocusSelected())
			return;

		super.FocusDefault();
	}

	//! @return the top-bar title
	override protected string GetScreenTitle()
	{
		return "Scenario Browser";
	}

	//! @return the session tab this screen highlights
	override protected int GetSessionTab()
	{
		return TBD_ESessionTab.SCENARIO_BROWSER;
	}

	//! @return the catalog's session identity for the bars, or null
	override protected TBD_SessionIdentity GetSessionIdentity()
	{
		if (!m_Catalog)
			return null;

		return m_Catalog.GetIdentity();
	}

	//! Select Scenario: record the selected mission and version in TBD_SessionSelection and log the
	//! pick; a press with nothing selected logs a WARNING.
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
		Print(string.Format("[TBD][selector] SELECT SCENARIO %1 (%2) on %3, version %4", mission.m_sTitle, mission.m_sId, mission.m_sTerrainKey, m_Inspector.GetSelectedVersionLabel()));
	}

	//! Point the browser at the picked terrain.
	protected void OnTerrainSelected(TBD_TerrainSelectorPanel panel, string terrainKey)
	{
		if (m_Browser)
			m_Browser.SetTerrain(terrainKey);
	}

	//! Show the picked mission in the inspector and enable Select Scenario when there is one.
	protected void OnMissionSelected(TBD_ScenarioBrowserPanel panel, string missionId)
	{
		TBD_MissionSummary mission = m_Catalog.FindMission(missionId);

		if (m_Inspector)
			m_Inspector.Show(mission);

		if (m_BottomBar)
			m_BottomBar.SetActionEnabled(ACTION_SELECT, mission != null);
	}

	//! Log a version pick.
	protected void OnVersionChanged(TBD_MissionInspectorPanel panel, int versionIndex)
	{
		Print(string.Format("[TBD][selector] version -> %1", panel.GetSelectedVersionLabel()));
	}

	//! @return the browser's selected mission, or null
	protected TBD_MissionSummary SelectedMission()
	{
		if (!m_Browser || !m_Catalog)
			return null;

		return m_Catalog.FindMission(m_Browser.GetSelectedId());
	}
}
