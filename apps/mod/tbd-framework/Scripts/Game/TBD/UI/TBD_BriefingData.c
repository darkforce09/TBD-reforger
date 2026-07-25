//! T-181.9.2 — the briefing payload, and the place side discipline is actually enforced.
//!
//! ── The fact this whole design is built on ──────────────────────────────────────────────────
//! **A client has no mission document.** `TBD_FrameworkManager.OnPostInit` returns early for
//! `RplMode.Client` *before* `TBD_MissionLoader.BeginLoad()`, so on a dedicated server
//! `TBD_MissionLoader.GetMission()` is null on every client, forever. `TBD_SpawnManager`'s
//! `m_mPlayerSlot` is a plain `map`, not an `RplProp`, so `GetAssignedSlot()` is server-only too.
//!
//! A briefing screen is a client-side menu. It therefore cannot read the mission at all — it can
//! only render what the server chose to send it.
//!
//! That constraint is a gift, because it makes side discipline structural instead of cosmetic.
//!
//! ── Side discipline: enforced at the wire, not at the widget ────────────────────────────────
//! Three properties, in decreasing order of how much work they do:
//!
//!  1. **The other side's ORBAT is never serialised into this player's payload.**
//!     `BuildForPlayer` resolves the faction from the server's OWN slot assignment and filters
//!     every group, role and zone by it. Bytes describing the enemy ORBAT never enter the string.
//!  2. **The delivery is a targeted RPC.** `RplRcver.Owner` on `SCR_PlayerController` sends to
//!     exactly one client — the requester — not a broadcast a sniffer could read.
//!  3. **The request carries no faction.** `TBD_RpcAsk_Briefing()` takes no arguments. There is
//!     no API surface through which a modified client can name a side; the server derives it
//!     from state the client cannot influence.
//!
//! So the leak paths that plague a render-time filter — a UI toggle, a console command, a
//! detached widget handler, reading the doc directly — are all absent by construction. There is
//! no enemy ORBAT on the client to reveal.
//!
//! **Fail closed:** a player with no assigned slot has no faction, and gets an empty briefing
//! that says so. Unknown side is never treated as "show everything".
//!
//! ── T-181.27: the WRITTEN ORDERS obey exactly the same three properties ─────────────────
//! `briefings` is `map<string, TBD_MissionBriefingStruct>` keyed by faction, so `situation` /
//! `mission` / `execution` are side-scoped intelligence in the same sense the ORBAT is — and more
//! sensitive in practice, because prose states intent. `golden-missions/bridgehead-at-levie.json`
//! is the proof by example: BLUFOR is told *"Alpha advances from the western treeline under MG
//! support"* while OPFOR is told *"Grom defends the eastern bank and the checkpoint"*. Shipping
//! both to both sides and filtering in the widget would hand each side the other's scheme of
//! manoeuvre at the one bridge that decides the round.
//!
//! `BuildOrders` therefore reads `TBD_MissionLoader.GetBriefingForFaction(own.faction)` and
//! nothing else. **A client asking for the other side's orders cannot phrase the question** —
//! `TBD_RpcAsk_Briefing()` still takes no arguments, so there is nowhere to put a faction, and the
//! answer is whatever the server's own `GetAssignedSlot` says the caller is. No slot means no
//! side, which means no orders: `BuildForPlayer` returns before `BuildOrders` is ever reached.
//!
//! ── Why the ORBAT is derived from `slots[]`, not from `orbat` ───────────────────────────────
//! This started as a limitation and is now a deliberate choice. T-181.23 modelled the missing
//! `callsign` / `type` / `slot` / `kit` keys on `TBD_MissionOrbatGroupStruct` and
//! `TBD_MissionOrbatRoleStruct`, so rendering from `orbat` would no longer produce the nameless
//! list it once would have.
//!
//! Deriving from `slots[]` is still correct, for a reason that never depended on what was
//! modelled: `slots[]` is the FLATTENED form carrying `faction`, `groupCallsign`, `role`, `kit`
//! and the loadout in full, and it is the same array `TBD_SpawnManager` actually spawns from.
//! The briefing therefore shows what will exist in the world, not what a parallel block claims —
//! and the two can legitimately disagree, which is precisely what `TBD_MissionValidator`'s
//! ORBAT/slots parity check exists to catch.

//! One role line inside a group: "RFL ×4", flagged when the reader's own seat sits on it.
class TBD_BriefingRole
{
	string m_sRole;
	int m_iCount;
	bool m_bIsOwn;

	//------------------------------------------------------------------------------------------------
	void TBD_BriefingRole(string role, int count, bool isOwn)
	{
		m_sRole = role;
		m_iCount = count;
		m_bIsOwn = isOwn;
	}
}

//! One ORBAT group (squad) of the reader's faction.
class TBD_BriefingGroup
{
	string m_sCallsign;
	int m_iSeats;
	bool m_bIsOwn; //!< the reader's own squad — starts expanded (progressive disclosure default)
	ref array<ref TBD_BriefingRole> m_aRoles;

	//------------------------------------------------------------------------------------------------
	void TBD_BriefingGroup(string callsign)
	{
		m_sCallsign = callsign;
		m_aRoles = {};
	}

	//------------------------------------------------------------------------------------------------
	//! Fold one more seat of `role` into this group, creating the role line on first sight.
	void AddSeat(string role, bool isOwn)
	{
		m_iSeats++;

		foreach (TBD_BriefingRole existing : m_aRoles)
		{
			if (existing.m_sRole != role)
				continue;

			existing.m_iCount++;
			if (isOwn)
				existing.m_bIsOwn = true;

			return;
		}

		m_aRoles.Insert(new TBD_BriefingRole(role, 1, isOwn));
	}
}

//! One AO entry the reader is allowed to see.
class TBD_BriefingZone
{
	string m_sTitle;
	string m_sDetail;
	bool m_bIsOwn; //!< belongs to the reader's faction (their spawn), vs. shared (objective/boundary)

	//------------------------------------------------------------------------------------------------
	void TBD_BriefingZone(string title, string detail, bool isOwn)
	{
		m_sTitle = title;
		m_sDetail = detail;
		m_bIsOwn = isOwn;
	}
}

