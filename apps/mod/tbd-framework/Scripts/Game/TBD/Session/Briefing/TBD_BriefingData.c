//! T-181.9.2 — the briefing payload, and the place side discipline is actually enforced.
//! UI reorg 2026-09-12: models only. Server builder + wire Serialise/Parse live in `TBD_BriefingService.c`.
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
