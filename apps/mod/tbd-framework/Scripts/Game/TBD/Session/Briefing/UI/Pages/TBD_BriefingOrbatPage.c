/**
 * @file TBD_BriefingOrbatPage.c
 * @brief The ORBAT page: the lobby roster read-only beside the kit inspector.
 *
 * Role: mounts TBD_OrbatPage.layout with a read-only TBD_LobbyRosterPanel filtered to the reader's
 * side and a TBD_KitInspectorPanel for the selected slot.  Position: TBD_BriefingNav.CreatePage creates it; TBD_BriefingScreen builds it into the page column.
 * State: the roster and kit panels, owned by the page.  Invariants: the page has no panel of
 * its own; squad Locate logs only, because squad positions are not on the wire.
 */

//! ORBAT: the lobby's roster (read-only, the reader's side) beside the kit inspector, exactly the
//! slotting screen's pair. No page panel of its own -- `TBD_OrbatPage.layout` holds the two docks.
class TBD_BriefingOrbatPage : TBD_BriefingPage
{
	protected ref TBD_LobbyRosterPanel m_Roster; //!< the read-only roster
	protected ref TBD_KitInspectorPanel m_Kit; //!< the kit inspector

	//! @return the panel title
	override string Title() { return "ORBAT"; }
	//! @return the header icon key
	override string Icon()  { return "groups"; }
	//! @return false: the page mounts its own two-dock layout
	override bool UsesPanel() { return false; }

	//! Mount the roster and kit inspector into `content` and select the reader's own slot.
	//! @param content the raw page dock
	override void Fill(Widget content)
	{
		if (!m_Lobby)
			return;

		m_wRoot = TBD_UILayouts.Create(TBD_UILayouts.BRIEFING_ORBAT_PAGE, content);
		if (!m_wRoot)
			return;

		Widget rosterRoot = TBD_UILayouts.Create(TBD_UILayouts.PANEL_FILL, m_wRoot.FindAnyWidget("RosterDock"));
		m_Roster = new TBD_LobbyRosterPanel();
		if (m_Roster.Build(rosterRoot, m_Lobby))
		{
			m_Roster.SetReadOnly(true);
			m_Roster.SetShowLocate(true);
			m_Roster.GetOnSelected().Insert(OnSlotSelected);
			m_Roster.GetOnLocate().Insert(OnSquadLocate);
		}

		Widget kitRoot = TBD_UILayouts.Create(TBD_UILayouts.LOBBY_KIT_INSPECTOR, m_wRoot.FindAnyWidget("KitDock"));
		m_Kit = new TBD_KitInspectorPanel();
		m_Kit.Build(kitRoot, m_Lobby);

		string factionKey;
		TBD_BriefingFaction own = m_Catalog.GetOwnFaction();
		if (own)
			factionKey = own.m_sKey;

		m_Roster.SetFaction(factionKey);
		if (!m_Lobby.GetOwnKey().IsEmpty())
			m_Roster.Select(m_Lobby.GetOwnKey(), true);
	}

	//! Log a squad Locate; squad positions are not on the wire, so the map does not pan.
	//! @param panel the roster
	//! @param callsign the squad callsign
	protected void OnSquadLocate(TBD_LobbyRosterPanel panel, string callsign)
	{
		Print(string.Format("[TBD][briefing] locate squad %1 (no squad position on the wire yet)", callsign));
	}

	//! Show the selected slot and its squad in the kit inspector.
	//! @param panel the roster
	//! @param slotKey the selected slot key
	protected void OnSlotSelected(TBD_LobbyRosterPanel panel, string slotKey)
	{
		if (!m_Kit || !m_Lobby)
			return;

		m_Kit.Show(m_Lobby.GetSlot(slotKey), m_Lobby.GetSquadOf(slotKey));
	}

	//! Unbind and destroy the roster and kit panels, then the page.
	override void Destroy()
	{
		if (m_Roster)
		{
			m_Roster.GetOnSelected().Remove(OnSlotSelected);
			m_Roster.GetOnLocate().Remove(OnSquadLocate);
			m_Roster.Destroy();
		}

		if (m_Kit)
			m_Kit.Destroy();

		m_Roster = null;
		m_Kit = null;
		super.Destroy();
	}
}
