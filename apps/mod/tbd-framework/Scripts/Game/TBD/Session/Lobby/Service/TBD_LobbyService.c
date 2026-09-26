/**
 * @file TBD_LobbyService.c
 * @brief Builds one player's lobby roster on the server and carries out their claim, release and deploy.
 *
 * Role: parses TBD_SpawnManager.BuildSlotRoster into a TBD_LobbyRoster with display names and the
 * reader's own seat, and turns the three seat actions into a verdict and a sentence.
 * Position: called by the modded SCR_PlayerController's lobby RPCs on the server (and in place on a
 * listen host); the roster then goes through TBD_LobbyRosterWire to TBD_LobbyClient.
 * State: none.  Invariants: the roster is only ever the parse of BuildSlotRoster, never a second
 * opinion on who holds a seat; a row without exactly ROSTER_COLUMNS fields, or whose key does not
 * survive TBD_WireCodec.Sanitise, is skipped with a WARNING; every display string passes
 * TBD_WireCodec.Sanitise; the in-world fact is set before any early return.
 */

//! Server roster builder and seat actions of the lobby. The lobby shows both sides' full ORBAT
//! with holders, unlike the briefing, which is side-scoped: a player picks a side from it.
class TBD_LobbyService
{
	static const string CH_LOBBY = "Lobby"; //!< log channel of the lobby wire and seat actions
	static const string STATE_OPEN = "OPEN"; //!< roster state: nobody holds the seat
	static const string STATE_HELD = "HELD"; //!< roster state: a living player holds the seat
	static const string STATE_DEAD = "DEAD"; //!< roster state: the holder spent their life (holder -1 when they left)
	static const string ACTION_CLAIM = "CLAIM"; //!< verdict action of a claim
	static const string ACTION_RELEASE = "RELEASE"; //!< verdict action of a release
	static const string ACTION_DEPLOY = "DEPLOY"; //!< verdict action of a deploy
	protected static const int ROSTER_COLUMNS = 6; //!< fields of one TBD_SlotRosterWire line; BuildForPlayer requires exactly this many

	//! Build the roster `playerId` sees now: stage, in-world and life-spent flags, mission name and terrain, and every seat of both sides with holder names and the reader's own seat marked.
	//! @param playerId the reader; always the controller the request arrived on
	//! @return the roster; carries an unavailable reason when the mission is loading, the spawn manager is missing or the mission has no slots
	//! @authority server
	static TBD_LobbyRoster BuildForPlayer(int playerId)
	{
		TBD_LobbyRoster roster = new TBD_LobbyRoster();

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (fm)
			roster.m_sStage = typename.EnumToString(TBD_EGameStage, fm.GetStage());

		PlayerManager players = GetGame().GetPlayerManager();

		// Resolved before every early return: a reply that says the roster is unavailable still tells a
		// player with a body that they have one, so the picker stands down over a live character.
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
			roster.m_sMissionName = TBD_WireCodec.Sanitise(doc.meta.name);
			roster.m_sTerrain = TBD_WireCodec.Sanitise(doc.meta.terrain);
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			roster.m_sUnavailableReason = "The server is still starting up.";
			return roster;
		}

		roster.m_bLifeSpent = spawn.IsPlayerDead(playerId);

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
			row.Split(TBD_WireCodec.FIELD_SEP, f, false);

			// Exact arity: an authored tab in a slot, faction, callsign or role shifts every later column,
			// so a short or long row is skipped loudly rather than shown under the wrong squad or state.
			if (f.Count() != ROSTER_COLUMNS)
			{
				TBD_Log.Warn(CH_LOBBY, string.Format(
					"skipped malformed roster row (%1 fields, expected %2 - a separator in an authored slot/faction/callsign/role?): '%3'",
					f.Count(), ROSTER_COLUMNS, row));
				continue;
			}

			string slotKey = f[0];