//! One "label: value" line of the reader's own loadout.
class TBD_BriefingKitLine
{
	string m_sLabel;
	string m_sValue;

	//------------------------------------------------------------------------------------------------
	void TBD_BriefingKitLine(string label, string value)
	{
		m_sLabel = label;
		m_sValue = value;
	}
}

//! Everything one player is permitted to read before the round goes live. Built on the server,
//! shipped as one string, rebuilt on the client. Never contains another faction's data.
class TBD_BriefingPayload
{
	// ── Mission identity ──────────────────────────────────────────────────────────────────────
	string m_sMissionName;
	string m_sTerrain;
	string m_sFactionKey;
	string m_sFactionName;

	// ── The reader's own seat ─────────────────────────────────────────────────────────────────
	bool m_bHasSlot;
	string m_sOwnGroup;
	string m_sOwnRole;
	string m_sOwnKit;
	ref array<ref TBD_BriefingKitLine> m_aKit;

	// ── Their side, and their ground ──────────────────────────────────────────────────────────
	ref array<ref TBD_BriefingGroup> m_aGroups;
	ref array<ref TBD_BriefingZone> m_aZones;

	// ── T-181.27 — their WRITTEN ORDERS ───────────────────────────────────────────
	//! One entry per authored PARAGRAPH, in document order. Empty means "this side authored none",
	//! which is the same rendering outcome as "the key was absent" and as "the key was blank" —
	//! three legal states, one honest answer: show nothing at all, never a blank heading.
	//!
	//! Paragraphs rather than one blob because the wire cannot carry a newline (see the note above
	//! `SplitLines`), and because a row-based list wants discrete units anyway.
	ref array<string> m_aSituation;
	ref array<string> m_aMission;
	ref array<string> m_aExecution;

	// ── How the round ends. Not faction-specific: both sides play the same win condition, so
	//    there is nothing to filter here.
	string m_sWinMode;
	ref array<string> m_aEndConditions;

	//! Set when the server could not answer (no mission, no slot). The screen shows this instead
	//! of an empty frame — design law: an empty state says why, it never shows a void.
	string m_sUnavailableReason;

	//------------------------------------------------------------------------------------------------
	void TBD_BriefingPayload()
	{
		m_aKit = {};
		m_aGroups = {};
		m_aZones = {};
		m_aEndConditions = {};
		m_aSituation = {};
		m_aMission = {};
		m_aExecution = {};
	}

	//------------------------------------------------------------------------------------------------
	bool IsAvailable()
	{
		return m_sUnavailableReason.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! True when this side authored at least one paragraph of orders.
	//!
	//! A CONTENT test on purpose. The arrays are allocated in the constructor and are therefore
	//! never null, so a null test here would be one of the dead guards this file already carries a
	//! warning about — it would read as a presence check while always being true.
	bool HasOrders()
	{
		return OrderParagraphCount() > 0;
	}

	//------------------------------------------------------------------------------------------------
	int OrderParagraphCount()
	{
		int n = m_aSituation.Count();
		n += m_aMission.Count();
		n += m_aExecution.Count();
		return n;
	}
}

//! Builds a briefing on the server, moves it over one RPC, and rebuilds it on the client.
//!
//! The wire format is line-based with tab-separated fields, matching the precedent already in
//! the tree (`TBD_MissionBrowserService`): it keeps the RPC signature to a single string, needs
//! no schema registration, and is greppable in a log when something goes wrong.
//!
//! ══ T-181.26 — every field carries a MARKER, so no field is ever the empty string ═══════════
//! `string.Split(sep, out, trim)` is a NATIVE engine call. Whether it emits a token for an empty
//! field between two separators is a RUNTIME property; no compile probe on this lane can settle
//! it and no oracle documents it (SLICE_WORKFLOW.md §What agents cannot do). Until this slice the
//! briefing wire simply BET on the answer, and lost either way:
//!
//!   * if `Split` DROPS empties, an empty field shortens the record and every later field shifts
//!     left. `Record3("G", callsign, seats, own)` with a blank callsign arrives as three tokens,
//!     fails the `f.Count() >= 4` guard, and the group line vanishes — taking its identity with
//!     it while its `R` role lines still arrive and fold into whichever group came BEFORE it.
//!     That is a squad's seats attributed to another squad, which is the one error a briefing
//!     must not make;
//!   * if it KEEPS them, the same record decodes correctly. Two opposite outcomes from a
//!     behaviour nobody in this program has ever observed.
//!
//! The fix removes the question rather than answering it. `Field()` writes `<TAB>.<value>`, so the
//! smallest token any field can produce is the one-character string `.` — which no tokeniser can
//! drop and no trim can erase. `Unmark()` strips the marker back off. The format is now correct
//! under BOTH behaviours, and an EMPTY field is distinguishable from a MISSING one.
//!
//! ── Why the marker and not `TBD_LobbyData`'s `~` sentinel ───────────────────────────────────
//! Both schemes exist in the tree. `TBD_LobbyData.EMPTY = "~"` maps empty->`~` on write and
//! `~`->empty on read, and defends the choice as "`~` is not a plausible whole-field value". That
//! is a plausibility argument, and it is LOSSY where it is wrong: a field authored as literally
//! `~` round-trips to the empty string. The briefing carries the most free-authored text of the
//! three delimited payloads — `meta.name` (120 chars of anything), `zone.label`, a faction
//! `displayName` — so it is the worst place to rest on what an author would not type.
//! `TBD_AdminData.FIELD_MARK` is BIJECTIVE instead: `Unmark(Field(x)) == x` for every `x`,
//! including `.` and the empty string, at a cost of one byte per field. This file takes that one.
//! Recorded so the next reader knows the divergence is a decision, not drift — and so the command
//! centre can converge `TBD_LobbyData` onto the same helper rather than keeping three answers.
//!
//! ── Why the marker must stay a single ASCII byte ────────────────────────────────────────────
//! `Unmark` is `Substring(1, length - 1)`, and MEASURED: `string.Length()` counts BYTES and
//! `Substring` is BYTE-indexed (`"·".Length()` is 2; `"café latte".Substring(0, 4)` returns a
//! broken UTF-8 sequence). A one-byte ASCII marker is therefore skipped safely no matter what the
//! value's own encoding is. A prettier multi-byte marker would corrupt every accented field.
class TBD_BriefingService
{
	//! Log channel. Deliberately a local constant rather than an edit to `TBD_Log`'s vocabulary —
	//! two slices adding a channel to that one enum block in the same wave is a merge conflict for
	//! no benefit. Fold it into `TBD_Log.CH_*` when the UI slices are next consolidated.
	static const string CH_BRIEFING = "Briefing";

