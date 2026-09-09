//! TBD Lobby Screen Controller -- 1:1 Parity with Stitch Reforger Dark Tactical Workstation Mockup.
//! Provides a 3-column split view: Left Sidebar (Factions, Observers, Voice/TFAR channels),
//! Center Master ORBAT Roster (Search, Filter Chips, Squad Cards, Roles), and Right Slot Inspector
//! (Role Specs, Vehicle, Primary/Secondary Weapons, Tactical Gear, Inventory & IFAK).

// -- Faction Row Component -------------------------------------------------------------------
class TBD_LobbyFactionRowComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected ImageWidget m_wBG;
	protected TextWidget m_wDot;
	protected TextWidget m_wName;
	protected ImageWidget m_wBadgeBG;
	protected TextWidget m_wBadge;

	protected int m_iIndex;
	protected bool m_bSelected;
	protected bool m_bIsFaction;

	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBG = ImageWidget.Cast(w.FindAnyWidget("RowBG"));
		m_wDot = TextWidget.Cast(w.FindAnyWidget("FactionDot"));
		m_wName = TextWidget.Cast(w.FindAnyWidget("FactionName"));
		m_wBadgeBG = ImageWidget.Cast(w.FindAnyWidget("BadgeBG"));
		m_wBadge = TextWidget.Cast(w.FindAnyWidget("FactionBadge"));
	}

	void Init(int index, string name, string countText, int dotColor, bool selected, bool isFaction = true)
	{
		m_iIndex = index;
		m_bIsFaction = isFaction;

		if (m_wName)
			m_wName.SetText(name);

		if (m_wBadge)
			m_wBadge.SetText(countText);

		if (m_wDot)
			m_wDot.SetColorInt(dotColor);

		SetSelected(selected);
	}

	void SetSelected(bool selected)
	{
		m_bSelected = selected;
		if (!m_wRoot)
			return;

		if (selected && m_bIsFaction)
		{
			if (m_wBG)
				m_wBG.SetColorInt(0x1F4D8EFF);
			if (m_wName)
				TBD_UITheme.Paint(m_wName, TBD_UITheme.ON_SURFACE);
			if (m_wBadgeBG)
				m_wBadgeBG.SetColorInt(0x554D8EFF);
			if (m_wBadge)
				TBD_UITheme.Paint(m_wBadge, TBD_UITheme.PRIMARY);
		}
		else
		{
			if (m_wBG)
				m_wBG.SetColorInt(0x00000000);
			if (m_wName)
				TBD_UITheme.Paint(m_wName, TBD_UITheme.ON_SURFACE_VARIANT);
			if (m_wBadgeBG)
				m_wBadgeBG.SetColorInt(0x00000000);
			if (m_wBadge)
				m_wBadge.SetColorInt(0x66FFFFFF);
		}
	}

	override bool OnClick(Widget w, int x, int y, int button)
	{
		if (!m_bIsFaction)
			return true;

		TBD_LobbyScreenComponent comp = TBD_LobbyScreenComponent.GetInstance();
		if (comp)
			comp.SelectFaction(m_iIndex);
		return true;
	}
}

