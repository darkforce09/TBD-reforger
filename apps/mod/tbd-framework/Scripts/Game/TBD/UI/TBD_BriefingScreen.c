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
//!   │  SITUATION                                                 │  <- T-181.27 written orders
//!   │    Soviet airborne forces hold the Levie bridge crossing.  │
//!   │  MISSION                                                   │
//!   │    Seize and hold Levie Bridge before the time limit       │
//!   │    expires.                                                │  <- wrapped, not clipped
//!   │  EXECUTION                                                 │
//!   │    Alpha advances from the western treeline under MG       │
//!   │    support.                                                │
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

	//! T-181.27 — width of one rendered line of orders prose, in BYTES.
	//!
	//! Bytes rather than characters because that is what `string.Length()` actually returns —
	//! measured on a live boot: `"…".Length()` is 3 and `"·".Length()` is 2. Accented prose
	//! therefore wraps a little earlier than 64 glyphs, which is the harmless direction to err.
	//!
	//! **The number itself is an ESTIMATE and cannot be verified from this lane.** Measuring it
	//! needs a rendered frame and nothing here returns one — the screen has never opened (the
	//! `resourceDatabase.rdb` blocker at the foot of this file). It is deliberately conservative:
	//! erring short costs one extra row per paragraph, while erring long would clip words off the
	//! right edge of a row whose width nothing here can query. The first operator pass on the real
	//! screen is where this gets its true value.
	protected static const int ORDERS_WRAP_WIDTH = 64;

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
		EmitOrders(list);
		EmitOrbat(list);
		EmitZones(list);
		EmitEndConditions(list);

		list.EndUpdate();
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.27 — the written orders, in the order an OPORD is read: what is happening, what we
	//! must achieve, how we are to do it. This is the Arma 3 briefing text the screen exists for;
	//! everything around it is structure.
	//!
	//! Placed after YOUR SEAT and before the ORBAT deliberately: the reader wants to know who they
	//! are, then the plan, then the detail of who is with them.
	//!
	//! ── Empty is not a heading ──────────────────────────────────────────────────
	//! Each of the three is INDEPENDENTLY optional — `briefing` declares no `required` in the
	//! schema, and `required` would not have meant non-empty even if it did. A mission may author
	//! `mission` and nothing else. Each section is emitted only when it has content, so an
	//! unauthored field costs exactly zero rows: no heading, no blank line, no trace. A side with
	//! no orders at all shows nothing here and the screen reads as if the section were never
	//! designed.
	//!
	//! Nothing is filtered at this point and nothing needs to be. The payload was built for one
	//! faction on the server, so the other side's orders are not in this process to render.
	protected void EmitOrders(TBD_ListBox list)
	{
		EmitOrderSection(list, "SITUATION", m_Payload.m_aSituation);
		EmitOrderSection(list, "MISSION", m_Payload.m_aMission);
		EmitOrderSection(list, "EXECUTION", m_Payload.m_aExecution);
	}

	//------------------------------------------------------------------------------------------------
	//! One heading and its prose, or nothing at all. A CONTENT test — the arrays are allocated by
	//! `TBD_BriefingPayload`'s constructor and are never null, so a null test here would always be
	//! false and would read as a presence check while being none.
	protected void EmitOrderSection(TBD_ListBox list, string heading, array<string> paragraphs)
	{
		if (paragraphs.Count() == 0)
			return;

		list.AddSection(heading);

		foreach (string paragraph : paragraphs)
		{
			array<string> wrapped = WrapText(paragraph, ORDERS_WRAP_WIDTH);

			foreach (string line : wrapped)
			{
				// Inert: prose is to be read, not picked. The whole line goes in the title column
				// because the detail column is a right-aligned value slot, not a second text lane.
				list.AddItem("    " + line, string.Empty, TAG_INERT, TBD_EUIState.NORMAL, false);
			}
		}
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
	//! Greedy word wrap into rows of at most `width` bytes.
	//!
	//! ── Hand-rolled, and not on `string.Split` ──────────────────────────────────
	//! Same reason as `TBD_BriefingService.SplitLines`: `Split`'s empty-token behaviour is a
	//! runtime property this lane cannot settle, and prose is full of double spaces. This loop
	//! skips empty words EXPLICITLY, so it produces identical output whether `Split` would have
	//! emitted them or swallowed them — the ambiguity is defused rather than inherited. It uses
	//! only `IndexOf` / `Substring` / `Length`, all already load-bearing in shipped code.
	//!
	//! A word longer than the whole line is hard-split rather than dropped or allowed to overflow:
	//! a resource path or a coordinate string pasted into orders still renders every character.
	//! **Known edge, stated rather than hidden:** `Substring` is byte-indexed (measured), so that
	//! hard split can sever a multi-byte character and cost one replacement glyph. It needs a
	//! single unbroken 64-byte token to trigger, which prose does not contain; the budget
	//! truncation on the server, which real text CAN reach, backs off to a space for this reason
	//! (`TBD_BriefingService.ClipToWord`). Splitting beats the alternatives — dropping the token
	//! loses data, and letting it overflow hands it to a widget whose clipping nothing here can
	//! observe.
	//!
	//! Terminating: every iteration consumes at least one byte of `rest` (an empty word is only
	//! produced by a separator that was itself consumed), so `rest` strictly shrinks.
	protected array<string> WrapText(string text, int width)
	{
		array<string> lines = {};

		// A width this small would make the hard-split loop below degenerate. Nothing sane reaches
		// it; the guard is here so that a future retune cannot turn a constant into a hang.
		if (text.IsEmpty() || width < 8)
		{
			lines.Insert(text);
			return lines;
		}

		string line;
		string rest = text;

		while (!rest.IsEmpty())
		{
			string word;

			int space = rest.IndexOf(" ");
			if (space < 0)
			{
				word = rest;
				rest = string.Empty;
			}
			else
			{
				word = rest.Substring(0, space);
				rest = rest.Substring(space + 1, rest.Length() - space - 1);
			}

			if (word.IsEmpty())
				continue; // a run of spaces — collapse it, do not emit a blank row

			while (word.Length() > width)
			{
				if (!line.IsEmpty())
				{
					lines.Insert(line);
					line = string.Empty;
				}

				lines.Insert(word.Substring(0, width));
				word = word.Substring(width, word.Length() - width);
			}

			if (line.IsEmpty())
			{
				line = word;
				continue;
			}

			int projected = line.Length() + 1;
			projected += word.Length();

			if (projected <= width)
			{
				line = line + " " + word;
				continue;
			}

			lines.Insert(line);
			line = word;
		}

		if (!line.IsEmpty())
			lines.Insert(line);

		return lines;
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
