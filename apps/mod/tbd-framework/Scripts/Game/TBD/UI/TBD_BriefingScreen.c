//! T-181.9.2 — the briefing screen: what your side reads, and plans from, before the round goes
//! live. Arma 3 hands you this for free; Reforger hands you nothing (TBD_MOD_DESIGN.md §1).
//!
//! ```
//!   ┌────────────────────────────────────────────────────────────┐
//!   │  BRIDGEHEAD AT LEVIE                            [ Back ]   │
//!   │  US Army · everon · BRIEFING                               │
//!   ├────────────────────────────────────────────────────────────┤
//!   │  YOUR SEAT                                                 │  <- section
//!   │    Alpha · SL                              kit:us_sl   +   │  <- pick to disclose kit
//!   │                                                            │
//!   │  ORBAT — US ARMY                              9 seats      │  <- section
//!   │    ALPHA                              9 seats · YOURS  -   │  <- own squad, pre-expanded
//!   │      SL                                       x1 · YOU     │
//!   │      TL                                       x2           │
//!   │    BRAVO                                      8 seats  +   │  <- collapsed
//!   │                                                            │
//!   │  AREA OF OPERATIONS                                        │  <- section
//!   │    Objective capture — z3                 5402, 6890 · r80 │
//!   │                                                            │
//!   │  HOW THIS ENDS                       Attack defend v1      │  <- section
//!   │    Time limit                                              │
//!   │    All objectives captured                                 │
//!   ├────────────────────────────────────────────────────────────┤
//!   │  Ready — 3 of 8 on US Army              [ I'M READY ]      │  <- ONE primary action
//!   └────────────────────────────────────────────────────────────┘
//! ```
//!
//! ── Design law this screen obeys (TBD_MOD_DESIGN.md §2, §6) ────────────────────────────────
//! * **ONE obvious primary action.** `I'M READY`. Everything else is a list pick or Back. The
//!   shell physically cannot grow a second loud button.
//! * **Progressive disclosure.** A 128-slot mission is not a wall of 128 rows: it is a handful of
//!   squads, and you open the one you care about. The reader's own squad starts expanded because
//!   that is the one they need; every other squad is one click away. Their kit is disclosed the
//!   same way, so a slot with no loadout costs no rows at all.
//! * **Direct manipulation.** Picking a row acts immediately — no select-then-confirm step.
//! * **Immediate feedback, nothing blocking.** Readiness reports into the footer status line, not
//!   a modal.
//! * **Aegis tokens only.** Every colour comes from `TBD_UITheme`; this file contains no literal.
//!
//! ── Open seam: getting BACK in ─────────────────────────────────────────────────────────────
//! The shell's Back button closes this screen, and the stage watcher only opens it on the
//! transition INTO BRIEFING — so a player who backs out cannot currently re-open their orders
//! until the phase changes. That is a real hole for a planning screen, and it is deliberately
//! NOT patched here by re-opening from the watcher, which would fight the user's own Back press.
//!
//! The right fix is an entry point the player drives, and it belongs to a neighbouring slice:
//! the lobby / slot picker (T-181.9.1) should offer a "View briefing" row that calls
//! `TBD_MenuStack.Open(ChimeraMenuPreset.TBD_UIBriefing)` — one line, no changes needed here.
//! A keybind would work equally well but needs an ActionContext `.conf`, which is blocked behind
//! the same Workbench `resourceDatabase.rdb` pass as the menu preset below.
//!
//! ── Where the data comes from, and why the screen is dumb ──────────────────────────────────
//! This screen renders a `TBD_BriefingPayload` and nothing else. It never reads
//! `TBD_MissionLoader` — it *cannot*, because a client has no mission document (see the header of
//! `TBD_BriefingData.c`). That is deliberate: with no second source to read, there is no path by
//! which this screen could display the other side's ORBAT even if it tried.
class TBD_BriefingScreen : TBD_ShellScreen
{
	//! Row tags. Negative = inert content, positive = something to open.
	protected static const int TAG_INERT = -1;
	protected static const int TAG_OWN_SEAT = 1;
	//! Group rows are TAG_GROUP_BASE + index, so a tag decodes to a group without a lookup table.
	protected static const int TAG_GROUP_BASE = 100;