// -- Slot Row Component ----------------------------------------------------------------------
class TBD_LobbySlotRowComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected ImageWidget m_wBG;
	protected ImageWidget m_wAccent;
	protected TextWidget m_wRoleIcon;
	protected TextWidget m_wRoleTitle;
	protected Widget m_wSpecialtyBadgeContainer;
	protected ImageWidget m_wSpecialtyBadgeBG;
	protected TextWidget m_wSpecialtyBadgeText;
	protected Widget m_wAiBadgeContainer;
	protected ImageWidget m_wAiBadgeBG;
	protected ImageWidget m_wAiBadgeBorder;
	protected TextWidget m_wAiBadgeText;
	protected ButtonWidget m_wClaimActionBtn;
	protected ImageWidget m_wClaimActionBG;
	protected ImageWidget m_wClaimActionBorder;
	protected TextWidget m_wClaimActionText;

	protected ref TBD_MockRoleLoadout m_Role;
	protected bool m_bSelected;

	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wBG = ImageWidget.Cast(w.FindAnyWidget("RowBG"));
		m_wAccent = ImageWidget.Cast(w.FindAnyWidget("RowAccent"));
		m_wRoleIcon = TextWidget.Cast(w.FindAnyWidget("RoleIcon"));
		m_wRoleTitle = TextWidget.Cast(w.FindAnyWidget("RoleTitle"));
		m_wSpecialtyBadgeContainer = w.FindAnyWidget("SpecialtyBadgeContainer");
		m_wSpecialtyBadgeBG = ImageWidget.Cast(w.FindAnyWidget("SpecialtyBadgeBG"));
		m_wSpecialtyBadgeText = TextWidget.Cast(w.FindAnyWidget("SpecialtyBadgeText"));
		m_wAiBadgeContainer = w.FindAnyWidget("AiBadgeContainer");
		m_wAiBadgeBG = ImageWidget.Cast(w.FindAnyWidget("AiBadgeBG"));
		m_wAiBadgeBorder = ImageWidget.Cast(w.FindAnyWidget("AiBadgeBorder"));
		m_wAiBadgeText = TextWidget.Cast(w.FindAnyWidget("AiBadgeText"));
		m_wClaimActionBtn = ButtonWidget.Cast(w.FindAnyWidget("ClaimActionBtn"));
		m_wClaimActionBG = ImageWidget.Cast(w.FindAnyWidget("ClaimActionBG"));
		m_wClaimActionBorder = ImageWidget.Cast(w.FindAnyWidget("ClaimActionBorder"));
		m_wClaimActionText = TextWidget.Cast(w.FindAnyWidget("ClaimActionText"));
	}

	void Init(TBD_MockRoleLoadout role, bool selected)
	{
		m_Role = role;

		if (m_wRoleTitle)
			m_wRoleTitle.SetText(role.m_sRoleTitle);

		if (m_wSpecialtyBadgeText)
			m_wSpecialtyBadgeText.SetText(role.m_sRoleCategory);

		if (m_wSpecialtyBadgeBG)
		{
			if (role.m_sRoleCategory == "MED")
			{
				m_wSpecialtyBadgeBG.SetColorInt(0x3310B981);
				if (m_wSpecialtyBadgeText)
					m_wSpecialtyBadgeText.SetColorInt(0xFF34D399);
			}
			else if (role.m_sRoleCategory == "ENG")
			{
				m_wSpecialtyBadgeBG.SetColorInt(0x33F59E0B);
				if (m_wSpecialtyBadgeText)
					m_wSpecialtyBadgeText.SetColorInt(0xFFFBBF24);
			}
			else
			{
				m_wSpecialtyBadgeBG.SetColorInt(0x14FFFFFF);
				if (m_wSpecialtyBadgeText)
					TBD_UITheme.Paint(m_wSpecialtyBadgeText, TBD_UITheme.ON_SURFACE_VARIANT);
			}
		}

		if (m_wAiBadgeText)
			m_wAiBadgeText.SetText(role.m_sAiStatus);

		if (m_wAiBadgeBG)
		{
			if (role.m_sAiStatus == "AI Ready")
			{
				m_wAiBadgeBG.SetColorInt(0x33DF7412);
				if (m_wAiBadgeBorder)
					m_wAiBadgeBorder.SetColorInt(0x66DF7412);
				if (m_wAiBadgeText)
					m_wAiBadgeText.SetColorInt(0xFFFDBA74);
			}
			else
			{
				m_wAiBadgeBG.SetColorInt(0x10FFFFFF);
				if (m_wAiBadgeBorder)
					m_wAiBadgeBorder.SetColorInt(0x18FFFFFF);
				if (m_wAiBadgeText)
					m_wAiBadgeText.SetColorInt(0x66FFFFFF);
			}
		}

		UpdateClaimState();
		SetSelected(selected);
	}

	void UpdateClaimState()
	{
		if (!m_Role)
			return;

		if (m_Role.m_bIsClaimed)
		{
			if (m_wClaimActionBG)
				m_wClaimActionBG.SetColorInt(0x33EF4444);
			if (m_wClaimActionBorder)
				m_wClaimActionBorder.SetColorInt(0x66EF4444);
			if (m_wClaimActionText)
			{
				m_wClaimActionText.SetText("Relinquish");
				m_wClaimActionText.SetColorInt(0xFFFCA5A5);
			}
		}
		else if (m_Role.m_sClaimStatus == "Claimable")
		{
			if (m_wClaimActionBG)
				TBD_UITheme.Paint(m_wClaimActionBG, TBD_UITheme.PRIMARY_CONTAINER);
			if (m_wClaimActionBorder)
				m_wClaimActionBorder.SetColorInt(0x884D8EFF);
			if (m_wClaimActionText)
			{
				m_wClaimActionText.SetText("+ Claimable");
				m_wClaimActionText.SetColorInt(0xFFFFFFFF);
			}
		}
		else
		{
			if (m_wClaimActionBG)
				m_wClaimActionBG.SetColorInt(0x10FFFFFF);
			if (m_wClaimActionBorder)
				m_wClaimActionBorder.SetColorInt(0x1AFFFFFF);
			if (m_wClaimActionText)
			{
				m_wClaimActionText.SetText("Restricted");
				m_wClaimActionText.SetColorInt(0x66FFFFFF);
			}
		}
	}

	void SetSelected(bool selected)
	{
		m_bSelected = selected;
		if (!m_wRoot)
			return;

		if (selected)
		{
			if (m_wBG)
				m_wBG.SetColorInt(0x284D8EFF);
			if (m_wAccent)
				TBD_UITheme.Paint(m_wAccent, TBD_UITheme.CARD_BORDER);
			if (m_wRoleTitle)
				TBD_UITheme.Paint(m_wRoleTitle, TBD_UITheme.ON_SURFACE);
		}
		else
		{
			if (m_wBG)
				m_wBG.SetColorInt(0x00000000);
			if (m_wAccent)
				m_wAccent.SetColorInt(0x00000000);
			if (m_wRoleTitle)
				m_wRoleTitle.SetColorInt(0xCCFFFFFF);
		}
	}

	TBD_MockRoleLoadout GetRole()
	{
		return m_Role;
	}

	override bool OnClick(Widget w, int x, int y, int button)
	{
		TBD_LobbyScreenComponent comp = TBD_LobbyScreenComponent.GetInstance();
		if (!comp)
			return false;

		string widgetName = "";
		if (w)
			widgetName = w.GetName();

		if (w == m_wClaimActionBtn || widgetName == "ClaimActionBtn" || widgetName == "ClaimActionOverlay" || widgetName == "ClaimActionBG" || widgetName == "ClaimActionBorder" || widgetName == "ClaimActionText")
		{
			comp.ToggleClaim(m_Role, this);
			return true;
		}

		comp.SelectRole(m_Role, this);
		return true;
	}
}

// -- Squad Card Component --------------------------------------------------------------------
class TBD_LobbySquadCardComponent : ScriptedWidgetComponent
{
	protected Widget m_wRoot;
	protected TextWidget m_wSquadIcon;
	protected TextWidget m_wSquadTitle;
	protected TextWidget m_wSquadVehicle;
	protected TextWidget m_wCategoryBadgeText;
	protected VerticalLayoutWidget m_wRolesContainer;

	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		m_wRoot = w;
		m_wSquadIcon = TextWidget.Cast(w.FindAnyWidget("SquadIcon"));
		m_wSquadTitle = TextWidget.Cast(w.FindAnyWidget("SquadTitle"));
		m_wSquadVehicle = TextWidget.Cast(w.FindAnyWidget("SquadVehicle"));
		m_wCategoryBadgeText = TextWidget.Cast(w.FindAnyWidget("CategoryBadgeText"));
		m_wRolesContainer = VerticalLayoutWidget.Cast(w.FindAnyWidget("RolesContainer"));
	}

	void Init(TBD_MockSquad squad)
	{
		if (m_wSquadTitle)
			m_wSquadTitle.SetText(squad.m_sCallsign);

		if (m_wSquadVehicle)
			m_wSquadVehicle.SetText(squad.m_sVehicleSummary);

		if (m_wCategoryBadgeText)
			m_wCategoryBadgeText.SetText(squad.m_sCategory);

		if (m_wSquadIcon)
		{
			if (squad.m_bIsCommand)
			{
				m_wSquadIcon.SetText("*");
				TBD_UITheme.Paint(m_wSquadIcon, TBD_UITheme.PRIMARY_CONTAINER);
			}
			else
			{
				m_wSquadIcon.SetText("=");
				TBD_UITheme.Paint(m_wSquadIcon, TBD_UITheme.CARD_BORDER);
			}
		}
	}

	VerticalLayoutWidget GetRolesContainer()
	{
		return m_wRolesContainer;
	}
}