	//! Defensive cap on one payload, mirroring `TBD_MissionBrowserService.MAX_LIST_LINES`. A
	//! pathological mission must not become an unbounded reliable-channel string.
	protected static const int MAX_PAYLOAD_LINES = 400;

	protected static const string FIELD_SEP = "\t";
	protected static const string LINE_SEP = "\n";

	//! T-181.26 — the per-field marker. See the class header for why this exists and why it is one
	//! ASCII byte. Same character and same semantics as `TBD_AdminData.FIELD_MARK`, deliberately:
	//! one convention, not a fourth.
	protected static const string FIELD_MARK = ".";

	//! T-181.27 — bounds on one side's written orders. The schema puts no `maxLength` on any of
	//! the three fields, so a pathological document could otherwise push an unbounded string down a
	//! reliable channel. Both caps are generous next to a real OPORD (a full Arma 3 briefing runs
	//! well under 2,000 bytes) and neither is ever applied SILENTLY — see `WarnOnce`.
	//!
	//! BYTES, not characters: `string.Length()` was measured returning 3 for `"…"` and 2 for `"·"`.
	//! Accented prose therefore spends the budget slightly faster than its glyph count suggests,
	//! which errs on the safe side for a reliable channel.
	protected static const int MAX_ORDER_CHARS = 6000;
	protected static const int MAX_ORDER_PARAGRAPHS = 16;

	//! Smallest remaining budget worth spending on a paragraph. Below this the field is dropped
	//! whole (and warned about) instead of rendering a stub too short to mean anything.
	protected static const int MIN_ORDER_TAIL = 24;

	//! Truncation warnings already emitted, keyed `faction|field`. A briefing is re-requested every
	//! time the screen opens, so an ungated warn would let one over-long mission fill the console.
	//! Same defect `TBD_MarkerService.ShouldLog` exists to avoid, and the same fix.
	protected static ref map<string, bool> s_mWarned;

	//! Ceiling on that table. Bounded by (factions x 3 fields) in practice; dropped wholesale rather
	//! than leaked if a long session of mission switches ever grows it past this.
	protected static const int MAX_WARN_STATES = 64;

	// ── SERVER ──────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Build the briefing this player — and only this player — is entitled to.
	//! @authority server — reads `TBD_SpawnManager` and `TBD_MissionLoader`, neither of which
	//! exists on a client.
	static TBD_BriefingPayload BuildForPlayer(int playerId)
	{
		TBD_BriefingPayload payload = new TBD_BriefingPayload();

		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !TBD_MissionLoader.IsValid())
		{
			payload.m_sUnavailableReason = "Mission is still loading.";
			return payload;
		}

		if (doc.meta)
		{
			payload.m_sMissionName = Sanitise(doc.meta.name);
			payload.m_sTerrain = Sanitise(doc.meta.terrain);
		}

		// ── The one question that decides everything below: which side is this player on? ──
		// Answered from server-owned state only. The client never supplies it.
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		TBD_MissionSlotStruct own;
		if (spawn)
			own = spawn.GetAssignedSlot(playerId);

		if (!own)
		{
			// FAIL CLOSED. No slot means no side, and no side means no ORBAT — not "all of it".
			payload.m_sUnavailableReason = "No slot assigned yet. Claim a slot in the lobby first.";
			return payload;
		}

		// T-181.26 — SANITISED like every other authored string that reaches the payload. It was the
		// one that was not, and the local-file load path (`TBD_MissionLoader.LoadFromProfileFile`)
		// applies NO json-schema validation, so `slot.faction`'s `^[a-z][a-z0-9_]*$` pattern is not
		// enforced on a hand-staged mission. Only the display copy is flattened; the comparisons
		// below (`BuildOrders` / `BuildOrbat` / `BuildZones`) keep the raw key, because a faction
		// must match itself exactly or side discipline stops meaning anything.
		payload.m_sFactionKey = Sanitise(own.faction);
		payload.m_sFactionName = ResolveFactionName(doc, own.faction);
		payload.m_bHasSlot = true;
		payload.m_sOwnGroup = Sanitise(own.groupCallsign);
		payload.m_sOwnRole = Sanitise(own.role);
		payload.m_sOwnKit = Sanitise(own.kit);

		BuildKit(payload, own);
		BuildOrders(payload, own.faction);
		BuildOrbat(payload, doc, own);
		BuildZones(payload, doc, own.faction);
		BuildEndConditions(payload, doc);

