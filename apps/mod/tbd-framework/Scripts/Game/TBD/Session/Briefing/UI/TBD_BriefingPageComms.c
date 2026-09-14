//! Briefing rebuild (2026-09-14) — Frequencies and ORBAT pages.

//! `frequencies_panel`: the LR command net (gold) and the SR squad nets; the reader's own squad
//! net is highlighted.
class TBD_BriefingFrequenciesPage : TBD_BriefingPage
{
	override string Title() { return "Radio Frequencies Net"; }
	override string Icon()  { return "cell_tower"; }

	override void Fill(Widget content)
	{
		array<ref TBD_NetInfo> nets = m_Catalog.GetNets();

		TBD_Caption.Mount(content, "Long Range (LR) Command");
		foreach (TBD_NetInfo lr : nets)
		{
			if (lr.m_bLongRange)
				AddNet(content, lr);
		}

		int squadNets;
		foreach (TBD_NetInfo count : nets)
		{
			if (!count.m_bLongRange)
				squadNets++;
		}

		TBD_Caption.Mount(content, "Short Range (SR) Squad Nets", string.Format("%1 nets", squadNets));
		foreach (TBD_NetInfo sr : nets)
		{
			if (!sr.m_bLongRange)
				AddNet(content, sr);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void AddNet(Widget content, TBD_NetInfo net)
	{
		Widget row = TBD_UILayouts.CreateStretched(TBD_UILayouts.BRIEFING_FREQ_ROW, content);
		if (!row)
			return;

		Widget border = row.FindAnyWidget("Border");
		Widget background = row.FindAnyWidget("Background");
		TBD_UILayouts.MountRounded(border, TBD_UITheme.RADIUS_ROW);
		TBD_UILayouts.MountRounded(background, TBD_UITheme.RADIUS_ROW - 1);

		int borderTone = TBD_UITheme.KIT_CARD_BORDER;
		TBD_EUITint chipTint = TBD_EUITint.NEUTRAL;
		int nameInk = TBD_UITheme.ON_SURFACE;
		if (net.m_bLongRange)
		{
			chipTint = TBD_EUITint.WARNING;
			nameInk = TBD_UITheme.BRIGHT_INK;
		}
		else if (net.m_bOwn)
		{
			chipTint = TBD_EUITint.PRIMARY;
			nameInk = TBD_UITheme.BRIGHT_INK;
			borderTone = TBD_UITheme.ROW_SELECTED_BORDER;
		}

		TBD_UITheme.PaintOver(border, borderTone, m_iGround);
		TBD_UITheme.PaintOver(background, TBD_UITheme.KIT_CARD_FILL, m_iGround);
		int rowGround = TBD_UITheme.Over(TBD_UITheme.KIT_CARD_FILL, m_iGround);

		TextWidget name = TextWidget.Cast(row.FindAnyWidget("Name"));
		TBD_UITheme.Write(name, net.Title());
		TBD_UITheme.Paint(name, nameInk);

		TBD_ChipComponent chip = TBD_ChipComponent.Mount(row.FindAnyWidget("FreqChipDock"), net.FreqText(), chipTint, rowGround);
		if (chip)
			chip.SetUppercase(false);

		TBD_UITheme.PaintOver(row.FindAnyWidget("AuxRule"), TBD_UITheme.KIT_CARD_BORDER, rowGround);
		TextWidget auxLabel = TextWidget.Cast(row.FindAnyWidget("AuxLabel"));
		TextWidget auxValue = TextWidget.Cast(row.FindAnyWidget("AuxValue"));
		if (net.m_bLongRange)
			TBD_UITheme.Write(auxLabel, "Auxiliary Fallback:");
		else
			TBD_UITheme.Write(auxLabel, "Aux Channels:");
		TBD_UITheme.Write(auxValue, net.AuxText());
		TBD_UITheme.Paint(auxLabel, TBD_UITheme.DIM_INK);
		TBD_UITheme.Paint(auxValue, TBD_UITheme.MUTED_INK);
	}
}

//! ORBAT: the lobby's roster (read-only, the reader's side) beside the kit inspector, exactly the
//! slotting screen's pair. No page panel of its own — `TBD_OrbatPage.layout` holds the two docks.
class TBD_BriefingOrbatPage : TBD_BriefingPage
{
	protected ref TBD_LobbyRosterPanel m_Roster;
	protected ref TBD_KitInspectorPanel m_Kit;

	override string Title() { return "ORBAT"; }
	override string Icon()  { return "groups"; }
	override bool UsesPanel() { return false; }

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

	//------------------------------------------------------------------------------------------------
	//! Squad positions are not on the wire yet (operator word: fine for now) — the button exists,
	//! the pan waits for the data.
	protected void OnSquadLocate(TBD_LobbyRosterPanel panel, string callsign)
	{
		Print(string.Format("[TBD][briefing] locate squad %1 (no squad position on the wire yet)", callsign));
	}

	//------------------------------------------------------------------------------------------------
	protected void OnSlotSelected(TBD_LobbyRosterPanel panel, string slotKey)
	{
		if (!m_Kit || !m_Lobby)
			return;

		m_Kit.Show(m_Lobby.GetSlot(slotKey), m_Lobby.GetSquadOf(slotKey));
	}

	//------------------------------------------------------------------------------------------------
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