// -- Main Screen Component (Root) ------------------------------------------------------------
class TBD_LobbyScreenComponent : ScriptedWidgetComponent
{
	protected static TBD_LobbyScreenComponent s_Instance;

	protected Widget m_wRoot;

	// Toolbar
	protected TextWidget m_wHeaderTitle;
	protected TextWidget m_wHeaderScenario;
	protected TextWidget m_wSlotCountText;
	protected ButtonWidget m_wTabOrbatBtn;
	protected ImageWidget m_wTabOrbatBG;
	protected TextWidget m_wTabOrbatText;
	protected ButtonWidget m_wTabBriefingBtn;
	protected ImageWidget m_wTabBriefingBG;
	protected TextWidget m_wTabBriefingText;
	protected ButtonWidget m_wToggleAiBtn;
	protected ImageWidget m_wToggleAiPill;
	protected TextWidget m_wToggleAiLabel;
	protected TextWidget m_wHostName;

	// Sidebar
	protected VerticalLayoutWidget m_wFactionsContainer;
	protected VerticalLayoutWidget m_wObserversContainer;
	protected VerticalLayoutWidget m_wVoiceTreeContainer;
	protected TextWidget m_wConnectedPlayersValue;
	protected ImageWidget m_wProgressFill;

	// Center Roster
	protected EditBoxWidget m_wSearchInput;
	protected TextWidget m_wRosterStatsPillText;
	protected TextWidget m_wRosterStatsLabel;
	protected ButtonWidget m_wChipAll;
	protected ButtonWidget m_wChipHq;
	protected ButtonWidget m_wChipArmor;
	protected ButtonWidget m_wChipMech;
	protected ButtonWidget m_wChipAvailable;
	protected VerticalLayoutWidget m_wSquadCardsContainer;

	// Inspector
	protected TextWidget m_wInspectorRoleTitle;
	protected TextWidget m_wInspectorRoleSquad;
	protected TextWidget m_wClearanceValue;
	protected TextWidget m_wRankValue;
	protected TextWidget m_wVehicleName;
	protected TextWidget m_wVehicleStatus;
	protected TextWidget m_wVehicleDesc;
	protected TextWidget m_wPrimaryName;
	protected TextWidget m_wPrimaryCaliber;
	protected TextWidget m_wPrimaryAmmo;
	protected TextWidget m_wPrimaryAttachments;
	protected TextWidget m_wSidearmName;
	protected TextWidget m_wSidearmAmmo;
	protected TextWidget m_wHelmetTitle;
	protected TextWidget m_wHelmetRating;
	protected TextWidget m_wRigTitle;
	protected TextWidget m_wRigLoad;
	protected TextWidget m_wRadioTitle;
	protected TextWidget m_wRadioFreq;
	protected TextWidget m_wRadioStatus;
	protected TextWidget m_wInvItem1;
	protected TextWidget m_wInvItem2;
	protected TextWidget m_wIfakTitle;
	protected TextWidget m_wIfakStatus;
	protected TextWidget m_wIfakPills;
	protected TextWidget m_wSlotStatusText;
	protected ImageWidget m_wSlotStatusBG;
	protected ImageWidget m_wSlotStatusBorder;
	protected TextWidget m_wRoleBadgeIcon;

	// Bottom Bar
	protected ButtonWidget m_wBackAction;
	protected ImageWidget m_wBackActionBG;
	protected ImageWidget m_wBackActionBorder;
	protected TextWidget m_wBackActionText;
	protected ButtonWidget m_wLockLobbyBtn;
	protected ImageWidget m_wLockLobbyBG;
	protected ImageWidget m_wLockLobbyBorder;
	protected TextWidget m_wLockLobbyText;
	protected ButtonWidget m_wReadyContinueBtn;
	protected ImageWidget m_wReadyContinueBG;
	protected TextWidget m_wReadyContinueText;

	// Data & State
	protected ref array<ref TBD_MockFaction> m_aFactions;
	protected ref array<ref TBD_MockVoiceChannel> m_aVoiceChannels;
	protected ref array<TBD_LobbyFactionRowComponent> m_aFactionRowComps;
	protected ref array<TBD_LobbySlotRowComponent> m_aSlotRowComps;

	protected int m_iSelectedFactionIndex;
	protected ref TBD_MockRoleLoadout m_SelectedRole;
	protected TBD_LobbySlotRowComponent m_SelectedSlotRowComp;
	protected string m_sActiveFilter;
	protected bool m_bAvailableOnly;
	protected bool m_bAiDisabled;
	protected bool m_bLobbyLocked;
	protected bool m_bPlayerReady;
	protected int m_iActiveTab; // 0: ORBAT, 1: Briefing

	static TBD_LobbyScreenComponent GetInstance()
	{
		return s_Instance;
	}

	override void HandlerAttached(Widget w)
	{
		super.HandlerAttached(w);
		s_Instance = this;
		m_wRoot = w;

		m_aFactionRowComps = {};
		m_aSlotRowComps = {};
		m_sActiveFilter = "ALL";
		m_bAvailableOnly = false;
		m_bAiDisabled = false;
		m_bLobbyLocked = false;
		m_bPlayerReady = false;
		m_iActiveTab = 0;
		m_iSelectedFactionIndex = 0;

		m_aFactions = TBD_LobbyMockData.GetMockFactions();
		m_aVoiceChannels = TBD_LobbyMockData.GetMockVoiceChannels();

		CreateSubLayouts(w);
		BindWidgets(w);
		SwitchTab(0);
		if (m_wToggleAiPill)
			m_wToggleAiPill.SetColorInt(0x33FFFFFF);
		if (m_wToggleAiLabel)
			m_wToggleAiLabel.SetColorInt(0x99FFFFFF);
		PopulateFactions();
		PopulateRoster();
		PopulateVoiceChannels();

		// Default select first role of first squad (Company Commander)
		if (m_aFactions.Count() > 0 && m_aFactions[0].m_aSquads.Count() > 0 && m_aFactions[0].m_aSquads[0].m_aRoles.Count() > 0)
		{
			TBD_MockRoleLoadout defaultRole = m_aFactions[0].m_aSquads[0].m_aRoles[0];
			TBD_LobbySlotRowComponent defaultComp = null;
			if (m_aSlotRowComps.Count() > 0)
				defaultComp = m_aSlotRowComps[0];
			SelectRole(defaultRole, defaultComp);
		}
	}