		return payload;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.27 — the WRITTEN ORDERS for the reader's side, and no other side's.
	//!
	//! ── Why this is the same side-discipline boundary as the ORBAT ──────────────────────
	//! `briefings` is keyed by faction exactly like `orbat`, so orders are SIDE-SCOPED
	//! INTELLIGENCE — "Grom defends the eastern bank and the checkpoint" is precisely what BLUFOR
	//! must not read. The key handed to `GetBriefingForFaction` is `own.faction`, resolved from
	//! `TBD_SpawnManager.GetAssignedSlot(playerId)` on the server. The other side's prose is never
	//! read out of the document, so it never enters the payload, the RPC or the screen.
	//!
	//! ── The three legal empty states, all rendering nothing ──────────────────────────
	//! `briefing` declares NO `required` in the schema, so every field is optional and `required`
	//! would not have meant non-empty even if it were there:
	//!   1. no `briefings` block at all        -> `GetBriefingForFaction` returns null;
	//!   2. a block with no entry for our side -> `GetBriefingForFaction` returns null;
	//!   3. an entry with the key absent, or present and blank -> empty string.
	//! `golden-missions/empty-warning-fields.json` ships states 2 and 3 side by side: `opfor` is
	//! `{}` and `blufor` has `situation`/`mission`/`execution` all `""`. All of them must produce
	//! ZERO paragraphs, so the screen emits no section and therefore no blank heading.
	//!
	//! Note what is NOT tested here: nullness of a nested `ref` field. `GetBriefingForFaction`
	//! returns null from a MAP LOOKUP MISS, which is a real signal; the struct's own string fields
	//! are then tested for CONTENT, never for null. `JsonLoadContext` allocates a nested `ref`
	//! whether or not the key was present, so a null test on one is always false and tells you
	//! nothing — the landmine documented on `TBD_MissionShapeStruct`.
	//! @authority server
	protected static void BuildOrders(TBD_BriefingPayload payload, string factionKey)
	{
		TBD_MissionBriefingStruct briefing = TBD_MissionLoader.GetBriefingForFaction(factionKey);
		if (!briefing)
			return; // this mission authored no orders for this side. Legal, and not an error.

		// One shared budget across the three fields so a single pathological paragraph cannot
		// crowd out the other two sections, and the whole block stays bounded on a reliable channel.
		int budget = MAX_ORDER_CHARS;
		budget = AppendParagraphs(payload.m_aSituation, briefing.situation, budget, factionKey, "situation");
		budget = AppendParagraphs(payload.m_aMission, briefing.mission, budget, factionKey, "mission");
		AppendParagraphs(payload.m_aExecution, briefing.execution, budget, factionKey, "execution");
	}

	//------------------------------------------------------------------------------------------------
	//! Split one authored field into display paragraphs, and return what is left of the budget.
	//!
	//! Blank paragraphs are dropped — an author's double newline between paragraphs is a separator,
	//! not an empty line to render.
	protected static int AppendParagraphs(array<string> destination, string raw, int budget, string factionKey, string field)
	{
		if (raw.IsEmpty())
			return budget; // CONTENT test: absent key and authored-blank are the same thing here.

		array<string> parts = SplitLines(raw);
		int kept = 0;

		foreach (string part : parts)
		{
			string paragraph = TrimSpaces(Sanitise(part));
			if (paragraph.IsEmpty())
				continue;

			if (kept >= MAX_ORDER_PARAGRAPHS)
			{
				WarnOnce(factionKey, field, string.Format(
					"faction '%1' authored more than %2 paragraphs of %3; the rest are not shown.",
					factionKey, MAX_ORDER_PARAGRAPHS, field));
				break;
			}

			// Not `budget <= 0`: a handful of bytes left would render the field as a meaningless
			// stub ("Alp…" was observed in a probe log). Below a useful remainder, drop the rest of
			// the field and say so, rather than showing a fragment that reads like corruption.
			if (budget < MIN_ORDER_TAIL)
			{
				WarnOnce(factionKey, field, string.Format(
					"faction '%1' orders exceed the %2-byte budget; %3 was cut short.",
					factionKey, MAX_ORDER_CHARS, field));
				break;
			}

			if (paragraph.Length() > budget)
			{
				paragraph = ClipToWord(paragraph, budget) + "…";
				WarnOnce(factionKey, field, string.Format(
					"faction '%1' orders exceed the %2-byte budget; %3 was truncated.",
					factionKey, MAX_ORDER_CHARS, field));
			}

			destination.Insert(paragraph);
			budget -= paragraph.Length();
			kept++;
		}

		return budget;
	}

