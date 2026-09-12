//! TBD Mission Selector UI -- Screen Controller and Interactive Components.
//! Provides a 1:1 translation of the Stitch Google mockup ("Reforger Dark Tactical" workstation)
//! with rich mock data and interactive navigation between terrains, mission cards, and the inspector.

// -- Terrain Row Component -------------------------------------------------------------------
class TBD_TerrainRowComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected ImageWidget m_wBG;
	protected ImageWidget m_wAccent;
	protected TextWidget m_wName;
	protected TextWidget m_wBadge;

	protected int m_iIndex;
	protected bool m_bSelected;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBG = ImageWidget.Cast(w.FindAnyWidget("RowBG"));
		m_wAccent = ImageWidget.Cast(w.FindAnyWidget("RowAccent"));
		m_wName = TextWidget.Cast(w.FindAnyWidget("TerrainName"));
		m_wBadge = TextWidget.Cast(w.FindAnyWidget("TerrainBadge"));
	}

	//------------------------------------------------------------------------------------------------
	void Init(int index, TBD_MockTerrain terrain, bool selected)
	{
		m_iIndex = index;
		if (m_wName)
			m_wName.SetText(terrain.m_sName);

		if (m_wBadge)
		{
			if (!terrain.m_sBadge.IsEmpty())
			{
				m_wBadge.SetText(terrain.m_sBadge);
				TBD_UITheme.Paint(m_wBadge, TBD_UITheme.PRIMARY);
			}
			else
			{
				m_wBadge.SetText(terrain.m_iCount.ToString());
				TBD_UITheme.Paint(m_wBadge, TBD_UITheme.ON_SURFACE_VARIANT);
			}
		}

		SetSelected(selected);
	}

	//------------------------------------------------------------------------------------------------
	void SetSelected(bool selected)
	{
		m_bSelected = selected;
		if (!m_wRoot)
			return;

		Widget chevron = m_wRoot.FindAnyWidget("TerrainChevron");
		if (chevron)
			chevron.SetVisible(selected);

		ImageWidget badgeBG = ImageWidget.Cast(m_wRoot.FindAnyWidget("BadgeBG"));

		if (selected)
		{
			if (m_wBG)
				TBD_UITheme.Paint(m_wBG, TBD_UITheme.SURFACE_CONTAINER_HIGH);
			if (m_wAccent)
				TBD_UITheme.Paint(m_wAccent, TBD_UITheme.CARD_BORDER);
			if (m_wName)
				TBD_UITheme.Paint(m_wName, TBD_UITheme.ON_SURFACE);
			if (badgeBG)
				TBD_UITheme.Paint(badgeBG, TBD_UITheme.SURFACE_CONTAINER_HIGHEST);
		}
		else
		{
			if (m_wBG)
				TBD_UITheme.Paint(m_wBG, TBD_UITheme.TRANSPARENT);
			if (m_wAccent)
				TBD_UITheme.Paint(m_wAccent, TBD_UITheme.TRANSPARENT);
			if (m_wName)
				TBD_UITheme.Paint(m_wName, TBD_UITheme.ON_SURFACE_VARIANT);
			if (badgeBG)
				TBD_UITheme.Paint(badgeBG, TBD_UITheme.TRANSPARENT);
		}
	}

	//------------------------------------------------------------------------------------------------
	override bool OnClick(Widget w, int x, int y, int button)
	{
		TBD_MissionSelectorComponent comp = TBD_MissionSelectorComponent.GetInstance();
		if (comp)
			comp.SelectTerrain(m_iIndex);
		return true;
	}
}