	override void HandlerDeattached(Widget w)
	{
		if (s_Instance == this)
			s_Instance = null;

		m_aFactionRowComps = null;
		m_aSlotRowComps = null;
		m_aFactions = null;
		m_aVoiceChannels = null;
		super.HandlerDeattached(w);
	}

	protected void CreateSubLayouts(Widget w)
	{
		Widget headerDock = w.FindAnyWidget("HeaderDock");
		if (headerDock)
			TBD_UILayouts.Create(TBD_UILayouts.LOBBY_HEADER, headerDock);

		Widget sidebarDock = w.FindAnyWidget("SidebarDock");
		if (sidebarDock)
			TBD_UILayouts.Create(TBD_UILayouts.LOBBY_SIDEBAR, sidebarDock);

		Widget rosterDock = w.FindAnyWidget("CenterRosterDock");
		if (rosterDock)
			TBD_UILayouts.Create(TBD_UILayouts.LOBBY_ROSTER, rosterDock);

		Widget inspectorDock = w.FindAnyWidget("InspectorDock");
		if (inspectorDock)
			TBD_UILayouts.Create(TBD_UILayouts.LOBBY_INSPECTOR, inspectorDock);

		Widget footerDock = w.FindAnyWidget("FooterDock");
		if (footerDock)
			TBD_UILayouts.Create(TBD_UILayouts.LOBBY_FOOTER, footerDock);
	}

	protected void BindWidgets(Widget w)
	{
		m_wHeaderTitle = TextWidget.Cast(w.FindAnyWidget("HeaderTitle"));
		m_wHeaderScenario = TextWidget.Cast(w.FindAnyWidget("HeaderScenario"));
		m_wSlotCountText = TextWidget.Cast(w.FindAnyWidget("SlotCountText"));
		m_wTabOrbatBtn = ButtonWidget.Cast(w.FindAnyWidget("TabOrbatBtn"));
		m_wTabOrbatBG = ImageWidget.Cast(w.FindAnyWidget("TabOrbatBG"));
		m_wTabOrbatText = TextWidget.Cast(w.FindAnyWidget("TabOrbatText"));
		m_wTabBriefingBtn = ButtonWidget.Cast(w.FindAnyWidget("TabBriefingBtn"));
		m_wTabBriefingBG = ImageWidget.Cast(w.FindAnyWidget("TabBriefingBG"));
		m_wTabBriefingText = TextWidget.Cast(w.FindAnyWidget("TabBriefingText"));
		m_wToggleAiBtn = ButtonWidget.Cast(w.FindAnyWidget("ToggleAiBtn"));
		m_wToggleAiPill = ImageWidget.Cast(w.FindAnyWidget("ToggleAiPill"));
		m_wToggleAiLabel = TextWidget.Cast(w.FindAnyWidget("ToggleAiLabel"));
		m_wHostName = TextWidget.Cast(w.FindAnyWidget("HostName"));

		m_wFactionsContainer = VerticalLayoutWidget.Cast(w.FindAnyWidget("FactionsContainer"));
		m_wObserversContainer = VerticalLayoutWidget.Cast(w.FindAnyWidget("ObserversContainer"));
		m_wVoiceTreeContainer = VerticalLayoutWidget.Cast(w.FindAnyWidget("VoiceTreeContainer"));
		m_wConnectedPlayersValue = TextWidget.Cast(w.FindAnyWidget("ConnectedPlayersValue"));
		m_wProgressFill = ImageWidget.Cast(w.FindAnyWidget("ProgressFill"));

		m_wSearchInput = EditBoxWidget.Cast(w.FindAnyWidget("SearchInput"));
		m_wRosterStatsPillText = TextWidget.Cast(w.FindAnyWidget("RosterStatsPillText"));
		m_wRosterStatsLabel = TextWidget.Cast(w.FindAnyWidget("RosterStatsLabel"));
		m_wChipAll = ButtonWidget.Cast(w.FindAnyWidget("ChipAll"));
		m_wChipHq = ButtonWidget.Cast(w.FindAnyWidget("ChipHq"));
		m_wChipArmor = ButtonWidget.Cast(w.FindAnyWidget("ChipArmor"));
		m_wChipMech = ButtonWidget.Cast(w.FindAnyWidget("ChipMech"));
		m_wChipAvailable = ButtonWidget.Cast(w.FindAnyWidget("ChipAvailable"));
		m_wSquadCardsContainer = VerticalLayoutWidget.Cast(w.FindAnyWidget("SquadCardsContainer"));

		m_wInspectorRoleTitle = TextWidget.Cast(w.FindAnyWidget("InspectorRoleTitle"));
		m_wInspectorRoleSquad = TextWidget.Cast(w.FindAnyWidget("InspectorRoleSquad"));
		m_wClearanceValue = TextWidget.Cast(w.FindAnyWidget("ClearanceValue"));
		m_wRankValue = TextWidget.Cast(w.FindAnyWidget("RankValue"));
		m_wVehicleName = TextWidget.Cast(w.FindAnyWidget("VehicleName"));
		m_wVehicleStatus = TextWidget.Cast(w.FindAnyWidget("VehicleStatus"));
		m_wVehicleDesc = TextWidget.Cast(w.FindAnyWidget("VehicleDesc"));
		m_wPrimaryName = TextWidget.Cast(w.FindAnyWidget("PrimaryName"));
		m_wPrimaryCaliber = TextWidget.Cast(w.FindAnyWidget("PrimaryCaliber"));
		m_wPrimaryAmmo = TextWidget.Cast(w.FindAnyWidget("PrimaryAmmo"));
		m_wPrimaryAttachments = TextWidget.Cast(w.FindAnyWidget("PrimaryAttachments"));
		m_wSidearmName = TextWidget.Cast(w.FindAnyWidget("SidearmName"));
		m_wSidearmAmmo = TextWidget.Cast(w.FindAnyWidget("SidearmAmmo"));
		m_wHelmetTitle = TextWidget.Cast(w.FindAnyWidget("HelmetTitle"));
		m_wHelmetRating = TextWidget.Cast(w.FindAnyWidget("HelmetRating"));
		m_wRigTitle = TextWidget.Cast(w.FindAnyWidget("RigTitle"));
		m_wRigLoad = TextWidget.Cast(w.FindAnyWidget("RigLoad"));
		m_wRadioTitle = TextWidget.Cast(w.FindAnyWidget("RadioTitle"));
		m_wRadioFreq = TextWidget.Cast(w.FindAnyWidget("RadioFreq"));
		m_wRadioStatus = TextWidget.Cast(w.FindAnyWidget("RadioStatus"));
		m_wInvItem1 = TextWidget.Cast(w.FindAnyWidget("InvItem1"));
		m_wInvItem2 = TextWidget.Cast(w.FindAnyWidget("InvItem2"));
		m_wIfakTitle = TextWidget.Cast(w.FindAnyWidget("IfakTitle"));
		m_wIfakStatus = TextWidget.Cast(w.FindAnyWidget("IfakStatus"));
		m_wIfakPills = TextWidget.Cast(w.FindAnyWidget("IfakPills"));
		m_wSlotStatusText = TextWidget.Cast(w.FindAnyWidget("SlotStatusText"));
		m_wSlotStatusBG = ImageWidget.Cast(w.FindAnyWidget("SlotStatusBG"));
		m_wSlotStatusBorder = ImageWidget.Cast(w.FindAnyWidget("SlotStatusBorder"));
		m_wRoleBadgeIcon = TextWidget.Cast(w.FindAnyWidget("RoleBadgeIcon"));

		m_wBackAction = ButtonWidget.Cast(w.FindAnyWidget("BackAction"));
		m_wBackActionBG = ImageWidget.Cast(w.FindAnyWidget("BackActionBG"));
		m_wBackActionBorder = ImageWidget.Cast(w.FindAnyWidget("BackActionBorder"));
		m_wBackActionText = TextWidget.Cast(w.FindAnyWidget("BackActionText"));

		m_wLockLobbyBtn = ButtonWidget.Cast(w.FindAnyWidget("LockLobbyBtn"));
		m_wLockLobbyBG = ImageWidget.Cast(w.FindAnyWidget("LockLobbyBG"));
		m_wLockLobbyBorder = ImageWidget.Cast(w.FindAnyWidget("LockLobbyBorder"));
		m_wLockLobbyText = TextWidget.Cast(w.FindAnyWidget("LockLobbyText"));

		m_wReadyContinueBtn = ButtonWidget.Cast(w.FindAnyWidget("ReadyContinueBtn"));
		m_wReadyContinueBG = ImageWidget.Cast(w.FindAnyWidget("ReadyContinueBG"));
		m_wReadyContinueText = TextWidget.Cast(w.FindAnyWidget("ReadyContinueText"));
	}