	protected ref TBD_BriefingPayload m_Payload;

	//! Disclosure state, rebuilt whenever a new payload lands.
	protected bool m_bKitExpanded;
	protected ref array<bool> m_aGroupExpanded;

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenOpen()
	{
		super.OnScreenOpen();

		m_aGroupExpanded = {};

		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Insert(OnRowPicked);

		GetOnPrimaryAction().Insert(OnReadyPressed);

		// Re-render whenever the server answers, without the screen polling for it.
		TBD_BriefingClient.GetOnPayloadChanged().Insert(OnPayloadChanged);
		TBD_BriefingClient.GetOnReadyStateChanged().Insert(OnReadyStateChanged);

		AdoptPayload(TBD_BriefingClient.GetPayload());

		// Ask every time the screen opens: the ORBAT moves while players claim slots in the lobby,
		// so a cached payload from two minutes ago is not the briefing they should plan from.
		TBD_BriefingClient.Request();
	}

	//------------------------------------------------------------------------------------------------
	override protected void OnScreenClose()
	{
		TBD_ListBox list = GetList();
		if (list)
			list.GetOnActivate().Remove(OnRowPicked);

		GetOnPrimaryAction().Remove(OnReadyPressed);

		TBD_BriefingClient.GetOnPayloadChanged().Remove(OnPayloadChanged);
		TBD_BriefingClient.GetOnReadyStateChanged().Remove(OnReadyStateChanged);

		super.OnScreenClose();
	}

	//------------------------------------------------------------------------------------------------
	override protected string GetScreenTitle()
	{
		if (m_Payload && !m_Payload.m_sMissionName.IsEmpty())
			return m_Payload.m_sMissionName;

		return "BRIEFING";
	}

	//------------------------------------------------------------------------------------------------
	//! Mission identity in one line: who you are, where you are, what phase this is.
	override protected string GetScreenSubtitle()
	{
		if (!m_Payload)
			return "Requesting briefing…";

		string faction = m_Payload.m_sFactionName;
		if (faction.IsEmpty())
			faction = "Unassigned";

		if (m_Payload.m_sTerrain.IsEmpty())
			return string.Format("%1 · BRIEFING", faction);

		return string.Format("%1 · %2 · BRIEFING", faction, m_Payload.m_sTerrain);
	}

	// ── Payload ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected void OnPayloadChanged(TBD_BriefingPayload payload)
	{
		AdoptPayload(payload);
	}

	//------------------------------------------------------------------------------------------------
	//! Take a new payload and reset disclosure to the sensible default: the reader's own squad
	//! open, everything else closed.
	protected void AdoptPayload(TBD_BriefingPayload payload)
	{
		m_Payload = payload;
		m_bKitExpanded = false;

		m_aGroupExpanded.Clear();

		if (m_Payload)
		{
			foreach (TBD_BriefingGroup group : m_Payload.m_aGroups)
			{
				m_aGroupExpanded.Insert(group.m_bIsOwn);
			}
		}

		SetTitle(GetScreenTitle());
		SetSubtitle(GetScreenSubtitle());

		Rebuild();
		RefreshFooter();
	}

	// ── Rendering ───────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Write the whole list from the payload plus the disclosure flags. Cheap by construction —
	//! `TBD_ListBox` pools its rows, so a rebuild is property writes on widgets that already exist.
	protected void Rebuild()
	{
		TBD_ListBox list = GetList();
		if (!list)
			return;

		list.BeginUpdate();

		if (!m_Payload)
		{
			list.AddSection("Requesting briefing from the server…");
			list.EndUpdate();
			return;
		}

		if (!m_Payload.IsAvailable())
		{
			// An empty state says why. Never a void.
			list.AddSection(m_Payload.m_sUnavailableReason);
			list.EndUpdate();
			return;
		}

		EmitOwnSeat(list);
		EmitOrbat(list);
		EmitZones(list);
		EmitEndConditions(list);

		list.EndUpdate();
	}