// -- Mission Card Component ------------------------------------------------------------------
class TBD_MissionCardComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected ImageWidget m_wBG;
	protected ImageWidget m_wAccent;
	protected TextWidget m_wTag;
	protected ImageWidget m_wTagBG;
	protected TextWidget m_wSlots;
	protected TextWidget m_wTitle;
	protected TextWidget m_wSubtitle;

	protected int m_iIndex;
	protected bool m_bSelected;

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBG = ImageWidget.Cast(w.FindAnyWidget("CardBG"));
		m_wAccent = ImageWidget.Cast(w.FindAnyWidget("CardAccent"));
		m_wTag = TextWidget.Cast(w.FindAnyWidget("TagPill"));
		m_wTagBG = ImageWidget.Cast(w.FindAnyWidget("TagPillBG"));
		m_wSlots = TextWidget.Cast(w.FindAnyWidget("SlotCount"));
		m_wTitle = TextWidget.Cast(w.FindAnyWidget("TitleText"));
		m_wSubtitle = TextWidget.Cast(w.FindAnyWidget("SubtitleText"));
	}

	//------------------------------------------------------------------------------------------------
	void Init(int index, TBD_MockMission mission, bool selected)
	{
		m_iIndex = index;
		if (m_wTag)
			m_wTag.SetText(mission.m_sTag);

		if (m_wTagBG)
		{
			if (mission.m_sTag == "WARLORDS")
			{
				TBD_UITheme.Paint(m_wTagBG, TBD_UITheme.PRIMARY_CONTAINER);
				if (m_wTag) TBD_UITheme.Paint(m_wTag, TBD_UITheme.ON_ACTION);
			}
			else if (mission.m_sTag == "PvP" || mission.m_sTag == "VANGUARD")
			{
				TBD_UITheme.Paint(m_wTagBG, TBD_UITheme.TERTIARY_CONTAINER);
				if (m_wTag) TBD_UITheme.Paint(m_wTag, TBD_UITheme.TERTIARY_WARM);
			}
			else if (mission.m_sTag == "RHS")
			{
				TBD_UITheme.Paint(m_wTagBG, TBD_UITheme.SURFACE_CONTAINER_HIGH);
				if (m_wTag) TBD_UITheme.Paint(m_wTag, TBD_UITheme.CARD_BORDER);
			}
			else if (mission.m_sTag == "ZEUS")
			{
				TBD_UITheme.Paint(m_wTagBG, TBD_UITheme.SURFACE_CONTAINER_HIGHEST);
				if (m_wTag) TBD_UITheme.Paint(m_wTag, TBD_UITheme.PRIMARY);
			}
			else
			{
				TBD_UITheme.Paint(m_wTagBG, TBD_UITheme.SURFACE_CONTAINER_HIGH);
				if (m_wTag) TBD_UITheme.Paint(m_wTag, TBD_UITheme.ON_SURFACE_VARIANT);
			}
		}

		if (m_wSlots)
			m_wSlots.SetText(string.Format("%1 SLOTS", mission.m_iSlots));

		if (m_wTitle)
			m_wTitle.SetText(mission.m_sTitle);

		if (m_wSubtitle)
			m_wSubtitle.SetText(mission.m_sSubtitle);

		SetSelected(selected);
	}

	//------------------------------------------------------------------------------------------------
	void SetSelected(bool selected)
	{
		m_bSelected = selected;
		if (!m_wRoot)
			return;

		Widget dot = m_wRoot.FindAnyWidget("ActiveDot");
		Widget chevron = m_wRoot.FindAnyWidget("CardChevron");
		if (dot)
			dot.SetVisible(selected);
		if (chevron)
			chevron.SetVisible(!selected);

		if (selected)
		{
			if (m_wBG)
				TBD_UITheme.Paint(m_wBG, TBD_UITheme.SURFACE_CONTAINER_HIGHEST);
			if (m_wAccent)
				TBD_UITheme.Paint(m_wAccent, TBD_UITheme.CARD_BORDER);
			if (m_wTitle)
				TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_ACTION);
		}
		else
		{
			if (m_wBG)
				TBD_UITheme.Paint(m_wBG, TBD_UITheme.SURFACE_CONTAINER_LOW);
			if (m_wAccent)
				TBD_UITheme.Paint(m_wAccent, TBD_UITheme.TRANSPARENT);
			if (m_wTitle)
				TBD_UITheme.Paint(m_wTitle, TBD_UITheme.ON_SURFACE);
		}
	}

	//------------------------------------------------------------------------------------------------
	override bool OnClick(Widget w, int x, int y, int button)
	{
		TBD_MissionSelectorComponent comp = TBD_MissionSelectorComponent.GetInstance();
		if (comp)
			comp.SelectMission(m_iIndex);
		return true;
	}
}