	protected void PopulateFactions()
	{
		if (!m_wFactionsContainer)
			return;

		m_aFactionRowComps.Clear();
		int count = m_aFactions.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			TBD_MockFaction faction = m_aFactions[i];
			Widget rowWidget = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_FACTION_ROW, m_wFactionsContainer);
			if (!rowWidget)
				continue;

			TBD_LobbyFactionRowComponent comp = new TBD_LobbyFactionRowComponent();
			rowWidget.AddHandler(comp);
			string badgeText = string.Format("%1 / %2", faction.m_iClaimed, faction.m_iTotalSlots);
			comp.Init(i, faction.m_sName, badgeText, faction.m_iColor, i == m_iSelectedFactionIndex, true);
			m_aFactionRowComps.Insert(comp);
		}

		if (m_wObserversContainer)
		{
			Widget obsWidget = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_FACTION_ROW, m_wObserversContainer);
			if (obsWidget)
			{
				TBD_LobbyFactionRowComponent obsComp = new TBD_LobbyFactionRowComponent();
				obsWidget.AddHandler(obsComp);
				obsComp.Init(99, "Spectators", "1", 0x88FFFFFF, false, false);
			}
		}
	}

	void SelectFaction(int index)
	{
		if (index < 0 || index >= m_aFactions.Count())
			return;

		m_iSelectedFactionIndex = index;
		int count = m_aFactionRowComps.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			m_aFactionRowComps[i].SetSelected(i == index);
		}

		TBD_MockFaction f = m_aFactions[index];
		if (m_wRosterStatsLabel)
			m_wRosterStatsLabel.SetText(string.Format("%1 Roster:", f.m_sName));

		if (m_wRosterStatsPillText)
			m_wRosterStatsPillText.SetText(string.Format("%1 / %2 CLAIMED", f.m_iClaimed, f.m_iTotalSlots));

		PopulateRoster();
	}

	void PopulateRoster()
	{
		if (!m_wSquadCardsContainer || m_iSelectedFactionIndex >= m_aFactions.Count())
			return;

		// Clear previous squad cards
		Widget child = m_wSquadCardsContainer.GetChildren();
		while (child)
		{
			Widget next = child.GetSibling();
			child.RemoveFromHierarchy();
			child = next;
		}

		m_aSlotRowComps.Clear();
		TBD_MockFaction faction = m_aFactions[m_iSelectedFactionIndex];

		string query = "";
		if (m_wSearchInput)
			query = m_wSearchInput.GetText();
		query.ToLower();

		int squadCount = faction.m_aSquads.Count();
		int i, j;
		for (i = 0; i < squadCount; i++)
		{
			TBD_MockSquad squad = faction.m_aSquads[i];

			// Unit filter check
			if (m_sActiveFilter != "ALL" && squad.m_sUnitType != m_sActiveFilter)
				continue;

			// Instantiate squad card
			Widget cardWidget = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_SQUAD_CARD, m_wSquadCardsContainer);
			if (!cardWidget)
				continue;

			TBD_LobbySquadCardComponent squadComp = new TBD_LobbySquadCardComponent();
			cardWidget.AddHandler(squadComp);
			squadComp.Init(squad);

			VerticalLayoutWidget rolesContainer = squadComp.GetRolesContainer();
			if (!rolesContainer)
				continue;

			int roleCount = squad.m_aRoles.Count();
			for (j = 0; j < roleCount; j++)
			{
				TBD_MockRoleLoadout role = squad.m_aRoles[j];

				// Available only check
				if (m_bAvailableOnly && !role.m_bIsClaimable)
					continue;

				// Search query check
				if (!query.IsEmpty())
				{
					string titleLower = role.m_sRoleTitle;
					titleLower.ToLower();
					string squadLower = squad.m_sCallsign;
					squadLower.ToLower();
					if (titleLower.IndexOf(query) < 0 && squadLower.IndexOf(query) < 0)
						continue;
				}

				Widget rowWidget = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_SLOT_ROW, rolesContainer);
				if (!rowWidget)
					continue;

				TBD_LobbySlotRowComponent slotComp = new TBD_LobbySlotRowComponent();
				rowWidget.AddHandler(slotComp);
				bool isSelected = (m_SelectedRole && m_SelectedRole.m_sKey == role.m_sKey);
				slotComp.Init(role, isSelected);
				m_aSlotRowComps.Insert(slotComp);

				if (isSelected)
					m_SelectedSlotRowComp = slotComp;
			}
		}
	}

	protected void PopulateVoiceChannels()
	{
		if (!m_wVoiceTreeContainer)
			return;

		int count = m_aVoiceChannels.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			TBD_MockVoiceChannel ch = m_aVoiceChannels[i];

			// Create channel row
			Widget chRow = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_FACTION_ROW, m_wVoiceTreeContainer);
			if (!chRow)
				continue;

			TBD_LobbyFactionRowComponent comp = new TBD_LobbyFactionRowComponent();
			chRow.AddHandler(comp);
			string badge = string.Format("%1 / %2", ch.m_iConnected, ch.m_iCapacity);
			int dot = 0x66FFFFFF;
			if (ch.m_bActive)
				dot = 0xFF10B981; // Emerald

			comp.Init(100 + i, ch.m_sName, badge, dot, ch.m_bActive, false);
		}
	}

	void SelectRole(TBD_MockRoleLoadout role, TBD_LobbySlotRowComponent comp)
	{
		m_SelectedRole = role;
		m_SelectedSlotRowComp = comp;

		int count = m_aSlotRowComps.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			TBD_LobbySlotRowComponent c = m_aSlotRowComps[i];
			c.SetSelected(c.GetRole() == role);
		}

		UpdateInspector(role);
	}

	void ToggleClaim(TBD_MockRoleLoadout role, TBD_LobbySlotRowComponent comp)
	{
		if (!role)
			return;

		if (!role.m_bIsClaimed && role.m_sClaimStatus != "Claimable")
			return;

		role.m_bIsClaimed = !role.m_bIsClaimed;
		if (role.m_bIsClaimed)
		{
			role.m_sOccupantName = "Mission Maker (You)";
			if (m_iSelectedFactionIndex >= 0 && m_iSelectedFactionIndex < m_aFactions.Count())
				m_aFactions[m_iSelectedFactionIndex].m_iClaimed++;
		}
		else
		{
			role.m_sOccupantName = "";
			if (m_iSelectedFactionIndex >= 0 && m_iSelectedFactionIndex < m_aFactions.Count())
			{
				if (m_aFactions[m_iSelectedFactionIndex].m_iClaimed > 0)
					m_aFactions[m_iSelectedFactionIndex].m_iClaimed--;
			}
		}

		if (comp)
			comp.UpdateClaimState();

		TBD_MockFaction f = m_aFactions[m_iSelectedFactionIndex];
		if (m_wRosterStatsPillText)
			m_wRosterStatsPillText.SetText(string.Format("%1 / %2 CLAIMED", f.m_iClaimed, f.m_iTotalSlots));

		if (m_iSelectedFactionIndex < m_aFactionRowComps.Count())
		{
			string b = string.Format("%1 / %2", f.m_iClaimed, f.m_iTotalSlots);
			m_aFactionRowComps[m_iSelectedFactionIndex].Init(m_iSelectedFactionIndex, f.m_sName, b, f.m_iColor, true, true);
		}

		SelectRole(role, comp);
	}

	protected void UpdateInspector(TBD_MockRoleLoadout role)
	{
		if (!role)
			return;

		if (m_wInspectorRoleTitle)
			m_wInspectorRoleTitle.SetText(role.m_sRoleTitle);

		if (m_wInspectorRoleSquad)
			m_wInspectorRoleSquad.SetText(role.m_sSquadAssignment);

		if (m_wClearanceValue)
			m_wClearanceValue.SetText(role.m_sClearance);

		if (m_wRankValue)
			m_wRankValue.SetText(role.m_sRank);

		if (m_wVehicleName)
			m_wVehicleName.SetText(role.m_sVehicleName);

		if (m_wVehicleStatus)
			m_wVehicleStatus.SetText(role.m_sVehicleStatus);

		if (m_wVehicleDesc)
			m_wVehicleDesc.SetText(role.m_sVehicleDesc);

		if (m_wPrimaryName)
			m_wPrimaryName.SetText(role.m_sPrimaryWeapon);

		if (m_wPrimaryCaliber)
			m_wPrimaryCaliber.SetText(role.m_sPrimaryCaliber);

		if (m_wPrimaryAmmo)
			m_wPrimaryAmmo.SetText(role.m_sPrimaryAmmo);

		if (m_wPrimaryAttachments)
		{
			string atts = "";
			int count = role.m_aPrimaryAttachments.Count();
			int i;
			for (i = 0; i < count; i++)
			{
				atts += string.Format("[%1]  ", role.m_aPrimaryAttachments[i]);
			}
			m_wPrimaryAttachments.SetText(atts);
		}

		if (m_wSidearmName)
			m_wSidearmName.SetText(role.m_sSidearm);

		if (m_wSidearmAmmo)
			m_wSidearmAmmo.SetText(role.m_sSidearmAmmo);

		if (m_wHelmetTitle)
			m_wHelmetTitle.SetText(role.m_sHelmetVest);

		if (m_wHelmetRating)
			m_wHelmetRating.SetText(role.m_sHelmetVestRating);

		if (m_wRigTitle)
			m_wRigTitle.SetText(role.m_sRig);

		if (m_wRigLoad)
			m_wRigLoad.SetText(role.m_sRigLoad);

		if (m_wRadioTitle)
			m_wRadioTitle.SetText(role.m_sRadio);

		if (m_wRadioFreq)
			m_wRadioFreq.SetText(role.m_sRadioFreq);

		if (m_wRadioStatus)
			m_wRadioStatus.SetText(role.m_sRadioStatus);

		if (m_wRoleBadgeIcon)
		{
			if (role.m_sRoleCategory == "OFFICER")
				m_wRoleBadgeIcon.SetText("[*]");
			else if (role.m_sRoleCategory == "NCO")
				m_wRoleBadgeIcon.SetText("[^]");
			else if (role.m_sRoleCategory == "MED")
				m_wRoleBadgeIcon.SetText("[+]");
			else if (role.m_sRoleCategory == "ENG")
				m_wRoleBadgeIcon.SetText("[#]");
			else if (role.m_sRoleCategory == "CREW")
				m_wRoleBadgeIcon.SetText("[@]");
			else
				m_wRoleBadgeIcon.SetText("[-]");
		}

		if (m_wInvItem1 && role.m_aInventoryItems && role.m_aInventoryItems.Count() >= 2)
		{
			m_wInvItem1.SetText(string.Format("[o] %1     [+] %2", role.m_aInventoryItems[0], role.m_aInventoryItems[1]));
		}

		if (m_wInvItem2 && role.m_aInventoryItems && role.m_aInventoryItems.Count() >= 4)
		{
			m_wInvItem2.SetText(string.Format("[~] %1  [!] %2", role.m_aInventoryItems[2], role.m_aInventoryItems[3]));
		}

		if (m_wIfakStatus)
			m_wIfakStatus.SetText(role.m_sIfakStatus);

		if (m_wIfakPills)
		{
			string pills = "";
			int pcount = role.m_aIfakPills.Count();
			int pi;
			for (pi = 0; pi < pcount; pi++)
			{
				pills += string.Format("[%1]  ", role.m_aIfakPills[pi]);
			}
			m_wIfakPills.SetText(pills);
		}

		if (m_wSlotStatusText)
		{
			if (role.m_bIsClaimed)
			{
				m_wSlotStatusText.SetText("CLAIMED");
				m_wSlotStatusText.SetColorInt(0xFF38BDF8);
				if (m_wSlotStatusBG)
					m_wSlotStatusBG.SetColorInt(0x3338BDF8);
				if (m_wSlotStatusBorder)
					m_wSlotStatusBorder.SetColorInt(0x4D38BDF8);
			}
			else if (role.m_sClaimStatus == "Claimable")
			{
				m_wSlotStatusText.SetText("SLOT OPEN");
				m_wSlotStatusText.SetColorInt(0xFF10B981);
				if (m_wSlotStatusBG)
					m_wSlotStatusBG.SetColorInt(0x3310B981);
				if (m_wSlotStatusBorder)
					m_wSlotStatusBorder.SetColorInt(0x4D10B981);
			}
			else
			{
				m_wSlotStatusText.SetText("RESTRICTED");
				m_wSlotStatusText.SetColorInt(0x88FFFFFF);
				if (m_wSlotStatusBG)
					m_wSlotStatusBG.SetColorInt(0x14FFFFFF);
				if (m_wSlotStatusBorder)
					m_wSlotStatusBorder.SetColorInt(0x1AFFFFFF);
			}
		}
	}

	void SetUnitFilter(string filter, Widget activeChip)
	{
		m_sActiveFilter = filter;

		UpdateChipStyle(m_wChipAll, activeChip == m_wChipAll);
		UpdateChipStyle(m_wChipHq, activeChip == m_wChipHq);
		UpdateChipStyle(m_wChipArmor, activeChip == m_wChipArmor);
		UpdateChipStyle(m_wChipMech, activeChip == m_wChipMech);

		PopulateRoster();
	}

	protected void UpdateChipStyle(Widget chip, bool active)
	{
		if (!chip)
			return;

		ImageWidget bg = ImageWidget.Cast(chip.FindAnyWidget(chip.GetName() + "BG"));
		TextWidget text = TextWidget.Cast(chip.FindAnyWidget(chip.GetName() + "Text"));

		if (active)
		{
			if (bg)
				bg.SetColorInt(0x28FFFFFF);
			if (text)
				text.SetColorInt(0xFFFFFFFF);
		}
		else
		{
			if (bg)
				bg.SetColorInt(0x0AFFFFFF);
			if (text)
				text.SetColorInt(0x99FFFFFF);
		}
	}

	void ToggleAvailableOnly()
	{
		m_bAvailableOnly = !m_bAvailableOnly;
		if (m_wChipAvailable)
		{
			ImageWidget border = ImageWidget.Cast(m_wChipAvailable.FindAnyWidget("ChipAvailableBorder"));
			TextWidget text = TextWidget.Cast(m_wChipAvailable.FindAnyWidget("ChipAvailableText"));
			if (m_bAvailableOnly)
			{
				if (border)
					border.SetColorInt(0xFF38BDF8);
				if (text)
					text.SetColorInt(0xFF38BDF8);
			}
			else
			{
				if (border)
					border.SetColorInt(0x33ADC6FF);
				if (text)
					text.SetColorInt(0xBBADC6FF);
			}
		}
		PopulateRoster();
	}

	void ToggleAi()
	{
		m_bAiDisabled = !m_bAiDisabled;
		if (m_wToggleAiPill)
		{
			if (m_bAiDisabled)
				m_wToggleAiPill.SetColorInt(0xFF4D8EFF);
			else
				m_wToggleAiPill.SetColorInt(0x33FFFFFF);
		}
		if (m_wToggleAiLabel)
		{
			if (m_bAiDisabled)
				m_wToggleAiLabel.SetColorInt(0xFFFFFFFF);
			else
				m_wToggleAiLabel.SetColorInt(0x99FFFFFF);
		}
	}

	void ToggleLock()
	{
		m_bLobbyLocked = !m_bLobbyLocked;
		if (m_bLobbyLocked)
		{
			if (m_wLockLobbyText)
			{
				m_wLockLobbyText.SetText("[!] Lobby Locked");
				m_wLockLobbyText.SetColorInt(0xFFF87171);
			}
			if (m_wLockLobbyBG)
				m_wLockLobbyBG.SetColorInt(0x33EF4444);
			if (m_wLockLobbyBorder)
				m_wLockLobbyBorder.SetColorInt(0x66EF4444);
		}
		else
		{
			if (m_wLockLobbyText)
			{
				m_wLockLobbyText.SetText("[*] Lock Lobby");
				m_wLockLobbyText.SetColorInt(0xFFFFB786);
			}
			if (m_wLockLobbyBG)
				m_wLockLobbyBG.SetColorInt(0x10FFFFFF);
			if (m_wLockLobbyBorder)
				m_wLockLobbyBorder.SetColorInt(0x14FFFFFF);
		}
	}

	void ToggleReady()
	{
		m_bPlayerReady = !m_bPlayerReady;
		if (m_bPlayerReady)
		{
			if (m_wReadyContinueText)
				m_wReadyContinueText.SetText("[V] Ready");
			if (m_wReadyContinueBG)
				m_wReadyContinueBG.SetColorInt(0xFF22C55E);
		}
		else
		{
			if (m_wReadyContinueText)
				m_wReadyContinueText.SetText("> Ready & Continue");
			if (m_wReadyContinueBG)
				m_wReadyContinueBG.SetColorInt(0xFF4D8EFF);
		}

		Print(string.Format("[TBD][lobby] Ready & Continue clicked (ready=%1).", m_bPlayerReady));
		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIBriefing);
	}

	void SwitchTab(int tab)
	{
		m_iActiveTab = tab;
		if (m_wTabOrbatBG)
		{
			if (tab == 0)
				m_wTabOrbatBG.SetColorInt(0x24FFFFFF);
			else
				m_wTabOrbatBG.SetColorInt(0x00000000);
		}
		if (m_wTabOrbatText)
		{
			if (tab == 0)
				m_wTabOrbatText.SetColorInt(0xFFFFFFFF);
			else
				m_wTabOrbatText.SetColorInt(0x99FFFFFF);
		}

		if (m_wTabBriefingBG)
		{
			if (tab == 1)
				m_wTabBriefingBG.SetColorInt(0x24FFFFFF);
			else
				m_wTabBriefingBG.SetColorInt(0x00000000);
		}
		if (m_wTabBriefingText)
		{
			if (tab == 1)
				m_wTabBriefingText.SetColorInt(0xFFFFFFFF);
			else
				m_wTabBriefingText.SetColorInt(0x99FFFFFF);
		}

		if (tab == 1)
		{
			Print("[TBD][lobby] Switch to Briefing requested.");
			TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIBriefing);
		}
	}

	override bool OnClick(Widget w, int x, int y, int button)
	{
		string name = "";
		if (w)
			name = w.GetName();

		if (w == m_wBackAction || name == "BackAction" || name == "BackActionOverlay" || name == "BackActionBG" || name == "BackActionBorder" || name == "BackActionText")
		{
			TBD_MenuStack.CloseTop();
			return true;
		}

		if (w == m_wLockLobbyBtn || name == "LockLobbyBtn" || name == "LockLobbyOverlay" || name == "LockLobbyBG" || name == "LockLobbyBorder" || name == "LockLobbyText")
		{
			ToggleLock();
			return true;
		}

		if (w == m_wReadyContinueBtn || name == "ReadyContinueBtn" || name == "ReadyContinueOverlay" || name == "ReadyContinueBG" || name == "ReadyContinueText")
		{
			ToggleReady();
			return true;
		}

		if (w == m_wToggleAiBtn || name == "ToggleAiBtn" || name == "ToggleAiLabel" || name == "ToggleAiPill" || name == "ToggleAiContent" || name == "ToggleAiBG" || name == "ToggleAiBorder" || name == "ToggleAiOverlay")
		{
			ToggleAi();
			return true;
		}

		if (w == m_wTabOrbatBtn || name == "TabOrbatBtn" || name == "TabOrbatText" || name == "TabOrbatBG" || name == "TabOrbatOverlay")
		{
			SwitchTab(0);
			return true;
		}

		if (w == m_wTabBriefingBtn || name == "TabBriefingBtn" || name == "TabBriefingText" || name == "TabBriefingBG" || name == "TabBriefingOverlay")
		{
			SwitchTab(1);
			return true;
		}

		if (w == m_wChipAll || name == "ChipAll" || name == "ChipAllText") { SetUnitFilter("ALL", m_wChipAll); return true; }
		if (w == m_wChipHq || name == "ChipHq" || name == "ChipHqText") { SetUnitFilter("HQ", m_wChipHq); return true; }
		if (w == m_wChipArmor || name == "ChipArmor" || name == "ChipArmorText") { SetUnitFilter("ARMOR", m_wChipArmor); return true; }
		if (w == m_wChipMech || name == "ChipMech" || name == "ChipMechText") { SetUnitFilter("MECH", m_wChipMech); return true; }
		if (w == m_wChipAvailable || name == "ChipAvailable" || name == "ChipAvailableText") { ToggleAvailableOnly(); return true; }

		return false;
	}
}

