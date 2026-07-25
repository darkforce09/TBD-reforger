//! T-181.19 — the SERVER half of map markers: who is allowed to see which markers, and in what
//! shape they cross the wire.
//!
//! ── Why the server has to do this at all ────────────────────────────────────────────────────
//! Clients hold NO mission document. `TBD_FrameworkManager.OnPostInit` returns early for
//! `RplMode.Client` before `BeginLoad()`, so a client physically cannot read `briefing.markers`
//! for itself. Markers are server-fed or they do not exist.
//!
//! ── Side discipline, enforced at the WIRE ───────────────────────────────────────────────────
//! `briefings` is `map<string, TBD_MissionBriefingStruct>` keyed by faction, exactly like `orbat`,
//! so markers are SIDE-SCOPED INTELLIGENCE. BLUFOR must never receive OPFOR's markers.
//!
//! Three properties make that structural rather than a promise:
//!   1. `BuildForPlayer` takes a **playerId and nothing else**. There is no faction parameter, so
//!      a client has nowhere to put a lie.
//!   2. The side is read from `TBD_SpawnManager.GetAssignedSlot(playerId)` — server-owned state a
//!      client cannot influence.
//!   3. Only the resolved side's rows are ever placed in the arrays that get sent. The other
//!      side's markers never leave the server process, so there is nothing on the client to filter
//!      wrongly, nothing to sniff off the wire, and nothing a modified client could reveal.
//!
//! **If a client asked for another faction's markers it could not phrase the question.** The only
//! request RPC takes no arguments; the answer is whatever `GetAssignedSlot` says the caller is. A
//! player with no slot gets `served = false` and zero rows — fail closed, not fail open.
//!
//! ── Why parallel arrays and not a delimited string ──────────────────────────────────────────
//! `string.Split`'s empty-token behaviour is a RUNTIME property: unprovable by compile probe and
//! absent from every oracle. `label` and `icon` are `{"type":"string"}` with NO `minLength`, so an
//! EMPTY label is perfectly schema-legal — the exact input that would make a delimited format
//! ambiguous. (The brief's "all four keys required, therefore complete" is true about PRESENCE and
//! not about CONTENT; this is the one place that distinction bites.) A label could also legally
//! contain any delimiter we picked.
//!
//! So there is no delimiter at all. Four parallel `array<...>` RPC parameters carry the four
//! fields positionally: an empty label is an empty array element and means exactly one thing.
//! `array<int>` / `array<string>` as RPC parameters is proven in BOTH oracles
//! (`SCR_RespawnBriefingComponent.RpcDo_RewriteEntry`, and CRF's
//! `CRF_PlayerRplToOwnerManager.RpcDo_ShareMarker(array<int>)`), and probed here with a failing
//! negative control on a bogus element type.
//!
//! This is not a hypothetical worry. `packages/tbd-schema/golden-missions/empty-warning-fields.json`
//! — a COMMITTED, schema-valid fixture — carries a marker whose `icon` AND `label` are both the
//! empty string. A delimited wire format would have shipped with that fixture already breaking it.
//!
//! ── The side-discipline case is also already in the fixtures ────────────────────────────────
//! `golden-missions/bridgehead-at-levie.json` gives blufor `objective / "OBJ BRIDGE"` and opfor
//! `defend / "HOLD BRIDGE"` at the SAME coordinates. Sending both to everyone and filtering in a
//! widget would tell each side exactly what the other has been ordered to do at the one place that
//! decides the round.
//! @contract mission.schema.json#/$defs/marker
class TBD_MarkerService
{
	//! Greppable channel for everything this slice logs: `grep '\[TBD\]\[Markers\]' console.log`.
	static const string CH_MARKERS = "Markers";

	//! Hard cap on markers sent to one client. A reliable RPC is not a bulk transport, and the
	//! schema puts no `maxItems` on `briefing.markers`, so a pathological document could otherwise
	//! try to push thousands of rows down a reliable channel. Truncation is LOGGED, never silent.
	static const int MAX_MARKERS = 64;