// -- Main Screen Component -------------------------------------------------------------------
class TBD_MissionSelectorComponent : ScriptedWidgetComponent
{
	protected static const ResourceName LAYOUT_PATH = TBD_UILayouts.MISSION_SELECTOR;
	protected static const ResourceName TERRAIN_ROW_LAYOUT = TBD_UILayouts.MISSION_SELECTOR_TERRAIN_ROW;
	protected static const ResourceName MISSION_CARD_LAYOUT = TBD_UILayouts.MISSION_SELECTOR_MISSION_CARD;

	protected static Widget s_wRoot;
	protected static TBD_MissionSelectorComponent s_Instance;

	// Top Bar
	protected TextWidget m_wHeaderTitle;
	protected TextWidget m_wAdminLabel;
	protected TextWidget m_wPlayerCount;

	// Column 1: Terrains
	protected TextWidget m_wTerrainsTitle;
	protected TextWidget m_wTerrainsCount;
	protected EditBoxWidget m_wTerrainSearch;
	protected VerticalLayoutWidget m_wTerrainListContent;
	protected ref array<TBD_TerrainRowComponent> m_aTerrainRowComponents;

	// Column 2: Missions
	protected TextWidget m_wMissionsTitle;
	protected TextWidget m_wMissionsAvailableCount;
	protected EditBoxWidget m_wMissionSearch;
	protected VerticalLayoutWidget m_wMissionListContent;
	protected ref array<TBD_MissionCardComponent> m_aMissionCardComponents;
	protected ref array<Widget> m_aFilterChipButtons;

	// Column 3: Inspector
	protected TextWidget m_wInspectorHeroTitle;
	protected TextWidget m_wInspectorHeroMeta;
	protected TextWidget m_wHeroTag;
	protected TextWidget m_wHeroVersion;
	protected TextWidget m_wInspectorPlayers;
	protected TextWidget m_wInspectorRespawn;
	protected TextWidget m_wInspectorDifficulty;
	protected TextWidget m_wInspectorSkill;
	protected TextWidget m_wInspectorSynopsis;

	// Footer Actions
	protected ButtonWidget m_wBackAction;
	protected ButtonWidget m_wGameOptionsAction;
	protected ButtonWidget m_wPlayAction;

	// State
	protected ref array<ref TBD_MockTerrain> m_aTerrains;
	protected ref array<ref TBD_MockMission> m_aMissions;
	protected int m_iSelectedTerrainIndex = 1; // Altis
	protected int m_iSelectedMissionIndex = 0; // SC 48 Warlords
	protected string m_sActiveTagFilter = "ALL";

	//------------------------------------------------------------------------------------------------
	static TBD_MissionSelectorComponent GetInstance()
	{
		return s_Instance;
	}

	//------------------------------------------------------------------------------------------------
	static bool IsOpen()
	{
		return s_wRoot != null;
	}

	//------------------------------------------------------------------------------------------------
	//! Open the mission selector on the local client workspace.
	static void Open()
	{
		if (s_wRoot)
			return;

		WorkspaceWidget workspace = GetGame().GetWorkspace();
		if (!workspace)
			return;

		Widget root = TBD_UILayouts.Create(LAYOUT_PATH, null);

		if (!root)
		{
			Print("[TBD][selector] Failed to instantiate TBD_MissionSelector.layout", LogLevel.ERROR);
			return;
		}

		s_wRoot = root;

		InputManager im = GetGame().GetInputManager();
		if (im)
			im.ActivateContext("MenuContext");

		Print("[TBD][selector] Mission Selector opened successfully.");
	}

	//------------------------------------------------------------------------------------------------
	//! Close the mission selector and clean up the workspace.
	static void Close()
	{
		if (!s_wRoot)
			return;

		s_wRoot.RemoveFromHierarchy();
		s_wRoot = null;
		s_Instance = null;
		Print("[TBD][selector] Mission Selector closed.");
	}

