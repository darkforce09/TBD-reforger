//! T-181.9.1 — the lobby roster: what the server knows about every seat, shaped for a screen.
//!
//! ── The fact this design is built on ────────────────────────────────────────────────────────
//! **A client has no mission document and no slot assignment.** `TBD_FrameworkManager.OnPostInit`
//! returns early for `RplMode.Client` *before* `TBD_MissionLoader.BeginLoad()`, so
//! `TBD_MissionLoader.GetMission()` is null on every client, forever; and `TBD_SpawnManager`'s
//! `m_mPlayerSlot` is a plain `map`, not an `RplProp`, so `GetAssignedSlot()` is server-only.
//!
//! The slot picker is a client-side menu. It therefore cannot read the roster at all — it can only
//! render what the server chose to send it. Same wall the briefing hit at T-181.9.2, and the same
//! answer: build on the server, ship one string over an owner-scoped RPC, rebuild on the client.
//!
//! ── The roster is NOT re-derived here ───────────────────────────────────────────────────────
//! `TBD_SpawnManager.BuildSlotRoster()` (T-181.9, shipped) is the authority's own answer to
//! "who holds what", emitted as TSV precisely so a picker could bind to it:
//!
//!     <slotKey> \t <faction> \t <group> \t <role> \t <state> \t <holderPlayerId>
//!     state ∈ OPEN | HELD | DEAD
//!
//! This service **parses that and nothing else**. It does not consult `m_mPlayerSlot`, does not
//! re-implement `IsSlotHeldByAnother`, and cannot disagree with the authority about who holds a
//! seat — because it never forms an opinion. What it adds is the two things a screen needs and a
//! playerId cannot supply: the holder's **display name**, and the faction's **display name**.
//!
//! `DEAD` with holder `-1` is a seat whose occupant spent their life and then quit. It reads as
//! DEAD, not OPEN, and it is not selectable — exactly as the authority reports it.
//!
//! ── Side discipline: deliberately NOT applied here ──────────────────────────────────────────
//! The briefing filters the other side out at the wire. The lobby must not: you cannot pick a side
//! you cannot see, and Arma 3 — the parity target — shows the full ORBAT of both sides with who is
//! in each seat. Side discipline begins the moment you have a seat, which is the briefing screen.
//! This is a deliberate divergence between two neighbouring screens, recorded so a later reader
//! does not "fix" one to match the other.

//! One seat. `m_bIsOwn` is resolved on the SERVER against the reader's own assignment, so the
//! client never has to work out which seat is theirs (it could not — it has no assignment).
class TBD_LobbySlot
{
	string m_sKey;    //!< durable slot key — the exact string ClaimSlot() takes
	string m_sRole;
	string m_sState;  //!< OPEN | HELD | DEAD, verbatim from the authority
	string m_sHolder; //!< display name; empty for OPEN and for a departed DEAD holder
	bool m_bIsOwn;

	//------------------------------------------------------------------------------------------------
	void TBD_LobbySlot(string key, string role, string state, string holder, bool isOwn)
	{
		m_sKey = key;
		m_sRole = role;
		m_sState = state;
		m_sHolder = holder;
		m_bIsOwn = isOwn;
	}

	//------------------------------------------------------------------------------------------------
	//! The one question the picker asks of a row. A dead seat is never selectable — that is the
	//! whole point of the DEAD state existing separately from HELD.
	bool IsOpen()
	{
		return m_sState == TBD_LobbyService.STATE_OPEN;
	}

	//------------------------------------------------------------------------------------------------
	bool IsDead()
	{
		return m_sState == TBD_LobbyService.STATE_DEAD;
	}
}

//! One squad. Seats and the open count are carried explicitly so a collapsed group can still say
//! how much room it has — that is what makes side -> group -> slot navigable without opening
//! everything to find out.
class TBD_LobbyGroup
{
	string m_sCallsign;
	int m_iOpen;
	bool m_bHasOwn; //!< the reader's seat is in this squad — the disclosure default
	ref array<ref TBD_LobbySlot> m_aSlots;

	//------------------------------------------------------------------------------------------------
	void TBD_LobbyGroup(string callsign)
	{
		m_sCallsign = callsign;
		m_aSlots = {};
	}

	//------------------------------------------------------------------------------------------------
	int Seats()
	{
		return m_aSlots.Count();
	}
}

