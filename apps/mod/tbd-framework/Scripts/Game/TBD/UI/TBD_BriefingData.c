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
//! ── Why the ORBAT is derived from `slots[]`, not from `orbat` ───────────────────────────────
//! `TBD_MissionDocumentStruct.orbat` is parsed, but `TBD_MissionOrbatGroupStruct` carries only
//! `roles`, and `TBD_MissionOrbatRoleStruct` carries only `count` — the JSON's `callsign`,
//! `slot` and `kit` keys are not modelled. Rendering from it would produce a nameless list.
//!
//! `slots[]` is the flattened form and carries `faction`, `groupCallsign`, `role`, `kit` and the
//! loadout in full. Deriving from it is not a workaround: it is the stronger source, because it
//! is the same array `TBD_SpawnManager` spawns from. The briefing therefore shows what will
//! actually exist in the world, not what a parallel block claims.

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
	}

	//------------------------------------------------------------------------------------------------
	bool IsAvailable()
	{
		return m_sUnavailableReason.IsEmpty();
	}
}

//! Builds a briefing on the server, moves it over one RPC, and rebuilds it on the client.
//!
//! The wire format is line-based with tab-separated fields, matching the precedent already in
//! the tree (`TBD_MissionBrowserService`): it keeps the RPC signature to a single string, needs
//! no schema registration, and is greppable in a log when something goes wrong.
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

		payload.m_sFactionKey = own.faction;
		payload.m_sFactionName = ResolveFactionName(doc, own.faction);
		payload.m_bHasSlot = true;
		payload.m_sOwnGroup = Sanitise(own.groupCallsign);
		payload.m_sOwnRole = Sanitise(own.role);
		payload.m_sOwnKit = Sanitise(own.kit);

		BuildKit(payload, own);
		BuildOrbat(payload, doc, own);
		BuildZones(payload, doc, own.faction);
		BuildEndConditions(payload, doc);

		return payload;
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

			// Polygon zones parse to a null circle (only `circle` is modelled in
			// TBD_MissionShapeStruct), so a boundary contributes its name but no coordinates.
			string detail;
			if (zone.shape && zone.shape.circle)
			{
				detail = string.Format("%1, %2 · r%3",
					Math.Round(zone.shape.circle.x),
					Math.Round(zone.shape.circle.z),
					Math.Round(zone.shape.circle.r));
			}
			else
			{
				detail = "area";
			}

			payload.m_aZones.Insert(new TBD_BriefingZone(PrettyZoneTitle(zone), detail, isOwn));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! `objective_capture` + id `z3` -> "Objective capture — z3".
	//!
	//! The mission JSON carries a human `label` on objective zones ("Levie Bridge"), but
	//! `TBD_MissionZoneStruct` does not model that key, so it is unavailable here. See the hook
	//! noted in the slice report; until it lands, the authored type and id are the honest answer.
	protected static string PrettyZoneTitle(TBD_MissionZoneStruct zone)
	{
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
				payload.m_sMissionName = f[1];
				payload.m_sTerrain = f[2];
				payload.m_sFactionKey = f[3];
				payload.m_sFactionName = f[4];
			}
			else if (kind == "X" && f.Count() >= 2)
			{
				payload.m_sUnavailableReason = f[1];
			}
			else if (kind == "S" && f.Count() >= 4)
			{
				payload.m_bHasSlot = true;
				payload.m_sOwnGroup = f[1];
				payload.m_sOwnRole = f[2];
				payload.m_sOwnKit = f[3];
			}
			else if (kind == "K" && f.Count() >= 3)
			{
				payload.m_aKit.Insert(new TBD_BriefingKitLine(f[1], f[2]));
			}
			else if (kind == "G" && f.Count() >= 4)
			{
				payload.m_aGroups.Insert(new TBD_BriefingGroup(f[1]));
				current = payload.m_aGroups[payload.m_aGroups.Count() - 1];
				current.m_iSeats = f[2].ToInt();
				current.m_bIsOwn = f[3] == "1";
			}
			else if (kind == "R" && f.Count() >= 4 && current)
			{
				current.m_aRoles.Insert(new TBD_BriefingRole(f[1], f[2].ToInt(), f[3] == "1"));
			}
			else if (kind == "Z" && f.Count() >= 4)
			{
				payload.m_aZones.Insert(new TBD_BriefingZone(f[1], f[2], f[3] == "1"));
			}
			else if (kind == "W" && f.Count() >= 2)
			{
				payload.m_sWinMode = f[1];
			}
			else if (kind == "E" && f.Count() >= 2)
			{
				payload.m_aEndConditions.Insert(f[1]);
			}
		}

		return payload;
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
	protected static string Record1(string kind, string a)
	{
		return kind + FIELD_SEP + a;
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record2(string kind, string a, string b)
	{
		return kind + FIELD_SEP + a + FIELD_SEP + b;
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record3(string kind, string a, string b, string c)
	{
		return kind + FIELD_SEP + a + FIELD_SEP + b + FIELD_SEP + c;
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record4(string kind, string a, string b, string c, string d)
	{
		return kind + FIELD_SEP + a + FIELD_SEP + b + FIELD_SEP + c + FIELD_SEP + d;
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
