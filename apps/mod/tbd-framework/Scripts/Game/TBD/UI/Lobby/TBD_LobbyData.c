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

	//! T-181.29 — the authority's answer to "does this reader already have a body?".
	//!
	//! ── Why the client cannot answer this for itself, and why the screen needed it ───────────
	//! The screen stands down on `TBD_LobbyClient.IsDeployed()`, which is latched by exactly ONE
	//! event: a DEPLOY verdict the player's own click asked for. Every OTHER door into the world is
	//! server-side and silent to this client — `TBD_SpawnManager`'s LOBBY auto-deploy wave
	//! (`m_bAutoDeploy`, still 1), the JIP `DeployJoiner` path, and `AdminRespawn`. A player any of
	//! those put in the world therefore had a live character AND the picker sitting on top of it,
	//! with nothing that would ever take it down.
	//!
	//! So the fact travels the same way every other fact this screen draws travels: computed on the
	//! authority in `BuildForPlayer`, carried on the wire, rebuilt on the client. It sits next to
	//! `m_bLifeSpent` because it is the same KIND of fact — something only the server can know about
	//! this player, which changes what the screen is allowed to offer.
	//!
	//! ── Deliberately NOT latched ────────────────────────────────────────────────────────────
	//! `m_bDeployed` latches for good; this does not. It is a per-roster observation, so a reading
	//! that turns out to be wrong is corrected by the very next refresh (2 s at worst) and the
	//! picker comes back on `TBD_LobbyStage.Tick`'s unconditional re-raise. That property is what
	//! makes closing on it safe: the worst case is a picker that flickers, never one that is gone
	//! for good — which is the failure `Raise()` is deliberately built to avoid.
	bool m_bInWorld;

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
//!
//! ── T-181.42: this was the last payload on a lossy sentinel ─────────────────────────────────
//! `TBD_AdminData` and `TBD_BriefingData` (T-181.26) both mark every field `<TAB>.<value>`, which
//! is BIJECTIVE. This file wrote `EMPTY = "~"`, which is a plausibility argument, and was the odd
//! one out. It now uses the same marker under the same names (`FIELD_MARK` / `Field` / `Unmark` /
//! `IsSet`), so all three delimited payloads answer the question once. See `FIELD_MARK`.
//!
//! Three things landed with that convergence, and only the first is the rename:
//!   1. the bijective marker — fixes a LATENT loss (no golden mission contains a `~`);
//!   2. `Parse` no longer leaves a rejected `F`/`G` pointing its cursor at the previous side or
//!      squad, so orphan rows are dropped instead of MISFILED (see `Parse`);
//!   3. `BuildForPlayer` now rejects a roster row whose column count is not exactly
//!      `ROSTER_COLUMNS`, which is the one defect here that is reachable from authored JSON the
//!      schema permits (see the guard for the measurement).
//!
//! And the load-bearing half: `SelfCheckWire` proves all of it AT BOOT, from
//! `TBD_LobbyComponent.OnPostInit`, because `world-boot.sh` runs with zero players and would never
//! otherwise execute a single line of this wire.
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

	//! Column count of one `TBD_SpawnManager.BuildSlotRoster()` row:
	//! `<slotKey>\t<faction>\t<group>\t<role>\t<state>\t<holderPlayerId>`. Named because the check
	//! against it is EXACT and the reason for that is load-bearing — see `BuildForPlayer`.
	protected static const int ROSTER_COLUMNS = 6;

	protected static const string FIELD_SEP = "\t";
	protected static const string LINE_SEP = "\n";

	//! T-181.42 — the per-field marker. Every field on the wire is written `<TAB>.<value>`, so no
	//! token is ever the empty string and `Unmark()` strips the marker back off. Same character and
	//! same semantics as `TBD_AdminData.FIELD_MARK` and `TBD_BriefingService.FIELD_MARK`,
	//! deliberately: one convention across all three delimited payloads, not three answers.
	//!
	//! ── What this replaced, and why it was not merely a rename ──────────────────────────────
	//! This file used to write `EMPTY = "~"`: empty->`~` on the way out, `~`->empty on the way
	//! back, defended as "`~` is not a plausible whole-field value". That is a plausibility
	//! argument about what a human would type, and it is LOSSY exactly where it is wrong — a squad
	//! callsign, role or player display name of literally `~` round-trips to the empty string. The
	//! marker is BIJECTIVE instead: `Unmark(Field(x)) == Sanitise(x)` for EVERY `x`, `~` and `.`
	//! and the empty string included, at a cost of one byte per field. Correctness stops depending
	//! on a guess about content.
	//!
	//! Scope of the win, stated honestly: no committed golden mission contains a `~`, so the old
	//! sentinel was not losing a field on any data this program actually ships. The defect it fixes
	//! is latent, and it is an AUTHORING hazard rather than a live one.
	//!
	//! ── The marker must stay a single ASCII byte ────────────────────────────────────────────
	//! `Unmark` is `Substring(1, length - 1)`, and MEASURED: `string.Length()` counts BYTES and
	//! `Substring` is BYTE-indexed (`"·".Length()` is 2; `"café latte".Substring(0, 4)` returns a
	//! broken UTF-8 sequence). A one-byte ASCII marker is skipped safely whatever the value's own
	//! encoding is; a prettier multi-byte one would corrupt every accented callsign.
	//!
	//! ── What this does NOT rest on ──────────────────────────────────────────────────────────
	//! The old comment here said `string.Split`'s empty-token behaviour "is not something this lane
	//! can prove". T-181.26 proved it — engine 1.7.0.54 KEEPS empty tokens — which means the old
	//! `~` scheme's field counts were never actually at risk on this build either. The marker is
	//! kept regardless, because that is one measured behaviour of one engine build and a `trim =
	//! true` caller would reintroduce the hazard immediately. `SelfCheckWire` reports the observed
	//! behaviour on every boot without depending on it.
	protected static const string FIELD_MARK = ".";

	//! T-181.42 — the wire self-check has run once this process, and what it concluded. Statics, so
	//! they survive a world change inside one process: what is being proven is a property of the
	//! ENGINE BUILD and of this format, and a new round changes neither.
	//!
	//! The guard lives at `SelfCheckWire`'s OWN entry rather than at a caller. T-181.26 armed the
	//! briefing's equivalent from `Serialise` and a later direct call from the framework roll-call
	//! bypassed that guard and ran it twice. A self-check that can be armed from more than one place
	//! must own its own once-ness.
	protected static bool s_bWireChecked;
	protected static bool s_bWireOk;

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

		PlayerManager players = GetGame().GetPlayerManager();

		// T-181.29 — resolved FIRST, ahead of every early return below, because it is the one fact
		// on this roster that stays true when the rest of it cannot be built. A player the deploy
		// wave already put in the world must be told so even on a reply whose body is
		// "Mission is still loading." — otherwise the exact reply that says the roster is
		// unavailable is also the one that leaves the picker sitting over their character.
		//
		// `GetPlayerControlledEntity` is this tree's established in-world test, not a new one:
		// `TBD_AdminData` (:270) derives its `in world` column from it, `TBD_SpectatorHost`,
		// `TBD_SafestartManager`, `TBD_PlayAreaComponent`, `TBD_ObjectivesComponent` and
		// `TBD_SpawnManager` itself all ask it the same question. It is also the SERVER-side twin of
		// `SCR_PlayerController.GetLocalControlledEntity()`, which `TBD_LobbyStage.Tick` (T-181.28)
		// already calls the reliable half of its re-raise guard.
		// Written as a guarded assignment rather than `players && ... != null` deliberately: the
		// field already defaults to false, so the guard is complete, and this is character-for-
		// character the idiom `TBD_AdminData` (:270) uses for the same question. A `&&` whose left
		// operand is a class ref compiles here, but "compiles" and "means what it reads as" are
		// different claims in this language and only one of them is worth resting a screen on.
		if (players)
			roster.m_bInWorld = players.GetPlayerControlledEntity(playerId) != null;

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

		foreach (string row : rows)
		{
			array<string> f = {};
			row.Split(FIELD_SEP, f, false);

			// A malformed row is skipped, never guessed at: mis-decoding one would attribute a
			// seat to the wrong player, which is the one error class this screen must not make.
			//
			// ── T-181.42: EXACTLY, not at-least ──────────────────────────────────────────────
			// This guard used to read `< 6`, and a too-LONG row is the dangerous one. MEASURED:
			// `TBD_SpawnManager.BuildSlotRoster` (:765) formats `slot.Key()`, `slot.faction`,
			// `slot.groupCallsign` and `slot.role` into a tab-separated row with NO sanitisation,
			// and `mission.schema.json` puts `minLength: 1` but NO `pattern` on any of them. So a
			// callsign authored as `AL<TAB>PHA` yields a SEVEN-field row that sailed straight past
			// `< 6` and shifted every column after it: the role rendered as `PHA`, the real role
			// was read as the STATE (so the seat matched neither OPEN nor DEAD and became
			// unselectable), and `holderId` came from `"OPEN".ToInt()` = 0. That is the exact shape
			// T-181.26 found in the briefing's `>= 5` guard — an authored string reaching a
			// delimited wire unsanitised and passing a one-sided count check.
			//
			// Rejecting turns a silent MISATTRIBUTION into a loud OMISSION, which is the right
			// trade on a roster: a missing seat is visible and recoverable, a seat shown under the
			// wrong squad is a player taking the wrong slot. The row count is fixed by
			// `BuildSlotRoster`'s own documented format, so anything else means either an injected
			// separator or an upstream column change — and both should stop here and say so.
			//
			// The real fix belongs upstream, in `BuildSlotRoster`'s `string.Format`. That file is
			// `Gamemode/**` and owned by another slice this wave; reported rather than edited.
			if (f.Count() != ROSTER_COLUMNS)
			{
				TBD_Log.Warn(CH_LOBBY, string.Format(
					"skipped malformed roster row (%1 fields, expected %2 — a separator in an authored slot/faction/callsign/role?): '%3'",
					f.Count(), ROSTER_COLUMNS, row));
				continue;
			}

			string slotKey = f[0];

			// A key that does not survive `Sanitise` cannot survive the wire either: `Field()`
			// would rewrite it, the client would send a DIFFERENT string back, and `ClaimSlot`
			// would refuse a seat that looks perfectly claimable. Arity above already rules out a
			// tab, so this catches a newline or carriage return in an authored `uid`/`id`. Drop it
			// loudly rather than shipping a seat nobody can ever take.
			if (Sanitise(slotKey) != slotKey)
			{
				TBD_Log.Warn(CH_LOBBY, string.Format(
					"skipped slot whose key carries a line separator — it could not round-trip the wire: '%1'", row));
				continue;
			}
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

			// Exact, for the same reason `BuildForPlayer` is exact: a shifted row would read the
			// STATE column out of the role and explain the refusal wrongly.
			if (f.Count() != ROSTER_COLUMNS || f[0] != slotKey)
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
	//!   `D` deployed  "1" when this reader already has a body (T-181.29)
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

		// T-181.29 — emitted ABOVE the `X` early return, alongside `L`, for the reason
		// `BuildForPlayer` resolves it first: "you already have a body" has to survive a reply whose
		// roster could not be built, or the unavailable-roster case is exactly the one that strands
		// the picker over a live character.
		if (roster.m_bInWorld)
			lines.Insert(Record(1, "D", "1", string.Empty, string.Empty, string.Empty, string.Empty));

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
	//!
	//! ── T-181.42: a skipped record must also invalidate the CURSOR ──────────────────────────
	//! `F` and `G` are stateful: they set the side/group that the following lines attach to. The
	//! original code only advanced that cursor on a record it ACCEPTED, and left it pointing at the
	//! previous side/group on one it rejected — so the slots belonging to a dropped squad were
	//! silently inserted into the squad ABOVE them. On a roster that is the worst available failure:
	//! not a missing row but a seat shown under the wrong callsign, and potentially under the wrong
	//! SIDE. Omission is recoverable by a refresh; misattribution is a player taking the wrong seat.
	//!
	//! Both handlers now clear their cursor FIRST and only re-arm it on success, so an orphan row
	//! is dropped rather than misfiled. `SelfCheckWire` phase 2 feeds this path a deliberately
	//! truncated `F` and `G` and asserts the orphans do not land, so the fix is gated rather than
	//! merely argued.
	//!
	//! Reachability, honestly: `Serialise` cannot emit a short record, and `Join` clips only on line
	//! boundaries, so on a well-formed wire no record is ever rejected and this bug is LATENT. It is
	//! fixed because a truncated payload is exactly the situation in which a picker must not lie.
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
				roster.m_sMissionName = Unmark(f[1]);
				roster.m_sTerrain = Unmark(f[2]);
				roster.m_sStage = Unmark(f[3]);
			}
			else if (kind == "V" && f.Count() >= 5)
			{
				roster.m_sAction = Unmark(f[1]);
				roster.m_bActionOk = IsSet(f[2]);
				roster.m_sActionReason = Unmark(f[3]);
				roster.m_sActionKey = Unmark(f[4]);
			}
			else if (kind == "L" && f.Count() >= 2)
			{
				roster.m_bLifeSpent = IsSet(f[1]);
			}
			else if (kind == "D" && f.Count() >= 2)
			{
				// T-181.29. Absent record = false = "not in the world" = keep the picker up, which
				// is the safe default: `IsSet` already fails a corrupt token to false, so every way
				// this record can go wrong leaves the player looking at the screen rather than
				// silently robbed of it.
				roster.m_bInWorld = IsSet(f[1]);
			}
			else if (kind == "X" && f.Count() >= 2)
			{
				roster.m_sUnavailableReason = Unmark(f[1]);
			}
			else if (kind == "F")
			{
				// Cursor cleared FIRST. A rejected side must not leave the following groups
				// attaching to the PREVIOUS side — see the header.
				side = null;
				group = null;

				if (f.Count() >= 3)
				{
					roster.m_aSides.Insert(new TBD_LobbySide(Unmark(f[1]), Unmark(f[2])));
					side = roster.m_aSides[roster.m_aSides.Count() - 1];
				}
			}
			else if (kind == "G")
			{
				// Same rule one level down: a rejected squad must not donate its seats to the
				// squad above it.
				group = null;

				if (f.Count() >= 2 && side)
				{
					side.m_aGroups.Insert(new TBD_LobbyGroup(Unmark(f[1])));
					group = side.m_aGroups[side.m_aGroups.Count() - 1];
				}
			}
			else if (kind == "S" && f.Count() >= 6 && group)
			{
				group.m_aSlots.Insert(new TBD_LobbySlot(Unmark(f[1]), Unmark(f[2]), Unmark(f[3]), Unmark(f[5]), IsSet(f[4])));
			}
		}

		roster.Recount();
		return roster;
	}

	// ── SELF-CHECK ──────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Prove this wire format on the machine that runs it, at BOOT, with nobody connected.
	//!
	//! ── Why this exists at all, and why it is armed at boot rather than on first use ────────
	//! MEASURED 2026-07-25 (T-181.26): `world-boot.sh --mission=` runs with **ZERO players**. So
	//! `BuildForPlayer`, `Serialise`, `Parse`, every RPC and every payload in this file never
	//! execute under the gate — a `grep -i briefing` over a full `--mission` console log returned
	//! only `flow.briefingSeconds`. The harness runs the boot-time spine and nothing else.
	//!
	//! The consequence is the whole reason this function is wired where it is: **a self-check armed
	//! lazily on first use is invisible to the gate, and one armed at boot is gated.** This one is
	//! called from `TBD_LobbyComponent.OnPostInit`, which is a component on `TBD_GameMode.et` and
	//! therefore runs on every world boot including a zero-player headless one. A deliberately
	//! broken `Unmark` produces `SCRIPT (E): [TBD][Lobby] wire self-check FAIL ...` and
	//! `world-boot.sh`'s fail-closed triage turns that into `WORLD BOOT: FAIL`.
	//!
	//! ── What it actually proves ─────────────────────────────────────────────────────────────
	//! Phase 1 round-trips a roster that is empty in every position an empty can legally reach, AND
	//! carries the two values a plausibility-based sentinel gets wrong: a field of literally `~`
	//! (this format's OLD sentinel) and a field of literally `.` (its current marker). Under the
	//! retired `~` scheme a squad, role, terrain or SLOT KEY of `~` decoded to the empty string; an
	//! empty slot key is a seat nobody can claim and a `HasOwnSlot()` that reads false for a player
	//! who does hold a seat, which disables DEPLOY. That is the concrete harm, and it is what the
	//! `terrain`/`slotKey` assertions below pin.
	//!
	//! Phase 2 feeds `Parse` a deliberately MALFORMED wire — a truncated `G` and a truncated `F`,
	//! each followed by slot rows — and asserts the orphans are DROPPED rather than attached to the
	//! squad above them. See `Parse`'s header: misattribution is the one error a roster must not
	//! make, and without this phase the fix for it would be untested.
	//!
	//! It also OBSERVES `string.Split`'s empty-token behaviour and reports it either way. The
	//! correctness of this format no longer depends on the answer — that is the point of the
	//! marker — but four files in this tree hand-roll splitters over that question, so the
	//! measurement is worth reprinting on every boot rather than trusting a note in a doc.
	//!
	//! Allocation-light, runs in microseconds, once per process. Cheap enough to be unconditional.
	//!
	//! @return true when both phases are clean. Callers may ignore it; the log line is the product.
	static bool SelfCheckWire()
	{
		// Guarded at the ENTRY, not at the caller — see s_bWireChecked.
		if (s_bWireChecked)
			return s_bWireOk;

		s_bWireChecked = true;

		// ── Direct observation of the behaviour this scheme deliberately does not depend on ──
		array<string> probe = {};
		string sample = "a" + FIELD_SEP + FIELD_SEP + "b";
		sample.Split(FIELD_SEP, probe, false);

		string splitVerdict = "dropped";
		if (probe.Count() >= 3)
			splitVerdict = "kept";

		array<string> faults = {};

		SelfCheckRoundTrip(faults);
		SelfCheckOrphans(faults);

		if (faults.IsEmpty())
		{
			// PrintFormat/string.Format, never Print(localVariable) — MEASURED: Print emits the
			// DECLARATION of a local, not its value.
			TBD_Log.Event(CH_LOBBY, string.Format(
				"wire self-check PASS marker=bijective empty-fields=lossless orphan-rows=dropped split-empties=%1",
				splitVerdict));

			s_bWireOk = true;
			return true;
		}

		string detail;
		foreach (string fault : faults)
		{
			if (!detail.IsEmpty())
				detail = detail + ",";

			detail = detail + fault;
		}

		TBD_Log.Error(CH_LOBBY, string.Format(
			"wire self-check FAIL split-empties=%1 lost=%2 — a lobby field does not survive the wire",
			splitVerdict, detail));

		s_bWireOk = false;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 1 — a roster empty in every legal position, plus the two literal values a
	//! plausibility sentinel gets wrong. Faults are appended, never logged here, so one FAIL line
	//! names everything that broke instead of one line per field.
	protected static void SelfCheckRoundTrip(notnull array<string> faults)
	{
		TBD_LobbyRoster sent = new TBD_LobbyRoster();
		sent.m_sMissionName = string.Empty;      // meta.name: minLength 1 in the schema, unenforced on the profile path
		sent.m_sTerrain = "~";                   // the RETIRED sentinel, as a literal value
		sent.m_sStage = "LOBBY";
		sent.m_bLifeSpent = true;
		sent.m_bInWorld = true;                  // T-181.29 — see the assertion below
		sent.m_sAction = ACTION_CLAIM;
		sent.m_bActionOk = false;
		sent.m_sActionReason = string.Empty;
		sent.m_sActionKey = FIELD_MARK;          // the CURRENT marker, as a literal value

		sent.m_aSides.Insert(new TBD_LobbySide(string.Empty, "~"));
		TBD_LobbySide side = sent.m_aSides[0];

		// A leading EMPTY callsign is the record most likely to take its whole squad down with it.
		side.m_aGroups.Insert(new TBD_LobbyGroup(string.Empty));
		side.m_aGroups[0].m_aSlots.Insert(new TBD_LobbySlot("a.b", string.Empty, STATE_OPEN, string.Empty, false));
		side.m_aGroups[0].m_aSlots.Insert(new TBD_LobbySlot("~", "SL", STATE_HELD, "Bob", true));

		side.m_aGroups.Insert(new TBD_LobbyGroup("BRAVO"));
		side.m_aGroups[1].m_aSlots.Insert(new TBD_LobbySlot("c", "RFL", STATE_DEAD, string.Empty, false));

		TBD_LobbyRoster got = Parse(Serialise(sent));

		if (!got.m_sMissionName.IsEmpty())
			faults.Insert("missionName");

		// The bijectivity assertion. Under the retired `~` sentinel this decoded to the empty string.
		if (got.m_sTerrain != "~")
			faults.Insert("terrain");

		if (got.m_sStage != "LOBBY")
			faults.Insert("stage");

		if (!got.m_bLifeSpent)
			faults.Insert("lifeSpent");

		// T-181.29 — the ONE thing about this slice the gate can actually execute.
		//
		// MEASURED (T-181.26, restated in this function's header): `world-boot.sh --mission=` runs
		// with ZERO players, so `BuildForPlayer` never resolves a real `GetPlayerControlledEntity`,
		// no RPC moves, no screen opens, and nothing closes. The BEHAVIOUR this slice adds is
		// therefore unobservable from the harness end to end. What IS observable is that the fact
		// survives the wire — and a `D` record that silently failed to round-trip would leave the
		// picker exactly as stuck as it was before, with no symptom anywhere else. So it is pinned
		// here, where a break becomes `wire self-check FAIL ... lost=inWorld` and `world-boot.sh`'s
		// fail-closed triage turns that into `WORLD BOOT: FAIL`.
		if (!got.m_bInWorld)
			faults.Insert("inWorld");

		if (got.m_sAction != ACTION_CLAIM || got.m_bActionOk || !got.m_sActionReason.IsEmpty() || got.m_sActionKey != FIELD_MARK)
			faults.Insert("verdict");

		if (got.m_aSides.Count() != 1)
		{
			faults.Insert("sideCount");
			return;
		}

		TBD_LobbySide back = got.m_aSides[0];
		if (!back.m_sKey.IsEmpty() || back.m_sName != "~")
			faults.Insert("side");

		if (back.m_aGroups.Count() != 2)
		{
			faults.Insert("groupCount");
			return;
		}

		// The empty-callsign squad kept its identity AND exactly its own two seats — it did not
		// vanish, and it did not absorb BRAVO's.
		TBD_LobbyGroup first = back.m_aGroups[0];
		if (!first.m_sCallsign.IsEmpty() || first.m_aSlots.Count() != 2)
		{
			faults.Insert("group");
		}
		else
		{
			if (first.m_aSlots[0].m_sKey != "a.b" || !first.m_aSlots[0].m_sRole.IsEmpty()
				|| !first.m_aSlots[0].IsOpen() || !first.m_aSlots[0].m_sHolder.IsEmpty() || first.m_aSlots[0].m_bIsOwn)
				faults.Insert("slotOpen");

			// A slot KEY of `~` is the harm case: the retired sentinel decoded it to the empty
			// string, which is a seat that cannot be claimed and a DEPLOY button that stays dead.
			if (first.m_aSlots[1].m_sKey != "~" || first.m_aSlots[1].m_sRole != "SL"
				|| first.m_aSlots[1].m_sState != STATE_HELD || first.m_aSlots[1].m_sHolder != "Bob" || !first.m_aSlots[1].m_bIsOwn)
				faults.Insert("slotHeld");
		}

		TBD_LobbyGroup second = back.m_aGroups[1];
		if (second.m_sCallsign != "BRAVO" || second.m_aSlots.Count() != 1)
		{
			faults.Insert("group2");
		}
		else if (second.m_aSlots[0].m_sKey != "c" || !second.m_aSlots[0].IsDead())
		{
			faults.Insert("slotDead");
		}

		// Recount() re-derives the reader's own seat from the slots, so this is the end-to-end
		// statement that the DEPLOY button lights for the player who holds `~`.
		if (got.m_sOwnKey != "~")
			faults.Insert("ownKey");
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 2 — a malformed wire must lose rows, never MISFILE them. Built through `Record` so it
	//! stays correct if the marker or the separator ever changes.
	protected static void SelfCheckOrphans(notnull array<string> faults)
	{
		array<string> bad = {};
		bad.Insert(Record(2, "F", "us", "US Army", string.Empty, string.Empty, string.Empty));
		bad.Insert(Record(1, "G", "ALPHA", string.Empty, string.Empty, string.Empty, string.Empty));
		bad.Insert(Record(5, "S", "k1", "SL", STATE_OPEN, Flag(false), string.Empty));

		// A truncated squad record, then its seats. They must NOT land in ALPHA.
		bad.Insert("G");
		bad.Insert(Record(5, "S", "k2", "RFL", STATE_OPEN, Flag(false), string.Empty));

		// A truncated side record, then a squad and a seat under it. All three must be dropped.
		bad.Insert("F");
		bad.Insert(Record(1, "G", "GHOST", string.Empty, string.Empty, string.Empty, string.Empty));
		bad.Insert(Record(5, "S", "k3", "RFL", STATE_OPEN, Flag(false), string.Empty));

		TBD_LobbyRoster got = Parse(Join(bad));

		if (got.m_aSides.Count() != 1 || got.m_aSides[0].m_aGroups.Count() != 1)
		{
			faults.Insert("orphanShape");
			return;
		}

		TBD_LobbyGroup only = got.m_aSides[0].m_aGroups[0];
		if (only.m_sCallsign != "ALPHA" || only.m_aSlots.Count() != 1 || only.m_aSlots[0].m_sKey != "k1")
			faults.Insert("orphanRows");
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! One record builder rather than five overloads. `fields` says how many of a..e are real; the
	//! rest are not emitted, so a record is never padded with markers it does not need.
	//!
	//! Appended in STEPS rather than as one `+` chain. MEASURED elsewhere in this tree
	//! (`TBD_AdminData.RecordPlayer`): a nine-field `+` chain trips Enfusion's expression-complexity
	//! ceiling with `Formula too complex`, and the second diagnostic on that line is a misleading
	//! `Incompatible parameter` that sends you hunting a type error which does not exist.
	protected static string Record(int fields, string kind, string a, string b, string c, string d, string e)
	{
		string line = kind;

		if (fields >= 1)
			line = line + Field(a);

		if (fields >= 2)
			line = line + Field(b);

		if (fields >= 3)
			line = line + Field(c);

		if (fields >= 4)
			line = line + Field(d);

		if (fields >= 5)
			line = line + Field(e);

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
	//! `<TAB>.<value>` — separator, marker, value. The marker is what guarantees a NON-EMPTY token
	//! for an empty value; see FIELD_MARK.
	//!
	//! `Sanitise` runs here as well as at build time. That is deliberate belt-and-braces: this is
	//! the single choke point every field passes through, so a field added straight to a `Record`
	//! call later cannot smuggle a raw tab into the stream and shift the rest of its record.
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
	//! produced a bare token — nothing at all. Both mean "empty", and rendering an empty string
	//! beats refusing the whole record: design law, an empty state says what it can rather than
	//! showing a void. Total by construction: `Unmark(Field(x)) == Sanitise(x)` for every `x`.
	protected static string Unmark(string field)
	{
		int length = field.Length();
		if (length <= 1)
			return string.Empty;

		return field.Substring(1, length - 1);
	}

	//------------------------------------------------------------------------------------------------
	//! A marked boolean. Anything that is not a marked `1` reads as false, so a corrupt token fails
	//! to the safe answer — an unclaimed seat, never "this seat is yours".
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