	//------------------------------------------------------------------------------------------------
	//! Toggle open/closed state.
	static void Toggle()
	{
		if (IsOpen())
			Close();
		else
			Open();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		s_Instance = this;
		s_wRoot = w;

		// Bind Header
		m_wHeaderTitle = TextWidget.Cast(w.FindAnyWidget("HeaderTitle"));
		m_wAdminLabel = TextWidget.Cast(w.FindAnyWidget("AdminLabel"));
		m_wPlayerCount = TextWidget.Cast(w.FindAnyWidget("PlayerCountText"));

		// Bind Col 1
		m_wTerrainsTitle = TextWidget.Cast(w.FindAnyWidget("TerrainsTitle"));
		m_wTerrainsCount = TextWidget.Cast(w.FindAnyWidget("TerrainsCount"));
		m_wTerrainSearch = EditBoxWidget.Cast(w.FindAnyWidget("TerrainSearch"));
		m_wTerrainListContent = VerticalLayoutWidget.Cast(w.FindAnyWidget("TerrainListContent"));

		// Bind Col 2
		m_wMissionsTitle = TextWidget.Cast(w.FindAnyWidget("MissionsTitle"));
		m_wMissionsAvailableCount = TextWidget.Cast(w.FindAnyWidget("MissionsAvailableCount"));
		m_wMissionSearch = EditBoxWidget.Cast(w.FindAnyWidget("MissionSearch"));
		m_wMissionListContent = VerticalLayoutWidget.Cast(w.FindAnyWidget("MissionListContent"));

		// Bind Filter Chips
		m_aFilterChipButtons = {};
		Widget chipAll = w.FindAnyWidget("ChipAll");
		Widget chipCoop = w.FindAnyWidget("ChipCoop");
		Widget chipWarlords = w.FindAnyWidget("ChipWarlords");
		Widget chipPvp = w.FindAnyWidget("ChipPvp");
		Widget chipRhs = w.FindAnyWidget("ChipRhs");
		Widget chipZeus = w.FindAnyWidget("ChipZeus");
		if (chipAll) m_aFilterChipButtons.Insert(chipAll);
		if (chipCoop) m_aFilterChipButtons.Insert(chipCoop);
		if (chipWarlords) m_aFilterChipButtons.Insert(chipWarlords);
		if (chipPvp) m_aFilterChipButtons.Insert(chipPvp);
		if (chipRhs) m_aFilterChipButtons.Insert(chipRhs);
		if (chipZeus) m_aFilterChipButtons.Insert(chipZeus);

		// Bind Col 3 Inspector
		m_wInspectorHeroTitle = TextWidget.Cast(w.FindAnyWidget("InspectorHeroTitle"));
		m_wInspectorHeroMeta = TextWidget.Cast(w.FindAnyWidget("InspectorHeroMeta"));
		m_wHeroTag = TextWidget.Cast(w.FindAnyWidget("HeroTag"));
		m_wHeroVersion = TextWidget.Cast(w.FindAnyWidget("HeroVersion"));
		m_wInspectorPlayers = TextWidget.Cast(w.FindAnyWidget("InspectorPlayers"));
		m_wInspectorRespawn = TextWidget.Cast(w.FindAnyWidget("InspectorRespawn"));
		m_wInspectorDifficulty = TextWidget.Cast(w.FindAnyWidget("InspectorDifficulty"));
		m_wInspectorSkill = TextWidget.Cast(w.FindAnyWidget("InspectorSkill"));
		m_wInspectorSynopsis = TextWidget.Cast(w.FindAnyWidget("InspectorSynopsis"));

		// Bind Footer
		m_wBackAction = ButtonWidget.Cast(w.FindAnyWidget("BackAction"));
		m_wGameOptionsAction = ButtonWidget.Cast(w.FindAnyWidget("GameOptionsAction"));
		m_wPlayAction = ButtonWidget.Cast(w.FindAnyWidget("PlayAction"));

		// Load Mock Data
		m_aTerrains = TBD_MissionSelectorData.GetMockTerrains();
		m_aMissions = TBD_MissionSelectorData.GetMockMissions();

		PopulateTerrains();
		PopulateMissions();
		UpdateInspector();
	}