//! One side. Same shape as a group, one level up.
class TBD_LobbySide
{
	string m_sKey;
	string m_sName;
	int m_iSeats;
	int m_iOpen;
	bool m_bHasOwn;
	ref array<ref TBD_LobbyGroup> m_aGroups;

	//------------------------------------------------------------------------------------------------
	void TBD_LobbySide(string key, string name)
	{
		m_sKey = key;
		m_sName = name;
		m_aGroups = {};
	}
}

//! Everything the picker draws. Built on the server, shipped as one string, rebuilt on the client.
class TBD_LobbyRoster
{
	string m_sMissionName;
	string m_sTerrain;
	string m_sStage;

	//! The reader's own seat. Empty key = no seat yet, which is what disables DEPLOY.
	string m_sOwnKey;
	string m_sOwnLabel; //!< "ALPHA · SL", ready to print

	//! ONE LIFE: this reader has already spent theirs. Claim and deploy will both be refused by
	//! the authority, so the screen says so up front instead of letting them find out by clicking.
	bool m_bLifeSpent;

	ref array<ref TBD_LobbySide> m_aSides;

	//! Set when the server could not answer. The screen shows this instead of an empty frame —
	//! design law: an empty state says why, it never shows a void.
	string m_sUnavailableReason;

	// ── The last thing the server did on this player's behalf, if anything. ──────────────────
	// Every server reply carries a whole fresh roster, so a claim/release/deploy answer and a
	// plain refresh are the SAME message shape. That is what makes optimistic reconciliation a
	// wholesale replace rather than a merge: there is never a partial update to apply.
	string m_sAction;  //!< CLAIM | RELEASE | DEPLOY, empty for a plain refresh
	bool m_bActionOk;
	string m_sActionReason;
	string m_sActionKey; //!< the slot the action was about (for the rejection highlight)

	//------------------------------------------------------------------------------------------------
	void TBD_LobbyRoster()
	{
		m_aSides = {};
	}