	//------------------------------------------------------------------------------------------------
	//! How the round ends. An Arma 3 briefing always states the win condition, and the mission
	//! document already carries it — a planning screen that omits it is asking players to plan
	//! blind.
	//!
	//! Both sides share one win condition, so unlike the ORBAT and the zones there is nothing to
	//! filter. `endOn` values are the schema enum (`time_limit`, `all_objectives_captured`,
	//! `faction_eliminated`, …); TBD one-life events only *evaluate* `faction_eliminated` today,
	//! but every declared trigger is shown, because hiding an authored condition would misinform
	//! the plan.
	protected static void BuildEndConditions(TBD_BriefingPayload payload, TBD_MissionDocumentStruct doc)
	{
		if (!doc.winConditions)
			return;

		payload.m_sWinMode = Humanise(doc.winConditions.mode);

		if (!doc.winConditions.endOn)
			return;

		foreach (string trigger : doc.winConditions.endOn)
		{
			if (!trigger.IsEmpty())
				payload.m_aEndConditions.Insert(Humanise(trigger));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The reader's own loadout, if the mission defined one. Only non-empty gear is listed —
	//! progressive disclosure means a slot with three items shows three lines, not ten blanks.
	protected static void BuildKit(TBD_BriefingPayload payload, TBD_MissionSlotStruct own)
	{
		if (!own.loadout)
			return;

		TBD_SlotGearStruct gear = own.loadout.gear;
		if (gear)
		{
			AddKitLine(payload, "Primary", gear.primary);
			AddKitLine(payload, "Optic", gear.optic);
			AddKitLine(payload, "Magazine", gear.magazine);
			AddKitLine(payload, "Uniform", gear.uniform);
			AddKitLine(payload, "Vest", gear.vest);
			AddKitLine(payload, "Helmet", gear.helmet);
			AddKitLine(payload, "Backpack", gear.backpack);
		}

		if (!own.loadout.cargo || own.loadout.cargo.IsEmpty())
			return;

		// Cargo is summarised, not enumerated: the briefing answers "am I carrying supplies",
		// and the full manifest belongs in the arsenal, not on a planning screen.
		int units = 0;
		foreach (TBD_SlotCargoStruct row : own.loadout.cargo)
		{
			if (row)
				units += row.qty;
		}

		payload.m_aKit.Insert(new TBD_BriefingKitLine("Cargo",
			string.Format("%1 item(s), %2 unit(s)", own.loadout.cargo.Count(), units)));
	}

	//------------------------------------------------------------------------------------------------
	protected static void AddKitLine(TBD_BriefingPayload payload, string label, string resource)
	{
		if (resource.IsEmpty())
			return;

		payload.m_aKit.Insert(new TBD_BriefingKitLine(label, PrettyResourceName(resource)));
	}

	//------------------------------------------------------------------------------------------------
	//! Fold the flattened slot array into groups -> roles, **for one faction only**.
	//!
	//! This loop is the side-discipline boundary. A slot whose faction differs from the reader's
	//! is skipped before anything about it is recorded, so it cannot reach the payload, the wire,
	//! or the screen.
	protected static void BuildOrbat(TBD_BriefingPayload payload, TBD_MissionDocumentStruct doc, TBD_MissionSlotStruct own)
	{
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots)
			return;

		string ownKey = own.Key();

		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;

			// ── THE FILTER. Everything downstream is already same-side. ──
			if (slot.faction != own.faction)
				continue;

			TBD_BriefingGroup group = AcquireGroup(payload, Sanitise(slot.groupCallsign));
			bool isOwnSeat = slot.Key() == ownKey;

			group.AddSeat(Sanitise(slot.role), isOwnSeat);

			if (isOwnSeat)
				group.m_bIsOwn = true;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_BriefingGroup AcquireGroup(TBD_BriefingPayload payload, string callsign)
	{
		foreach (TBD_BriefingGroup existing : payload.m_aGroups)
		{
			if (existing.m_sCallsign == callsign)
				return existing;
		}

		payload.m_aGroups.Insert(new TBD_BriefingGroup(callsign));
		return payload.m_aGroups[payload.m_aGroups.Count() - 1];
	}

	//------------------------------------------------------------------------------------------------
	//! Zones the reader may see: their own faction's, plus every shared one (objectives,
	//! boundary). A zone belonging to the OTHER faction — notably the enemy spawn — is dropped
	//! here and therefore never crosses the wire. Knowing where the enemy spawns is exactly the
	//! kind of thing a briefing must not leak.
	protected static void BuildZones(TBD_BriefingPayload payload, TBD_MissionDocumentStruct doc, string factionKey)
	{
		if (!doc.zones)
			return;

		foreach (TBD_MissionZoneStruct zone : doc.zones)
		{
			if (!zone)
				continue;

			bool isOwn = zone.faction == factionKey;
			bool isShared = zone.faction.IsEmpty();

			if (!isOwn && !isShared)
				continue; // the other side's ground — not ours to show

			// T-181.18 — BOTH claims in the comment that used to sit here were wrong, and the
			// code below followed them. Polygon zones do NOT parse to a null circle:
			// `JsonLoadContext` allocates a nested `ref` field whether or not the JSON key was
			// present, so `zone.shape.circle` is ALWAYS non-null and this branch was ALWAYS
			// taken — a polygon-only boundary rendered as the literal "0, 0 · r0" rather than
			// falling through to "area". And polygons are modelled as of T-181.18, so there are
			// real vertices to describe. Test CONTENT, never non-null; see the landmine on
			// TBD_MissionShapeStruct in TBD_MissionLoader.c.
			string detail = "area";
			if (zone.shape)
			{
				if (zone.shape.circle && zone.shape.circle.r > 0)
				{
					detail = string.Format("%1, %2 · r%3",
						Math.Round(zone.shape.circle.x),
						Math.Round(zone.shape.circle.z),
						Math.Round(zone.shape.circle.r));
				}
				else if (zone.shape.polygon && zone.shape.polygon.Count() > 0)
				{
					detail = string.Format("area · %1 pts", zone.shape.polygon.Count());
				}
			}

			payload.m_aZones.Insert(new TBD_BriefingZone(PrettyZoneTitle(zone), detail, isOwn));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The authored name when the mission gave one ("Levie Bridge"), else the honest fallback built
	//! from type + id ("Objective capture — z3").
	//!
	//! T-181.23 modelled `label` on `TBD_MissionZoneStruct`, so the human name the mission author
	//! actually wrote is finally reachable here. The key stays OPTIONAL in the schema — spawn and
	//! boundary zones routinely omit it — so the type+id fallback is kept, not replaced.
	protected static string PrettyZoneTitle(TBD_MissionZoneStruct zone)
	{
		if (!zone.label.IsEmpty())
			return Sanitise(zone.label);

		string label = Humanise(zone.type);
		if (zone.id.IsEmpty())
			return label;

		return string.Format("%1 — %2", label, Sanitise(zone.id));
	}

	//------------------------------------------------------------------------------------------------
	protected static string ResolveFactionName(TBD_MissionDocumentStruct doc, string factionKey)
	{
		if (doc.factions)
		{
			foreach (TBD_MissionFactionStruct faction : doc.factions)
			{
				if (faction && faction.key == factionKey && !faction.displayName.IsEmpty())
					return Sanitise(faction.displayName);
			}
		}

		return Sanitise(factionKey);
	}

	// ── WIRE ────────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Flatten a payload to one string. Record types:
	//!   `M` mission   name / terrain / factionKey / factionName
	//!   `X` unavailable reason (terminal — nothing else follows)
	//!   `S` own seat  group / role / kit
	//!   `K` kit line  label / value
	//!   `G` group     callsign / seats / isOwn      (subsequent `R` lines attach to it)
	//!   `R` role      role / count / isOwn
	//!   `Z` zone      title / detail / isOwn
	//!   `W` win mode  label
	//!   `E` end-on    one declared round-end trigger
	//!
	//! The WRITTEN ORDERS are deliberately absent from this record set — they ride parallel
	//! `array<string>` RPC parameters instead. See `AdoptOrders` for why free prose must not be
	//! put through a delimited format.
	static string Serialise(TBD_BriefingPayload payload)
	{
		if (!payload)
			return string.Empty;

		array<string> lines = {};

		lines.Insert(Record4("M", payload.m_sMissionName, payload.m_sTerrain, payload.m_sFactionKey, payload.m_sFactionName));

		if (!payload.IsAvailable())
		{
			lines.Insert(Record1("X", payload.m_sUnavailableReason));
			return Join(lines);
		}

		lines.Insert(Record3("S", payload.m_sOwnGroup, payload.m_sOwnRole, payload.m_sOwnKit));

		foreach (TBD_BriefingKitLine kit : payload.m_aKit)
		{
			lines.Insert(Record2("K", kit.m_sLabel, kit.m_sValue));
		}

		foreach (TBD_BriefingGroup group : payload.m_aGroups)
		{
			lines.Insert(Record3("G", group.m_sCallsign, group.m_iSeats.ToString(), Flag(group.m_bIsOwn)));

			foreach (TBD_BriefingRole role : group.m_aRoles)
			{
				lines.Insert(Record3("R", role.m_sRole, role.m_iCount.ToString(), Flag(role.m_bIsOwn)));
			}
		}

		foreach (TBD_BriefingZone zone : payload.m_aZones)
		{
			lines.Insert(Record3("Z", zone.m_sTitle, zone.m_sDetail, Flag(zone.m_bIsOwn)));
		}

		if (!payload.m_sWinMode.IsEmpty())
			lines.Insert(Record1("W", payload.m_sWinMode));

		foreach (string trigger : payload.m_aEndConditions)
		{
			lines.Insert(Record1("E", trigger));
		}

		return Join(lines);
	}

	//------------------------------------------------------------------------------------------------
	//! Rebuild a payload on the client. A malformed line is skipped rather than fatal — a briefing
	//! that renders most of itself beats a blank screen (design law: nothing blocking).
	static TBD_BriefingPayload Parse(string wire)
	{
		TBD_BriefingPayload payload = new TBD_BriefingPayload();

		if (wire.IsEmpty())
		{
			payload.m_sUnavailableReason = "No briefing received from the server.";
			return payload;
		}

		array<string> lines = {};
		wire.Split(LINE_SEP, lines, false);

		TBD_BriefingGroup current;

		foreach (string line : lines)
		{
			array<string> f = {};
			line.Split(FIELD_SEP, f, false);
			if (f.IsEmpty())
				continue;

			string kind = f[0];

			if (kind == "M" && f.Count() >= 5)
			{
				payload.m_sMissionName = Unmark(f[1]);
				payload.m_sTerrain = Unmark(f[2]);
				payload.m_sFactionKey = Unmark(f[3]);
				payload.m_sFactionName = Unmark(f[4]);
			}
			else if (kind == "X" && f.Count() >= 2)
			{
				payload.m_sUnavailableReason = Unmark(f[1]);
			}
			else if (kind == "S" && f.Count() >= 4)
			{
				payload.m_bHasSlot = true;
				payload.m_sOwnGroup = Unmark(f[1]);
				payload.m_sOwnRole = Unmark(f[2]);
				payload.m_sOwnKit = Unmark(f[3]);
			}
			else if (kind == "K" && f.Count() >= 3)
			{
				payload.m_aKit.Insert(new TBD_BriefingKitLine(Unmark(f[1]), Unmark(f[2])));
			}
			else if (kind == "G")
			{
				// T-181.26 — a REJECTED group must also clear `current`, or the `R` lines that
				// follow it attach to whichever group came before and that squad silently grows
				// another squad's seats. Misattribution is worse than omission on a briefing, so a
				// group we could not decode takes its roles down with it instead of donating them.
				// Marking every field is what makes `f.Count()` trustworthy in the first place; this
				// guard is what keeps the failure honest when the count is short anyway (a wire
				// clipped at MAX_PAYLOAD_LINES can cut a record in half).
				current = null;

				if (f.Count() >= 4)
				{
					payload.m_aGroups.Insert(new TBD_BriefingGroup(Unmark(f[1])));
					current = payload.m_aGroups[payload.m_aGroups.Count() - 1];
					current.m_iSeats = Unmark(f[2]).ToInt();
					current.m_bIsOwn = IsSet(f[3]);
				}
				else
				{
					TBD_Log.Warn(CH_BRIEFING, string.Format(
						"dropped malformed group record (%1 fields) and every role under it", f.Count()));
				}
			}
			else if (kind == "R" && f.Count() >= 4 && current)
			{
				current.m_aRoles.Insert(new TBD_BriefingRole(Unmark(f[1]), Unmark(f[2]).ToInt(), IsSet(f[3])));
			}
			else if (kind == "Z" && f.Count() >= 4)
			{
				payload.m_aZones.Insert(new TBD_BriefingZone(Unmark(f[1]), Unmark(f[2]), IsSet(f[3])));
			}
			else if (kind == "W" && f.Count() >= 2)
			{
				payload.m_sWinMode = Unmark(f[1]);
			}
			else if (kind == "E" && f.Count() >= 2)
			{
				payload.m_aEndConditions.Insert(Unmark(f[1]));
			}
		}

		return payload;
	}

	//------------------------------------------------------------------------------------------------
	//! T-181.27 — attach the orders arrays a client just received to the payload it just parsed.
	//!
	//! ── Why orders do NOT ride the delimited wire above ─────────────────────────────
	//! Everything `Serialise` carries is a short structured field that `Sanitise` can safely flatten
	//! — a callsign, a role, a rounded coordinate. Orders are the opposite: free prose, authored by
	//! a human, containing newlines by design and any punctuation at all. Pushing that through a
	//! tab-and-newline record format would mean flattening the author's paragraph breaks into
	//! spaces AND resting the result on `string.Split`'s unproven empty-token behaviour — the exact
	//! fragility T-181.26 exists to put a sentinel under. Adding the single most delimiter-hostile
	//! payload in the mod to that format, in the same wave, would be a choice rather than an
	//! oversight.
	//!
	//! So orders travel as three `array<string>` RPC parameters instead. There is no delimiter, so
	//! there is nothing to escape and nothing to mis-split: element i is paragraph i, an empty array
	//! means no orders, and the author's paragraph breaks survive intact. This is T-181.19's
	//! parallel-array precedent (`TBD_RpcDo_Markers`), taken for the same reason it was taken there.
	//!
	//! The arrays are COPIED rather than adopted by reference: they belong to the RPC call frame,
	//! and the payload outlives it on `TBD_BriefingClient`.
	static void AdoptOrders(TBD_BriefingPayload payload, array<string> situation, array<string> mission, array<string> execution)
	{
		if (!payload)
			return;

		CopyInto(payload.m_aSituation, situation);
		CopyInto(payload.m_aMission, mission);
		CopyInto(payload.m_aExecution, execution);
	}

	//------------------------------------------------------------------------------------------------
	//! A null `source` is a real state here — it is what an RPC parameter is when the sender had
	//! nothing to send — and is not the dead nested-`ref` null test the header warns about.
	protected static void CopyInto(array<string> destination, array<string> source)
	{
		destination.Clear();

		if (!source)
			return;

		foreach (string line : source)
		{
			destination.Insert(line);
		}
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	protected static string Join(array<string> lines)
	{
		int shown = lines.Count();
		bool clipped = false;
		if (shown > MAX_PAYLOAD_LINES)
		{
			shown = MAX_PAYLOAD_LINES;
			clipped = true;
		}

		string result;
		for (int i = 0; i < shown; i++)
		{
			if (i > 0)
				result = result + LINE_SEP;

			result = result + lines[i];
		}

		if (clipped)
		{
			TBD_Log.Warn(CH_BRIEFING, string.Format("payload clipped at %1 lines (mission has more) — raise MAX_PAYLOAD_LINES", MAX_PAYLOAD_LINES));
		}

		return result;
	}

	//------------------------------------------------------------------------------------------------
	//! The RECORD KIND is written bare — it is the one token that is never empty by construction and
	//! never authored, so marking it would buy nothing and would only make the wire harder to read
	//! in a log. Every field after it goes through `Field`.
	protected static string Record1(string kind, string a)
	{
		return kind + Field(a);
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record2(string kind, string a, string b)
	{
		return kind + Field(a) + Field(b);
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record3(string kind, string a, string b, string c)
	{
		return kind + Field(a) + Field(b) + Field(c);
	}

	//------------------------------------------------------------------------------------------------
	//! MEASURED elsewhere in this tree (`TBD_AdminData.RecordPlayer`): a NINE-field `+` chain trips
	//! Enfusion's expression-complexity ceiling with `Formula too complex`, and the second
	//! diagnostic on the line is a misleading `Incompatible parameter`. Four fields is well under
	//! that, but if this ever grows, append in steps rather than hunting a type error that is not
	//! there.
	protected static string Record4(string kind, string a, string b, string c, string d)
	{
		return kind + Field(a) + Field(b) + Field(c) + Field(d);
	}

	//------------------------------------------------------------------------------------------------
	//! `<TAB>.<value>` — separator, marker, value. The marker is what guarantees a NON-EMPTY token
	//! for an empty value; see FIELD_MARK and the class header.
	//!
	//! `Sanitise` runs here as well as at build time. That is deliberate belt-and-braces: this is
	//! the single choke point every field passes through, so a future field added straight to a
	//! `Record*` call cannot smuggle a raw tab into the stream and shift the rest of its record.
	//! `Sanitise` only ever substitutes one character for another, so it is idempotent and applying
	//! it twice costs nothing.
	protected static string Field(string value)
	{
		return FIELD_SEP + FIELD_MARK + Sanitise(value);
	}

	//------------------------------------------------------------------------------------------------
	//! Strip the marker back off a parsed field.
	//!
	//! A token of length <= 1 is the marker alone (an authored empty), or — if a truncated wire ever
	//! produced a bare token — nothing at all. Both mean "empty", and rendering an empty string beats
	//! refusing the whole record: design law, an empty state says what it can rather than showing a
	//! void. Total by construction: `Unmark(Field(x)) == Sanitise(x)` for every `x`, `.` and the
	//! empty string included.
	protected static string Unmark(string field)
	{
		int length = field.Length();
		if (length <= 1)
			return string.Empty;

		return field.Substring(1, length - 1);
	}

	//------------------------------------------------------------------------------------------------
	//! A marked boolean. Anything that is not a marked `1` reads as false, so a corrupt token fails
	//! to the safe answer rather than to "this is your own squad".
	protected static bool IsSet(string field)
	{
		return Unmark(field) == "1";
	}

	//------------------------------------------------------------------------------------------------
	protected static string Flag(bool value)
	{
		if (value)
			return "1";

		return "0";
	}

	//------------------------------------------------------------------------------------------------
	//! Strip the field and line separators out of authored text so a mission name containing a
	//! tab cannot shift every field of its record.
	protected static string Sanitise(string value)
	{
		if (value.IsEmpty())
			return value;

		// MEASURED: `string.Replace` mutates the receiver IN PLACE and returns the replacement
		// COUNT, not the new string (same shape the tree already relies on in
		// Scripts/WorkbenchGame/TBD_ExportPaths.c). Assigning its result to a string does not
		// compile. A probe that only `Print`ed the result passed for the wrong reason, because
		// Print happily takes an int — the compile gate is what caught it.
		string clean = value;
		clean.Replace(FIELD_SEP, " ");
		clean.Replace(LINE_SEP, " ");
		clean.Replace("\r", " ");
		return clean;
	}

	//------------------------------------------------------------------------------------------------
	//! Break a string on newlines WITHOUT using `string.Split`.
	//!
	//! ── Why this is hand-rolled ───────────────────────────────────────────────
	//! `string.Split`'s empty-token behaviour is a RUNTIME property: no compile probe on this lane
	//! can settle it and no oracle documents it. Orders are free prose — the one input most likely
	//! to contain a leading newline, a double newline between paragraphs, or a trailing one — so
	//! "does Split emit an empty token, or swallow it?" would decide whether paragraphs land in the
	//! right order, and it is a question this lane cannot answer.
	//!
	//! This loop has no such state. It uses only `IndexOf` / `Substring` / `Length`, all three of
	//! which are already load-bearing in shipped code (`PrettyResourceName` below), and its output
	//! is fully determined: N newlines always yield exactly N+1 parts, empty ones included. The
	//! caller drops the blank parts explicitly, which is a decision rather than an inherited
	//! behaviour.
	//!
	//! Terminating: each iteration removes at least the character at `nl`, so `rest` strictly
	//! shrinks. `MAX_LINE_SCAN` is a belt-and-braces stop, not the mechanism.
	protected static array<string> SplitLines(string raw)
	{
		array<string> parts = {};
		string rest = raw;

		for (int guard = 0; guard < MAX_LINE_SCAN; guard++)
		{
			int nl = rest.IndexOf(LINE_SEP);
			if (nl < 0)
			{
				parts.Insert(rest);
				return parts;
			}

			parts.Insert(rest.Substring(0, nl));
			rest = rest.Substring(nl + 1, rest.Length() - nl - 1);
		}

		return parts;
	}

	//! Hard stop for `SplitLines`. A field with more newlines than this is pathological; the
	//! paragraph cap would have discarded the tail anyway.
	protected static const int MAX_LINE_SCAN = 512;

	//------------------------------------------------------------------------------------------------
	//! Strip leading and trailing SPACES.
	//!
	//! ── `string.Trim()` does exist. This is still hand-rolled, deliberately ───────────────
	//! MEASURED T-181.27, with negative controls, because `string` is a native type and neither
	//! index covers it:
	//!   * a bogus method errors  -> `Undefined function 'string.ZZ…'`, so silence means EXISTS;
	//!   * `string x = s.Trim();`      compiles;
	//!   * `array<string> x = s.Trim();` FAILS  -> the return is neither void nor a container;
	//!   * `string x = s.Replace(a,b);` FAILS, and so do `ToUpper()` / `ToLower()` -> those three
	//!     really do return the documented COUNT;
	//!   * `int n = <a string>;` compiles but `string s = <an int>;` does NOT — Enfusion coerces
	//!     string->int implicitly and never the other way. That asymmetry is what makes
	//!     `string x = s.Foo();` the DISCRIMINATING test and `int n = s.Foo();` a useless one.
	//! Together: `Trim()` returns a real string and is NOT a member of the mutate-in-place family.
	//!
	//! What no probe on this lane can settle is WHICH characters it strips — spaces only, or
	//! tabs/newlines/other Unicode whitespace too. That is a runtime property, and orders are the
	//! one input where it would matter. So this keeps a splitter whose behaviour is fully
	//! determined: it runs AFTER `Sanitise` has already turned tabs and carriage returns into
	//! spaces, so testing for the single space character is provably sufficient.
	protected static string TrimSpaces(string value)
	{
		int length = value.Length();

		int first = 0;
		while (first < length && value.Substring(first, 1) == " ")
		{
			first++;
		}

		int last = length - 1;
		while (last >= first && value.Substring(last, 1) == " ")
		{
			last--;
		}

		if (last < first)
			return string.Empty;

		return value.Substring(first, last - first + 1);
	}

	//------------------------------------------------------------------------------------------------
	//! Cut `value` to at most `limit` BYTES, preferring the last word boundary.
	//!
	//! ── Why a word boundary, and not just `Substring(0, limit)` ───────────────────────
	//! MEASURED T-181.27 on a live boot: **`string.Length()` counts BYTES and `Substring` is
	//! byte-indexed**, not character-indexed. `"…".Length()` is 3, `"·".Length()` is 2, and
	//! `"café latte".Substring(0, 4)` returns `caf` plus the FIRST BYTE of `é` — a broken UTF-8
	//! sequence that renders as a replacement glyph. A blind cut at a byte offset can therefore
	//! sever a multi-byte character, and Everon place names are exactly the accented prose that
	//! would hit it.
	//!
	//! Backing off to the last space fixes that for free: 0x20 cannot appear inside a multi-byte
	//! UTF-8 sequence, so a cut at a space is always on a character boundary. It also reads better
	//! — orders are truncated at a word, not mid-syllable.
	//!
	//! The fallback (no space within the limit — one unbroken 6,000-byte token) keeps the blind
	//! cut, because refusing to truncate would be worse than one malformed trailing glyph.
	protected static string ClipToWord(string value, int limit)
	{
		if (limit <= 0)
			return string.Empty;

		if (value.Length() <= limit)
			return value;

		string head = value.Substring(0, limit);

		int lastSpace = head.LastIndexOf(" ");
		if (lastSpace > 0)
			return head.Substring(0, lastSpace);

		return head;
	}

	//------------------------------------------------------------------------------------------------
	//! Warn once per faction+field. Truncating a player's orders is never silent, and never spammy.
	protected static void WarnOnce(string factionKey, string field, string message)
	{
		if (!s_mWarned)
			s_mWarned = new map<string, bool>();

		if (s_mWarned.Count() > MAX_WARN_STATES)
			s_mWarned.Clear();

		string key = factionKey + "|" + field;

		bool seen;
		if (s_mWarned.Find(key, seen))
			return;

		s_mWarned.Set(key, true);
		TBD_Log.Warn(CH_BRIEFING, message);
	}

	//------------------------------------------------------------------------------------------------
	//! `{ABC123}Prefabs/Weapons/Rifles/M4A1.et` -> `M4A1`. A briefing shows equipment, not paths.
	static string PrettyResourceName(string resource)
	{
		string s = Sanitise(resource);

		int close = s.IndexOf("}");
		if (close >= 0)
			s = s.Substring(close + 1, s.Length() - close - 1);

		int slash = s.LastIndexOf("/");
		if (slash >= 0)
			s = s.Substring(slash + 1, s.Length() - slash - 1);

		int dot = s.LastIndexOf(".");
		if (dot > 0)
			s = s.Substring(0, dot);

		if (s.IsEmpty())
			return Sanitise(resource);

		return s;
	}

	//------------------------------------------------------------------------------------------------
	//! `objective_capture` -> `Objective capture`. Snake-case keys are authored data; a planning
	//! screen should read like prose.
	static string Humanise(string key)
	{
		if (key.IsEmpty())
			return key;

		string s = Sanitise(key);
		s.Replace("_", " "); // in-place; see the note in Sanitise()

		string head = s.Substring(0, 1);
		head.ToUpper(); // in-place, like Replace — the return value is not the new string
		string tail = s.Substring(1, s.Length() - 1);
		return head + tail;
	}
}