	//! Longest label we will ship. The schema has no `maxLength`; this bounds the packet without
	//! ever dropping the marker itself.
	static const int MAX_LABEL_CHARS = 64;

	//! playerId -> the last outcome logged for them, so a polling client cannot fill the console.
	protected static ref map<int, string> s_mLastLogged;

	//! Hard ceiling on that map. Player ids are recycled on a dedicated server and there is no
	//! disconnect hook in this slice's files, so rather than leak across a long session the whole
	//! table is dropped when it grows past a session's worth of players. The only cost of dropping
	//! it is one repeated log line per player.
	protected static const int MAX_LOG_STATES = 256;

	//------------------------------------------------------------------------------------------------
	//! @authority server — build ONE player's marker set.
	//!
	//! Never returns null: an unslotted player, an unloaded mission and a mission with no briefing
	//! for their side are three different legal states, and each yields an empty served=false
	//! answer carrying the reason. Absent `briefings`, absent `markers` and an empty `markers`
	//! array are also three different legal states — none of them is an error, and none of them
	//! logs like one.
	static TBD_MarkerWire BuildForPlayer(int playerId)
	{
		TBD_MarkerWire wire = Build(playerId);

		// Logged HERE, not at the RPC handler, because a listen host short-circuits the RPC
		// entirely and would otherwise be the one topology that produced no marker log at all.
		// Host/dedicated behaviour divergence is a recorded failure mode in this program, and the
		// cheapest place to not have it is the single function both paths already share.
		LogOutcome(playerId, wire);

		return wire;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the decision itself, with no logging in it.
	protected static TBD_MarkerWire Build(int playerId)
	{
		TBD_MarkerWire wire = new TBD_MarkerWire();

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			wire.m_sRefusal = "no spawn manager";
			return wire;
		}

		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (!slot)
		{
			// Fail closed. No seat means no side, and no side means no side's intelligence.
			wire.m_sRefusal = "no slot assigned";
			return wire;
		}

		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
		{
			wire.m_sRefusal = "no mission loaded";
			return wire;
		}

		wire.m_sFactionKey = slot.faction;
		wire.m_sMissionId = doc.meta.id;

		// From here on the answer is authoritative even when it is empty: the player HAS a side and
		// the server HAS a mission, so "this side authored no markers" is a real, served answer and
		// the client should stop asking.
		wire.m_bServed = true;

		TBD_MissionBriefingStruct briefing = TBD_MissionLoader.GetBriefingForFaction(slot.faction);
		if (!briefing || !briefing.markers)
			return wire;

		int total = briefing.markers.Count();
		int sent = 0;

		foreach (TBD_MissionMarkerStruct marker : briefing.markers)
		{
			if (!marker)
				continue;

			if (sent >= MAX_MARKERS)
				break;

			// Marker world position is (X, Z): `SCR_MapMarkerBase.TestVisibleFrame` compares its
			// second coordinate against the map frame's `[2]` component, i.e. the world Z axis.
			// The schema's `{x, z}` therefore maps across one-to-one, and there is deliberately no
			// `y` in either model.
			wire.m_aX.Insert(RoundToInt(marker.x));
			wire.m_aZ.Insert(RoundToInt(marker.z));
			wire.m_aIcon.Insert(marker.icon);
			wire.m_aLabel.Insert(CapLabel(marker.label));

			sent++;
		}

		if (total > sent)
		{
			TBD_Log.Warn(CH_MARKERS, string.Format(
				"mission '%1' authored %2 markers for faction '%3'; sent the first %4 (cap %5).",
				wire.m_sMissionId, total, wire.m_sFactionKey, sent, MAX_MARKERS));
		}

		return wire;
	}