	//------------------------------------------------------------------------------------------------
	bool IsAvailable()
	{
		return m_sUnavailableReason.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	bool HasOwnSlot()
	{
		return !m_sOwnKey.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	int TotalSeats()
	{
		int n = 0;
		foreach (TBD_LobbySide side : m_aSides)
		{
			n += side.m_iSeats;
		}

		return n;
	}

	//------------------------------------------------------------------------------------------------
	int TotalOpen()
	{
		int n = 0;
		foreach (TBD_LobbySide side : m_aSides)
		{
			n += side.m_iOpen;
		}

		return n;
	}

	//------------------------------------------------------------------------------------------------
	//! Slot by key, or null. Linear because a lobby list is walked once per interaction, not per
	//! frame — a map would cost more to keep in step than it saves.
	TBD_LobbySlot FindSlot(string key)
	{
		if (key.IsEmpty())
			return null;

		foreach (TBD_LobbySide side : m_aSides)
		{
			foreach (TBD_LobbyGroup group : side.m_aGroups)
			{
				foreach (TBD_LobbySlot slot : group.m_aSlots)
				{
					if (slot.m_sKey == key)
						return slot;
				}
			}
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Recount every side and group from the slots. Called after a local optimistic edit so the
	//! headline counts a collapsed row shows cannot drift from the rows underneath it.
	void Recount()
	{
		m_sOwnKey = string.Empty;
		m_sOwnLabel = string.Empty;

		foreach (TBD_LobbySide side : m_aSides)
		{
			side.m_iSeats = 0;
			side.m_iOpen = 0;
			side.m_bHasOwn = false;

			foreach (TBD_LobbyGroup group : side.m_aGroups)
			{
				group.m_iOpen = 0;
				group.m_bHasOwn = false;

				foreach (TBD_LobbySlot slot : group.m_aSlots)
				{
					side.m_iSeats++;

					if (slot.IsOpen())
					{
						group.m_iOpen++;
						side.m_iOpen++;
					}

					if (!slot.m_bIsOwn)
						continue;

					group.m_bHasOwn = true;
					side.m_bHasOwn = true;
					m_sOwnKey = slot.m_sKey;
					m_sOwnLabel = string.Format("%1 · %2", group.m_sCallsign, slot.m_sRole);
				}
			}
		}
	}
}

//! Builds the roster on the server, moves it over one RPC, and rebuilds it on the client. Also the
//! server-side home of the three things a picker can ask the authority to DO.
//!
//! The wire is line-based with tab-separated fields, matching the precedent already in the tree
//! (`TBD_MissionBrowserService`, `TBD_BriefingService`): it keeps the RPC signature to a single
//! string, needs no schema registration, and is greppable in a log when something goes wrong.
class TBD_LobbyService
{
	//! Log channel. A local constant rather than an edit to `TBD_Log`'s enum block — two slices
	//! adding a channel to that one block in the same wave is a merge conflict for no benefit.
	//! Fold it into `TBD_Log.CH_*` when the UI slices are next consolidated.
	static const string CH_LOBBY = "Lobby";

	//! State vocabulary, verbatim from `TBD_SpawnManager.BuildSlotRoster()`. Named here so the
	//! screen never compares against a bare literal and a rename on the authority side breaks in
	//! one place instead of five.
	static const string STATE_OPEN = "OPEN";
	static const string STATE_HELD = "HELD";
	static const string STATE_DEAD = "DEAD";

	static const string ACTION_CLAIM = "CLAIM";
	static const string ACTION_RELEASE = "RELEASE";
	static const string ACTION_DEPLOY = "DEPLOY";

	//! Defensive cap on one payload, mirroring `TBD_BriefingService.MAX_PAYLOAD_LINES`. Sized for
	//! the 128-slot mission `TBD_ListBox` was built for: 128 slot lines + ~20 group lines + a
	//! handful of side/header lines, with headroom. A pathological mission must not become an
	//! unbounded reliable-channel string.
	protected static const int MAX_PAYLOAD_LINES = 600;

	protected static const string FIELD_SEP = "\t";
	protected static const string LINE_SEP = "\n";

	//! Sentinel for an EMPTY field on the wire.
	//!
	//! `string.Split` is a native with no source in any oracle, so whether it preserves empty
	//! tokens is not something this lane can prove — and a dropped empty token silently shifts
	//! every field after it, which would decode one player's seat as another's. Packing empties to
	//! a sentinel makes the format correct under either behaviour, so the question never has to be
	//! answered. `~` is not a plausible whole-field value (names, callsigns and roles all come from
	//! authored JSON or a display name).
	protected static const string EMPTY = "~";

	// ── SERVER ──────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Build the roster as it stands right now.
	//! @authority server — reads `TBD_SpawnManager` and `TBD_MissionLoader`, neither of which
	//! exists on a client.
	static TBD_LobbyRoster BuildForPlayer(int playerId)
	{
		TBD_LobbyRoster roster = new TBD_LobbyRoster();

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (fm)
			roster.m_sStage = typename.EnumToString(TBD_EGameStage, fm.GetStage());

		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !TBD_MissionLoader.IsValid())
		{
			roster.m_sUnavailableReason = "Mission is still loading.";
			return roster;
		}

		if (doc.meta)
		{
			roster.m_sMissionName = Sanitise(doc.meta.name);
			roster.m_sTerrain = Sanitise(doc.meta.terrain);
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			roster.m_sUnavailableReason = "The server is still starting up.";
			return roster;
		}

		roster.m_bLifeSpent = spawn.IsPlayerDead(playerId);

		// The authority's own answer, not a re-derivation. See the class header.
		array<string> rows = spawn.BuildSlotRoster();
		if (!rows || rows.IsEmpty())
		{
			roster.m_sUnavailableReason = "This mission has no slots.";
			return roster;
		}

		TBD_MissionSlotStruct own = spawn.GetAssignedSlot(playerId);
		string ownKey;
		if (own)
			ownKey = own.Key();

		PlayerManager players = GetGame().GetPlayerManager();

		foreach (string row : rows)
		{
			array<string> f = {};
			row.Split(FIELD_SEP, f, false);

			// A malformed row is skipped, never guessed at: mis-decoding one would attribute a
			// seat to the wrong player, which is the one error class this screen must not make.
			if (f.Count() < 6)
			{
				TBD_Log.Warn(CH_LOBBY, string.Format("skipped malformed roster row (%1 fields): '%2'", f.Count(), row));
				continue;
			}

			string slotKey = f[0];
			string factionKey = f[1];
			string callsign = f[2];
			string role = f[3];
			string state = f[4];
			int holderId = f[5].ToInt();

			string holder;
			if (holderId > 0 && players)
				holder = Sanitise(players.GetPlayerName(holderId));

			bool isOwn = !ownKey.IsEmpty() && slotKey == ownKey;

			TBD_LobbySide side = AcquireSide(roster, doc, factionKey);
			TBD_LobbyGroup group = AcquireGroup(side, Sanitise(callsign));

			group.m_aSlots.Insert(new TBD_LobbySlot(slotKey, Sanitise(role), state, holder, isOwn));
		}

		// One place computes the counts, and it is the same one the client re-runs after an
		// optimistic edit — so a collapsed side's headline can never disagree with its rows.
		roster.Recount();
		return roster;
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — take a seat. The rule is `TBD_SpawnManager.ClaimSlot`'s and stays
	//! there: first-come, refuses a dead player, refuses a seat somebody else holds. This function
	//! adds no policy, only a sentence a human can read.
	//!
	//! `accepted` is what the client latches on. Without it an optimistic claim would stay on
	//! screen after the authority refused it, and two players would each believe they hold the
	//! same seat until something else forced a refresh.
	static string ApplyClaim(int playerId, string slotKey, out bool accepted)
	{
		accepted = false;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return "The server is not ready yet.";

		if (spawn.IsPlayerDead(playerId))
			return "Your life is spent. Only an admin can put you back in.";

		if (spawn.ClaimSlot(playerId, slotKey))
		{
			accepted = true;

			TBD_Log.Event(CH_LOBBY, string.Format("claim ok player=%1 slot=%2 name='%3'",
				playerId, slotKey, GetGame().GetPlayerManager().GetPlayerName(playerId)));

			return "Seat taken.";
		}

		TBD_Log.Event(CH_LOBBY, string.Format("claim refused player=%1 slot=%2", playerId, slotKey));

		// Say WHY, from the authority's own view of the seat, so a refusal is legible rather than
		// a shrug. The roster shipped alongside this reason already shows the new holder's name.
		return DescribeRefusal(spawn, slotKey);
	}

	//------------------------------------------------------------------------------------------------
	//! Turn "ClaimSlot said no" into the specific reason, read back off the authority.
	//! @authority server
	protected static string DescribeRefusal(TBD_SpawnManager spawn, string slotKey)
	{
		TBD_MissionSlotStruct slot = TBD_MissionLoader.GetSlotById(slotKey);
		if (!slot)
			return "That seat is not part of this mission.";

		array<string> rows = spawn.BuildSlotRoster();
		foreach (string row : rows)
		{
			array<string> f = {};
			row.Split(FIELD_SEP, f, false);
			if (f.Count() < 6 || f[0] != slotKey)
				continue;

			string state = f[4];
			int holderId = f[5].ToInt();

			if (state == STATE_DEAD)
				return "That seat belongs to someone who is already down.";

			if (state == STATE_HELD && holderId > 0)
				return string.Format("%1 got there first.", GetGame().GetPlayerManager().GetPlayerName(holderId));

			if (state == STATE_HELD)
				return "Someone got there first.";
		}

		return "The server refused that seat.";
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — give a seat back. `ReleaseSlot` refuses once the life is spent or once
	//! the player has deployed; both are correct and both need saying out loud.
	static string ApplyRelease(int playerId, out bool accepted)
	{
		accepted = false;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return "The server is not ready yet.";

		if (spawn.ReleaseSlot(playerId))
		{
			accepted = true;
			TBD_Log.Event(CH_LOBBY, string.Format("release ok player=%1", playerId));
			return "Seat given up.";
		}

		if (spawn.IsPlayerDead(playerId))
			return "Your life is spent — the seat stays yours.";

		return "You are already in the world; the seat is yours.";
	}

	//------------------------------------------------------------------------------------------------
	//! @authority server — the one consequential click. `DeployPlayerEx` is the ONE-LIFE
	//! enforcement boundary (see its header in `TBD_SpawnManager`); this maps its verdict onto a
	//! sentence, and adds nothing.
	static string ApplyDeploy(int playerId, out bool accepted, out string resultName)
	{
		accepted = false;
		resultName = string.Empty;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return "The server is not ready yet.";

		if (!spawn.GetAssignedSlot(playerId))
			return "Take a seat first.";

		TBD_EDeployResult result = spawn.DeployPlayerEx(playerId);
		resultName = typename.EnumToString(TBD_EDeployResult, result);

		TBD_Log.Event(CH_LOBBY, string.Format("deploy player=%1 result=%2", playerId, resultName));

		if (result == TBD_EDeployResult.DEPLOYED || result == TBD_EDeployResult.ALREADY)
		{
			accepted = true;
			return "Deploying.";
		}

		if (result == TBD_EDeployResult.DENIED)
			return "Your life is spent. Only an admin can put you back in.";

		if (result == TBD_EDeployResult.RETRY)
			return "The server is not ready to deploy you yet — try again in a moment.";

		if (result == TBD_EDeployResult.NOT_MINE)
			return "No framework mission is loaded.";

		return "Deploy failed — the slot body could not be prepared. Tell an admin.";
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_LobbySide AcquireSide(TBD_LobbyRoster roster, TBD_MissionDocumentStruct doc, string factionKey)
	{
		foreach (TBD_LobbySide existing : roster.m_aSides)
		{
			if (existing.m_sKey == factionKey)
				return existing;
		}

		roster.m_aSides.Insert(new TBD_LobbySide(factionKey, ResolveFactionName(doc, factionKey)));
		return roster.m_aSides[roster.m_aSides.Count() - 1];
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_LobbyGroup AcquireGroup(TBD_LobbySide side, string callsign)
	{
		foreach (TBD_LobbyGroup existing : side.m_aGroups)
		{
			if (existing.m_sCallsign == callsign)
				return existing;
		}

		side.m_aGroups.Insert(new TBD_LobbyGroup(callsign));
		return side.m_aGroups[side.m_aGroups.Count() - 1];
	}

	//------------------------------------------------------------------------------------------------
	protected static string ResolveFactionName(TBD_MissionDocumentStruct doc, string factionKey)
	{
		if (doc && doc.factions)
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
	//! Flatten a roster to one string. Record types:
	//!   `M` mission   name / terrain / stage
	//!   `X` unavailable reason (terminal — nothing else follows)
	//!   `L` life      "1" when this reader has spent theirs
	//!   `V` verdict   action / ok / reason / slotKey — what the server just did on their behalf
	//!   `F` side      key / name                    (subsequent `G` lines attach to it)
	//!   `G` group     callsign                      (subsequent `S` lines attach to it)
	//!   `S` slot      key / role / state / isOwn / holder
	//!
	//! Counts are NOT serialised: `Recount()` derives them from the slots on arrival, so the
	//! headline numbers and the rows are the same data by construction and cannot disagree after
	//! an optimistic edit.
	static string Serialise(TBD_LobbyRoster roster)
	{
		if (!roster)
			return string.Empty;

		array<string> lines = {};

		lines.Insert(Record(3, "M", roster.m_sMissionName, roster.m_sTerrain, roster.m_sStage, string.Empty, string.Empty));

		if (!roster.m_sAction.IsEmpty())
			lines.Insert(Record(4, "V", roster.m_sAction, Flag(roster.m_bActionOk), roster.m_sActionReason, roster.m_sActionKey, string.Empty));

		if (roster.m_bLifeSpent)
			lines.Insert(Record(1, "L", "1", string.Empty, string.Empty, string.Empty, string.Empty));

		if (!roster.IsAvailable())
		{
			lines.Insert(Record(1, "X", roster.m_sUnavailableReason, string.Empty, string.Empty, string.Empty, string.Empty));
			return Join(lines);
		}

		foreach (TBD_LobbySide side : roster.m_aSides)
		{
			lines.Insert(Record(2, "F", side.m_sKey, side.m_sName, string.Empty, string.Empty, string.Empty));

			foreach (TBD_LobbyGroup group : side.m_aGroups)
			{
				lines.Insert(Record(1, "G", group.m_sCallsign, string.Empty, string.Empty, string.Empty, string.Empty));

				foreach (TBD_LobbySlot slot : group.m_aSlots)
				{
					lines.Insert(Record(5, "S", slot.m_sKey, slot.m_sRole, slot.m_sState, Flag(slot.m_bIsOwn), slot.m_sHolder));
				}
			}
		}

		return Join(lines);
	}

	//------------------------------------------------------------------------------------------------
	//! Rebuild a roster on the client. A malformed line is skipped rather than fatal — a picker
	//! that renders most of itself beats a blank screen (design law: nothing blocking).
	static TBD_LobbyRoster Parse(string wire)
	{
		TBD_LobbyRoster roster = new TBD_LobbyRoster();

		if (wire.IsEmpty())
		{
			roster.m_sUnavailableReason = "No roster received from the server.";
			return roster;
		}

		array<string> lines = {};
		wire.Split(LINE_SEP, lines, false);

		TBD_LobbySide side;
		TBD_LobbyGroup group;

		foreach (string line : lines)
		{
			array<string> f = {};
			line.Split(FIELD_SEP, f, false);
			if (f.IsEmpty())
				continue;

			string kind = f[0];

			if (kind == "M" && f.Count() >= 4)
			{
				roster.m_sMissionName = Unpack(f[1]);
				roster.m_sTerrain = Unpack(f[2]);
				roster.m_sStage = Unpack(f[3]);
			}
			else if (kind == "V" && f.Count() >= 5)
			{
				roster.m_sAction = Unpack(f[1]);
				roster.m_bActionOk = f[2] == "1";
				roster.m_sActionReason = Unpack(f[3]);
				roster.m_sActionKey = Unpack(f[4]);
			}
			else if (kind == "L" && f.Count() >= 2)
			{
				roster.m_bLifeSpent = f[1] == "1";
			}
			else if (kind == "X" && f.Count() >= 2)
			{
				roster.m_sUnavailableReason = Unpack(f[1]);
			}
			else if (kind == "F" && f.Count() >= 3)
			{
				roster.m_aSides.Insert(new TBD_LobbySide(Unpack(f[1]), Unpack(f[2])));
				side = roster.m_aSides[roster.m_aSides.Count() - 1];
				group = null;
			}
			else if (kind == "G" && f.Count() >= 2 && side)
			{
				side.m_aGroups.Insert(new TBD_LobbyGroup(Unpack(f[1])));
				group = side.m_aGroups[side.m_aGroups.Count() - 1];
			}
			else if (kind == "S" && f.Count() >= 6 && group)
			{
				group.m_aSlots.Insert(new TBD_LobbySlot(Unpack(f[1]), Unpack(f[2]), Unpack(f[3]), Unpack(f[5]), f[4] == "1"));
			}
		}

		roster.Recount();
		return roster;
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! One record builder rather than five overloads. `fields` says how many of a..e are real; the
	//! rest are not emitted, so a record is never padded with sentinels it does not need.
	protected static string Record(int fields, string kind, string a, string b, string c, string d, string e)
	{
		string line = kind;

		if (fields >= 1)
			line = line + FIELD_SEP + Pack(a);

		if (fields >= 2)
			line = line + FIELD_SEP + Pack(b);

		if (fields >= 3)
			line = line + FIELD_SEP + Pack(c);

		if (fields >= 4)
			line = line + FIELD_SEP + Pack(d);

		if (fields >= 5)
			line = line + FIELD_SEP + Pack(e);

		return line;
	}

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
			TBD_Log.Warn(CH_LOBBY, string.Format("roster clipped at %1 lines (mission has more) — raise MAX_PAYLOAD_LINES", MAX_PAYLOAD_LINES));
		}

		return result;
	}

	//------------------------------------------------------------------------------------------------
	//! Empty -> sentinel. See the EMPTY constant for why this exists.
	protected static string Pack(string value)
	{
		if (value.IsEmpty())
			return EMPTY;

		return Sanitise(value);
	}

	//------------------------------------------------------------------------------------------------
	protected static string Unpack(string value)
	{
		if (value == EMPTY)
			return string.Empty;

		return value;
	}

	//------------------------------------------------------------------------------------------------
	protected static string Flag(bool value)
	{
		if (value)
			return "1";

		return "0";
	}

	//------------------------------------------------------------------------------------------------
	//! Strip the separators out of authored text and out of player DISPLAY NAMES, so a name
	//! containing a tab cannot shift every field of its record.
	//!
	//! MEASURED (T-181.9.2, and relearned by the compile gate): `string.Replace` mutates the
	//! receiver IN PLACE and returns the replacement COUNT, not the new string. `s = s.Replace(a,b)`
	//! does not compile.
	protected static string Sanitise(string value)
	{
		if (value.IsEmpty())
			return value;

		string clean = value;
		clean.Replace(FIELD_SEP, " ");
		clean.Replace(LINE_SEP, " ");
		clean.Replace("\r", " ");
		return clean;
	}
}