	//------------------------------------------------------------------------------------------------
	override void HandlerDeattached(Widget w)
	{
		if (s_wRoot == w)
			s_wRoot = null;
		if (s_Instance == this)
			s_Instance = null;
		super.HandlerDeattached(w);
	}

	//------------------------------------------------------------------------------------------------
	protected void PopulateTerrains()
	{
		if (!m_wTerrainListContent)
			return;

		m_aTerrainRowComponents = {};
		WorkspaceWidget ws = GetGame().GetWorkspace();
		if (!ws)
			return;

		int count = m_aTerrains.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			Widget rowWidget = TBD_UILayouts.Create(TERRAIN_ROW_LAYOUT, m_wTerrainListContent);
			if (!rowWidget)
				continue;

			TBD_TerrainRowComponent comp = new TBD_TerrainRowComponent();
			rowWidget.AddHandler(comp);
			comp.Init(i, m_aTerrains[i], i == m_iSelectedTerrainIndex);
			m_aTerrainRowComponents.Insert(comp);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void PopulateMissions()
	{
		if (!m_wMissionListContent)
			return;

		// Clear previous cards
		Widget child = m_wMissionListContent.GetChildren();
		while (child)
		{
			Widget next = child.GetSibling();
			child.RemoveFromHierarchy();
			child = next;
		}

		m_aMissionCardComponents = {};
		WorkspaceWidget ws = GetGame().GetWorkspace();
		if (!ws)
			return;

		int visibleCount = 0;
		int count = m_aMissions.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			TBD_MockMission mission = m_aMissions[i];
			if (m_sActiveTagFilter != "ALL" && mission.m_sTag != m_sActiveTagFilter)
				continue;

			Widget cardWidget = TBD_UILayouts.Create(MISSION_CARD_LAYOUT, m_wMissionListContent);
			if (!cardWidget)
				continue;

			TBD_MissionCardComponent comp = new TBD_MissionCardComponent();
			cardWidget.AddHandler(comp);
			comp.Init(i, mission, i == m_iSelectedMissionIndex);
			m_aMissionCardComponents.Insert(comp);
			visibleCount++;
		}

		if (m_wMissionsAvailableCount)
			m_wMissionsAvailableCount.SetText(string.Format("%1 AVAILABLE", visibleCount));
	}