	//------------------------------------------------------------------------------------------------
	//! One line per player, only when the answer CHANGES.
	//!
	//! Both outcomes are NORMAL level. A refusal is not an error — an unslotted player asking
	//! during LOBBY is the ordinary case and is how the client knows to keep asking — and
	//! `world-boot.sh` triages any TBD-owned `SCRIPT (E)` line as a gate failure.
	protected static void LogOutcome(int playerId, TBD_MarkerWire wire)
	{
		if (!wire.m_bServed)
		{
			if (ShouldLog(playerId, "refused:" + wire.m_sRefusal))
			{
				TBD_Log.Kv(CH_MARKERS, "refused",
					string.Format("player=%1 reason='%2'", playerId, wire.m_sRefusal));
			}

			return;
		}

		string outcome = string.Format("served:%1:%2:%3",
			wire.m_sFactionKey, wire.m_sMissionId, wire.Count());

		if (!ShouldLog(playerId, outcome))
			return;

		TBD_Log.Kv(CH_MARKERS, "served", string.Format(
			"player=%1 faction=%2 mission=%3 markers=%4",
			playerId, wire.m_sFactionKey, wire.m_sMissionId, wire.Count()));
	}

	//------------------------------------------------------------------------------------------------
	//! True the FIRST time this player's outcome differs from the last one logged for them.
	//!
	//! Why this exists: an unslotted client re-asks every 5 s, and a full server sitting in LOBBY
	//! would otherwise emit a dozen identical "refused" lines a second. That exact defect —
	//! "unbounded console warn per refused RPC" — is already on the books against the admin service
	//! (T-181.30 item 4); re-committing it in a new slice would be a choice, not an oversight.
	protected static bool ShouldLog(int playerId, string outcome)
	{
		if (!s_mLastLogged)
			s_mLastLogged = new map<int, string>();

		if (s_mLastLogged.Count() > MAX_LOG_STATES)
			s_mLastLogged.Clear();

		string previous;
		if (s_mLastLogged.Find(playerId, previous) && previous == outcome)
			return false;

		s_mLastLogged.Set(playerId, outcome);
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! Nearest-integer, both signs. Enfusion has no ternary operator (`cond ? a : b` fails with
	//! `Broken expression (missing ';'?)` and never mentions `?`), so this is written out.
	protected static int RoundToInt(float value)
	{
		if (value >= 0)
			return value + 0.5;

		return value - 0.5;
	}

	//------------------------------------------------------------------------------------------------
	//! Bound the label without ever dropping the marker. An over-long caption is truncated; it is
	//! never a reason to withhold a piece of the briefing.
	protected static string CapLabel(string label)
	{
		if (label.Length() <= MAX_LABEL_CHARS)
			return label;

		return label.Substring(0, MAX_LABEL_CHARS);
	}
}

//! One player's marker set, server-side, in the exact shape the RPC takes.
//!
//! Deliberately four parallel arrays rather than an array of row objects: these fields ARE the RPC
//! parameter list, and keeping the class isomorphic to the wire means the packing step cannot
//! reorder or drop a column without the compiler noticing.
class TBD_MarkerWire
{
	//! False = the server had no authoritative answer for this player yet (no slot, no mission).
	//! The client keeps asking while this is false, and stops the moment it is true — including
	//! when it is true with zero rows, which is a real answer.
	bool m_bServed;

	//! The side these markers belong to. Diagnostic only; the client never sends it back and never
	//! makes a trust decision with it.
	string m_sFactionKey;

	//! Lets the client tell "same orders again" from "the admin switched missions".
	string m_sMissionId;

	ref array<int> m_aX = {};
	ref array<int> m_aZ = {};
	ref array<string> m_aIcon = {};
	ref array<string> m_aLabel = {};

	//! Why the server declined, when it did. Logged, not shown to the player.
	string m_sRefusal;

	//------------------------------------------------------------------------------------------------
	int Count()
	{
		return m_aX.Count();
	}
}