// -- Chimera Menu Wrapper --------------------------------------------------------------------
class TBD_LobbyScreen : TBD_MenuBase
{
	static void OpenFromPause()
	{
		if (TBD_MenuStack.IsOpen(ChimeraMenuPreset.TBD_UILobby))
			return;

		TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UILobby);
	}

	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();
		Print("[TBD][lobby] TBD_LobbyScreen opened (Stitch 1:1 workstation).");
	}

	override protected void OnScreenClose()
	{
		Print("[TBD][lobby] TBD_LobbyScreen closed.");
		super.OnScreenClose();
	}
}

modded enum ChimeraMenuPreset
{
	TBD_UILobby
}

// -- Pause Menu Integration for Slot Change --------------------------------------------------
modded class PauseMenuUI
{
	protected SCR_ButtonTextComponent m_TbdChangeSlotButton;

	override void OnMenuOpen()
	{
		super.OnMenuOpen();
		HookTbdChangeSlot();
	}

	override void OnMenuClose()
	{
		if (m_TbdChangeSlotButton)
		{
			m_TbdChangeSlotButton.m_OnClicked.Remove(OnTbdChangeSlot);
			m_TbdChangeSlotButton = null;
		}
		super.OnMenuClose();
	}

	protected void HookTbdChangeSlot()
	{
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		TBD_EGameStage stage = fm.GetStage();
		if (stage != TBD_EGameStage.BRIEFING
			&& stage != TBD_EGameStage.SAFE_START
			&& stage != TBD_EGameStage.LIVE)
			return;

		Widget root = GetRootWidget();
		if (!root)
			return;

		SCR_ButtonTextComponent btn = SCR_ButtonTextComponent.GetButtonText("LeaveFaction", root);
		if (!btn)
			return;

		Widget row = btn.GetRootWidget();
		if (row)
			row.SetVisible(true);

		btn.SetText("Change slot");
		btn.SetEnabled(true);
		btn.m_OnClicked.Insert(OnTbdChangeSlot);
		m_TbdChangeSlotButton = btn;
	}

	protected void OnTbdChangeSlot()
	{
		Close();
		GetGame().GetCallqueue().CallLater(TBD_LobbyScreen.OpenFromPause, 0, false);
	}
}