	//------------------------------------------------------------------------------------------------
	//! Requirement 3 — which seat you hold, and its loadout summary when one is defined.
	protected void EmitOwnSeat(TBD_ListBox list)
	{
		if (!m_Payload.m_bHasSlot)
			return;

		list.AddSection("YOUR SEAT");

		string title = string.Format("%1 · %2", m_Payload.m_sOwnGroup, m_Payload.m_sOwnRole);

		bool hasKit = !m_Payload.m_aKit.IsEmpty();
		string detail = m_Payload.m_sOwnKit;

		if (hasKit)
			detail = string.Format("%1  %2", detail, DisclosureMark(m_bKitExpanded));

		// The seat row is only pickable when there is something behind it to disclose.
		list.AddItem(title, detail, TAG_OWN_SEAT, TBD_EUIState.ACTIVE, hasKit);

		if (!hasKit || !m_bKitExpanded)
			return;

		foreach (TBD_BriefingKitLine kit : m_Payload.m_aKit)
		{
			list.AddItem("    " + kit.m_sLabel, kit.m_sValue, TAG_INERT, TBD_EUIState.NORMAL, false);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Requirement 2 — their ORBAT, and only theirs. Squads collapse; the reader's own is open.
	protected void EmitOrbat(TBD_ListBox list)
	{
		if (m_Payload.m_aGroups.IsEmpty())
			return;

		int seats = 0;
		foreach (TBD_BriefingGroup counted : m_Payload.m_aGroups)
		{
			seats += counted.m_iSeats;
		}

		string heading = m_Payload.m_sFactionName;
		heading.ToUpper(); // in place — see TBD_BriefingService.Sanitise
		list.AddSection(string.Format("ORBAT — %1", heading), string.Format("%1 seats", seats));

		for (int i = 0; i < m_Payload.m_aGroups.Count(); i++)
		{
			TBD_BriefingGroup group = m_Payload.m_aGroups[i];
			bool expanded = IsGroupExpanded(i);

			string detail = string.Format("%1 seats", group.m_iSeats);
			if (group.m_bIsOwn)
				detail = detail + " · YOURS";

			detail = string.Format("%1  %2", detail, DisclosureMark(expanded));

			TBD_EUIState state = TBD_EUIState.NORMAL;
			if (group.m_bIsOwn)
				state = TBD_EUIState.ACTIVE;

			list.AddItem(group.m_sCallsign, detail, TAG_GROUP_BASE + i, state, true);

			if (!expanded)
				continue;

			foreach (TBD_BriefingRole role : group.m_aRoles)
			{
				string count = string.Format("x%1", role.m_iCount);
				TBD_EUIState roleState = TBD_EUIState.NORMAL;

				if (role.m_bIsOwn)
				{
					count = count + " · YOU";
					roleState = TBD_EUIState.ACTIVE;
				}

				list.AddItem("    " + role.m_sRole, count, TAG_INERT, roleState, false);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Requirement 4 — the ground that is theirs to think about. The other side's spawn was
	//! filtered out on the server and is not present in the payload at all.
	protected void EmitZones(TBD_ListBox list)
	{
		if (m_Payload.m_aZones.IsEmpty())
			return;

		list.AddSection("AREA OF OPERATIONS");

		foreach (TBD_BriefingZone zone : m_Payload.m_aZones)
		{
			TBD_EUIState state = TBD_EUIState.NORMAL;
			if (zone.m_bIsOwn)
				state = TBD_EUIState.ACTIVE;

			list.AddItem("    " + zone.m_sTitle, zone.m_sDetail, TAG_INERT, state, false);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! How the round ends. Shared by both sides, so nothing here is filtered — but a player being
	//! asked to plan deserves to know what they are planning toward.
	protected void EmitEndConditions(TBD_ListBox list)
	{
		if (m_Payload.m_sWinMode.IsEmpty() && m_Payload.m_aEndConditions.IsEmpty())
			return;

		list.AddSection("HOW THIS ENDS", m_Payload.m_sWinMode);

		foreach (string trigger : m_Payload.m_aEndConditions)
		{
			list.AddItem("    " + trigger, string.Empty, TAG_INERT, TBD_EUIState.NORMAL, false);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Requirement 5 — the ready-to-proceed affordance, and the non-blocking feedback under it.
	protected void RefreshFooter()
	{
		if (!m_Payload || !m_Payload.IsAvailable())
		{
			// Nothing to be ready for yet: show no loud button at all.
			SetPrimaryAction(string.Empty, false);

			if (m_Payload && !m_Payload.IsAvailable())
				SetStatus(m_Payload.m_sUnavailableReason);
			else
				SetStatus("Waiting for the server…");

			return;
		}

		if (TBD_BriefingClient.IsReady())
		{
			SetPrimaryAction("READY", false);
			SetStatus(TBD_BriefingClient.GetReadyTally());
			return;
		}

		SetPrimaryAction("I'M READY", true);

		string tally = TBD_BriefingClient.GetReadyTally();
		if (tally.IsEmpty())
			SetStatus("Read your orders, then mark ready.");
		else
			SetStatus(tally);
	}

	// ── Interaction ─────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! One click opens or closes one disclosure. Direct manipulation — no confirm step.
	protected void OnRowPicked(TBD_ListBox list, int tag)
	{
		if (tag == TAG_OWN_SEAT)
		{
			m_bKitExpanded = !m_bKitExpanded;
			Rebuild();
			return;
		}

		if (tag < TAG_GROUP_BASE)
			return;

		int index = tag - TAG_GROUP_BASE;
		if (index < 0 || index >= m_aGroupExpanded.Count())
			return;

		m_aGroupExpanded[index] = !m_aGroupExpanded[index];
		Rebuild();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnReadyPressed(TBD_ShellScreen screen)
	{
		if (TBD_BriefingClient.IsReady())
			return;

		TBD_BriefingClient.ReportReady();

		// Optimistic, immediate feedback; the server's tally overwrites this the moment it lands.
		SetPrimaryAction("READY", false);
		SetStatus("Ready — waiting for the rest of your side.");
	}

	//------------------------------------------------------------------------------------------------
	protected void OnReadyStateChanged(string tally)
	{
		RefreshFooter();
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected bool IsGroupExpanded(int index)
	{
		if (index < 0 || index >= m_aGroupExpanded.Count())
			return false;

		return m_aGroupExpanded[index];
	}

	//------------------------------------------------------------------------------------------------
	//! The only affordance a row has for "there is more behind me". Text, not an icon, because the
	//! shared row layout carries no image slot for one.
	//!
	//! Deliberately plain ASCII. Nothing here can render a framebuffer, so the UI font's glyph
	//! coverage is unverifiable from this lane — a geometric triangle that the font lacks would
	//! draw as a tofu box. `+`/`-` cannot miss.
	protected string DisclosureMark(bool expanded)
	{
		if (expanded)
			return "-";

		return "+";
	}
}

//! The briefing preset. Bound to a layout and this class in
//! `Configs/System/chimeraMenus.conf`.
//!
//! ── KNOWN BLOCKER, expected, not this slice's ──────────────────────────────────────────────
//! Adding this enum value and the `.conf` block is necessary but NOT sufficient. Until the
//! addon's `resourceDatabase.rdb` lists `Configs/System/chimeraMenus.conf`, the engine cannot see
//! the preset and logs, at every startup:
//!
//!     GUI       (E): Menu preset 'TBD_UIBriefing' not found!
//!
//! Only a Workbench pass regenerates that index; the headless compile lane cannot. Everything in
//! this slice compiles and is structurally complete; the screen cannot OPEN until that one pass.
//! See the measured note in `TBD_UILayouts` — the same wall the shell hit at T-181.7.
modded enum ChimeraMenuPreset
{
	TBD_UIBriefing
}