			// A key that Sanitise rewrites would come back from the client as a different string that
			// ClaimSlot refuses, so the seat is dropped rather than shown unclaimable.
			if (TBD_WireCodec.Sanitise(slotKey) != slotKey)
			{
				TBD_Log.Warn(CH_LOBBY, string.Format(
					"skipped slot whose key carries a line separator - it could not round-trip the wire: '%1'", row));
				continue;
			}
			string factionKey = f[1];
			string callsign = f[2];
			string role = f[3];
			string state = f[4];
			int holderId = f[5].ToInt();

			string holder;
			if (holderId > 0 && players)
				holder = TBD_WireCodec.Sanitise(players.GetPlayerName(holderId));

			bool isOwn = !ownKey.IsEmpty() && slotKey == ownKey;

			TBD_LobbySide side = AcquireSide(roster, doc, factionKey);
			TBD_LobbyGroup group = AcquireGroup(side, TBD_WireCodec.Sanitise(callsign));

			group.m_aSlots.Insert(new TBD_LobbySlot(slotKey, TBD_WireCodec.Sanitise(role), state, holder, isOwn));
		}

		// Counts come from the slots, the same pass the client runs after an optimistic edit.
		roster.Recount();
		return roster;
	}

	//! Take `slotKey` for `playerId` through TBD_SpawnManager.ClaimSlot, which owns the rule (first come, not dead, not held by another).
	//! @param accepted true when the seat is now theirs; the client latches on it
	//! @return the sentence shown to the player, naming the reason on a refusal
	//! @authority server
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

		return DescribeRefusal(spawn, slotKey);
	}

	//! Turn a refused claim into its reason, read back off the authority's roster row for `slotKey`.
	//! @return the sentence: unknown seat, a dead holder, who got there first, or a generic refusal
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
			row.Split(TBD_WireCodec.FIELD_SEP, f, false);

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

	//! Give back `playerId`'s seat through TBD_SpawnManager.ReleaseSlot, which refuses once the life is spent or the player has deployed.
	//! @param accepted true when the seat was released
	//! @return the sentence shown to the player
	//! @authority server
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
			return "Your life is spent - the seat stays yours.";

		return "You are already in the world; the seat is yours.";
	}

	//! Deploy `playerId` through TBD_SpawnManager.DeployPlayerEx, the one-life boundary, and word its verdict. AUTHORIZING and UNAUTHORIZED (the TBD platform decides the seat) are not accepted: the screen latches an accepted deploy as done while the platform may still refuse.
	//! @param accepted true for DEPLOYED or ALREADY
	//! @param resultName the TBD_EDeployResult name; empty when the request never reached DeployPlayerEx
	//! @return the sentence shown to the player
	//! @authority server
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
			return "The server is not ready to deploy you yet - try again in a moment.";

		if (result == TBD_EDeployResult.NOT_MINE)
			return "No framework mission is loaded.";

		if (result == TBD_EDeployResult.AUTHORIZING)
			return "Checking your seat with the TBD platform - you deploy once it allows it; a refusal arrives in chat.";

		if (result == TBD_EDeployResult.UNAUTHORIZED)
			return "Your seat cannot be authorized right now - the reason is in your chat. The seat stays yours; deploy again shortly.";

		return "Deploy failed - the slot body could not be prepared. Tell an admin.";
	}

	//! The side of `factionKey` in `roster`, appended with its sanitised display name on first use.
	//! @return the side; never null
	protected static TBD_LobbySide AcquireSide(TBD_LobbyRoster roster, TBD_MissionDocumentStruct doc, string factionKey)
	{
		foreach (TBD_LobbySide existing : roster.m_aSides)
		{
			if (existing.m_sKey == factionKey)
				return existing;
		}

		roster.m_aSides.Insert(new TBD_LobbySide(factionKey, TBD_WireCodec.Sanitise(TBD_MissionFactionNames.DisplayName(doc, factionKey))));
		return roster.m_aSides[roster.m_aSides.Count() - 1];
	}

	//! The squad `callsign` in `side`, appended on first use.
	//! @return the squad; never null
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
}
