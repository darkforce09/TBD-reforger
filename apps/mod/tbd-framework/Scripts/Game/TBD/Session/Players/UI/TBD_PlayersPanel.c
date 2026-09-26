/**
 * @file TBD_PlayersPanel.c
 * @brief The briefing's PLAYERS panel: a header with the total and four sections of connected players.
 *
 * Role: builds TBD_UILayouts.PLAYERS_PANEL into a host dock with the title, the total, and the
 * BLUFOR, OPFOR, Spectators and Unslotted sections, and removes it again.
 * Position: TBD_BriefingScreen builds it into its WideDock in the PLAYERS mode and destroys it on
 * the next mode; data comes from TBD_PlayersCatalog and the role labels from TBD_LobbyCatalog;
 * each section is a TBD_PlayerLane.
 * State: the panel root widget and the sections, per instance, client.
 * Invariants: a plain Managed controller, not a menu: it pops out beside the primary navigation
 * with no scrim or window, because a stacked menu would hide the briefing and close its map.
 */

//! The PLAYERS panel controller.
class TBD_PlayersPanel : Managed
{
	protected Widget m_wRoot; //!< the panel root inside the host dock; null when not built
	protected ref array<ref TBD_PlayerLane> m_aLanes; //!< the built sections

	//! Build the panel into a dock: frame, title, total and the four sections.
	//! @param dock the host dock widget
	//! @return false when the dock is null or the layout will not load
	bool Build(Widget dock)
	{
		m_aLanes = {};
		if (!dock)
			return false;

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.PLAYERS_PANEL, dock);
		if (!m_wRoot)
			return false;

		TBD_PlayersCatalog players = TBD_PlayersCatalog.Get();
		TBD_LobbyCatalog lobby = TBD_LobbyCatalog.Get();

		Widget border = m_wRoot.FindAnyWidget("WindowBorder");
		Widget background = m_wRoot.FindAnyWidget("WindowBG");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_PANEL);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_PANEL - 1);
		TBD_UITheme.PaintOver(border, TBD_UITheme.PanelBorder(TBD_EUITint.NEUTRAL), TBD_UITheme.Ground());
		TBD_UITheme.PaintOver(background, TBD_UITheme.PANEL_FILL, TBD_UITheme.Ground());
		int ground = TBD_UITheme.PanelGround();

		TBD_UITheme.Paint(m_wRoot.FindAnyWidget("Title"), TBD_UITheme.BRIGHT_INK);
		TBD_UITheme.PaintOver(m_wRoot.FindAnyWidget("HeaderRule"), TBD_UITheme.STRIP_BORDER, ground);
		TextWidget total = TextWidget.Cast(m_wRoot.FindAnyWidget("TotalText"));
		TBD_UITheme.Write(total, string.Format("TOTAL: %1 PLAYERS", players.Total()));
		TBD_UITheme.Paint(total, TBD_UITheme.MUTED_INK);

		AddLane("BluforDock", "BLUFOR", RoleOf(lobby, "BLUFOR"), TBD_EUITint.BLUFOR, players.GetSlotted("BLUFOR"), "Slotted:", CountText(players.CountSlotted("BLUFOR"), players.Capacity("BLUFOR")));
		AddLane("OpforDock", "OPFOR", RoleOf(lobby, "OPFOR"), TBD_EUITint.OPFOR, players.GetSlotted("OPFOR"), "Slotted:", CountText(players.CountSlotted("OPFOR"), players.Capacity("OPFOR")));
		array<TBD_PlayerInfo> spectators = players.GetByState(TBD_EPlayerState.SPECTATOR);
		AddLane("SpectatorDock", "Spectators", "", TBD_EUITint.NEUTRAL, spectators, "Count:", spectators.Count().ToString());
		array<TBD_PlayerInfo> unslotted = players.GetByState(TBD_EPlayerState.UNSLOTTED);
		AddLane("UnslottedDock", "Unslotted", "", TBD_EUITint.NEUTRAL, unslotted, "Pending:", unslotted.Count().ToString());

		Print("[TBD][players] panel opened.");
		return true;
	}

	//! Destroy the sections and remove the panel from the hierarchy. Safe when not built.
	void Destroy()
	{
		if (m_aLanes)
		{
			foreach (TBD_PlayerLane lane : m_aLanes)
			{
				if (lane)
					lane.Destroy();
			}
			m_aLanes.Clear();
		}

		if (m_wRoot)
			m_wRoot.RemoveFromHierarchy();

		m_wRoot = null;
		Print("[TBD][players] panel closed.");
	}

	//! Build one section into the named dock and keep it when it builds.
	//! @param dockName the section dock widget name in the panel layout
	//! @param name the section title
	//! @param role the role chip text; empty for no chip
	//! @param tint the section tint
	//! @param rows the players of the section
	//! @param countLabel the label before the count chip
	//! @param countText the count chip text
	protected void AddLane(string dockName, string name, string role, TBD_EUITint tint, array<TBD_PlayerInfo> rows, string countLabel, string countText)
	{
		TBD_PlayerLane lane = new TBD_PlayerLane();
		if (lane.Build(m_wRoot.FindAnyWidget(dockName), name, role, tint, rows, countLabel, countText))
			m_aLanes.Insert(lane);
	}

	//! The side's role label from the lobby catalog, capitalised ("Attackers").
	//! @param lobby the lobby catalog, or null
	//! @param factionKey "BLUFOR" or "OPFOR"
	//! @return the label, or empty when unknown
	protected static string RoleOf(TBD_LobbyCatalog lobby, string factionKey)
	{
		if (!lobby)
			return string.Empty;

		TBD_LobbyFactionInfo faction = lobby.GetFaction(factionKey);
		if (!faction)
			return string.Empty;

		string role = faction.m_sRoleLabel;
		role.ToLower();
		if (role.IsEmpty())
			return string.Empty;

		string first = role.Substring(0, 1);
		first.ToUpper();
		return first + role.Substring(1, role.Length() - 1);
	}

	//! @param slotted players slotted on the side
	//! @param capacity seats on the side; 0 or less when unknown
	//! @return "slotted / capacity", or the slotted count alone without a capacity
	protected static string CountText(int slotted, int capacity)
	{
		if (capacity > 0)
			return string.Format("%1 / %2", slotted, capacity);

		return slotted.ToString();
	}
}