	//------------------------------------------------------------------------------------------------
	void SelectTerrain(int index)
	{
		if (index < 0 || index >= m_aTerrains.Count())
			return;

		m_iSelectedTerrainIndex = index;
		TBD_MockTerrain terrain = m_aTerrains[index];

		if (m_wMissionsTitle)
		{
			string terrainUpper = terrain.m_sName;
			terrainUpper.ToUpper();
			m_wMissionsTitle.SetText(string.Format("%1 MISSIONS", terrainUpper));
		}

		int count = m_aTerrainRowComponents.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			m_aTerrainRowComponents[i].SetSelected(i == index);
		}
	}

	//------------------------------------------------------------------------------------------------
	void SelectMission(int index)
	{
		if (index < 0 || index >= m_aMissions.Count())
			return;

		m_iSelectedMissionIndex = index;

		int count = m_aMissionCardComponents.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			m_aMissionCardComponents[i].SetSelected(i == index);
		}

		UpdateInspector();
	}

	//------------------------------------------------------------------------------------------------
	protected void UpdateInspector()
	{
		if (m_iSelectedMissionIndex < 0 || m_iSelectedMissionIndex >= m_aMissions.Count())
			return;

		TBD_MockMission mission = m_aMissions[m_iSelectedMissionIndex];

		if (m_wInspectorHeroTitle)
			m_wInspectorHeroTitle.SetText(mission.m_sTitle);

		if (m_wInspectorHeroMeta)
		{
			string mapUpper = mission.m_sTerrain;
			mapUpper.ToUpper();
			m_wInspectorHeroMeta.SetText(string.Format("%1  -  MAP: %2 (ISLAND)  -  SECTORS: 18 CONTESTED", mission.m_sAuthor, mapUpper));
		}

		if (m_wHeroTag)
			m_wHeroTag.SetText(mission.m_sTag);

		if (m_wHeroVersion)
			m_wHeroVersion.SetText(mission.m_sVersion);

		if (m_wInspectorPlayers)
			m_wInspectorPlayers.SetText(string.Format("%1 SLOTS", mission.m_sPlayersRange));

		if (m_wInspectorRespawn)
			m_wInspectorRespawn.SetText(mission.m_sRespawnRule);

		if (m_wInspectorDifficulty)
			m_wInspectorDifficulty.SetText(mission.m_sDifficulty);

		if (m_wInspectorSkill)
			m_wInspectorSkill.SetText(mission.m_sAiSkill);

		if (m_wInspectorSynopsis)
			m_wInspectorSynopsis.SetText(mission.m_sSynopsis);
	}

	//------------------------------------------------------------------------------------------------
	void SetTagFilter(string tag, Widget activeChip)
	{
		m_sActiveTagFilter = tag;

		int chipCount = m_aFilterChipButtons.Count();
		int i;
		for (i = 0; i < chipCount; i++)
		{
			Widget chip = m_aFilterChipButtons[i];
			Widget chipBG = chip.FindAnyWidget(chip.GetName() + "BG");
			Widget chipText = chip.FindAnyWidget(chip.GetName() + "Text");
			if (chip == activeChip)
			{
				if (chipBG)
					TBD_UITheme.Paint(chipBG, TBD_UITheme.PRIMARY_CONTAINER);
				if (chipText)
					TBD_UITheme.Paint(chipText, TBD_UITheme.ON_ACTION);
			}
			else
			{
				if (chipBG)
					TBD_UITheme.Paint(chipBG, TBD_UITheme.SURFACE_CONTAINER_HIGH);
				if (chipText)
					TBD_UITheme.Paint(chipText, TBD_UITheme.ON_SURFACE_VARIANT);
			}
		}

		PopulateMissions();
	}

	//------------------------------------------------------------------------------------------------
	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (w == m_wBackAction)
		{
			Close();
			return true;
		}

		if (w == m_wGameOptionsAction)
		{
			Print("[TBD][selector] GAME OPTIONS clicked.");
			return true;
		}

		if (w == m_wPlayAction)
		{
			if (m_iSelectedMissionIndex >= 0 && m_iSelectedMissionIndex < m_aMissions.Count())
			{
				TBD_MockMission selected = m_aMissions[m_iSelectedMissionIndex];
				Print(string.Format("[TBD][selector] PLAY selected mission: %1", selected.m_sTitle));
			}
			return true;
		}

		string name = w.GetName();
		if (name == "ChipAll") { SetTagFilter("ALL", w); return true; }
		if (name == "ChipCoop") { SetTagFilter("COOP", w); return true; }
		if (name == "ChipWarlords") { SetTagFilter("WARLORDS", w); return true; }
		if (name == "ChipPvp") { SetTagFilter("PvP", w); return true; }
		if (name == "ChipRhs") { SetTagFilter("RHS", w); return true; }
		if (name == "ChipZeus") { SetTagFilter("ZEUS", w); return true; }

		return false;
	}
}

// -- Chimera Menu Preset Wrapper -------------------------------------------------------------
class TBD_MissionSelectorScreen : TBD_MenuBase
{
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();
		Print("[TBD][selector] TBD_MissionSelectorScreen opened via ChimeraMenu stack.");
	}

	override protected void OnScreenClose()
	{
		Print("[TBD][selector] TBD_MissionSelectorScreen closed via ChimeraMenu stack.");
		super.OnScreenClose();
	}
}

modded enum ChimeraMenuPreset
{
	TBD_UIMissionSelector
}
