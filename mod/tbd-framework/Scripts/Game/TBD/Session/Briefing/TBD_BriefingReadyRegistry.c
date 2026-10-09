/**
 * @file TBD_BriefingReadyRegistry.c
 * @brief Who has marked ready in the briefing, and the per-side ready tally.
 *
 * Role: records readiness per player against the current mission id and counts it per faction.
 * Position: SCR_PlayerController's TBD_MarkReady writes and reads it on the server; the tally text
 * goes back to the requesting client only.
 * State: a static map playerId -> TBD_BriefingReadyEntry on the server, for the life of the script
 * VM.  Invariants: a count includes only connected players whose entry names the current mission
 * id, and prunes every other row as it finds it, so the map cannot grow without bound; a tally
 * names one faction, the reader's, and never another side's readiness.
 */

//! One readiness record: the faction the player was ready for, and the mission it was recorded
//! against, so a mission switch invalidates it with no reset hook.
class TBD_BriefingReadyEntry
{
	string m_sFaction; //!< faction key of the player's slot when they marked ready
	string m_sMissionId; //!< `meta.id` of the mission loaded when they marked ready

	//! Record one readiness.
	//! @param faction the faction key
	//! @param missionId the current mission id
	void TBD_BriefingReadyEntry(string faction, string missionId)
	{
		m_sFaction = faction;
		m_sMissionId = missionId;
	}
}

//! Server readiness bookkeeping for the briefing stage.
class TBD_BriefingReadyRegistry
{
	protected static ref map<int, ref TBD_BriefingReadyEntry> m_mReady; //!< playerId -> readiness record; null until first use

	//! Create the map on first use.
	protected static void Ensure()
	{
		if (!m_mReady)
			m_mReady = new map<int, ref TBD_BriefingReadyEntry>();
	}

	//! Record that `playerId` is ready for `factionKey` in the current mission.
	//! @param playerId the player
	//! @param factionKey the faction key of their slot
	//! @authority server
	static void SetReady(int playerId, string factionKey)
	{
		Ensure();
		m_mReady.Set(playerId, new TBD_BriefingReadyEntry(factionKey, TBD_MissionLoader.GetMissionId()));
	}

	//! Count the ready players of one faction in the current mission. A row whose player has
	//! disconnected, or whose mission id is not the current one, is pruned instead of counted.
	//! @param factionKey the faction to count
	//! @return the number of connected players ready for `factionKey` in this mission
	//! @authority server
	static int CountReadyForFaction(string factionKey)
	{
		Ensure();

		array<int> connected = {};
		GetGame().GetPlayerManager().GetPlayers(connected);

		string missionId = TBD_MissionLoader.GetMissionId();

		array<int> stale = {};
		int n = 0;

		foreach (int playerId, TBD_BriefingReadyEntry entry : m_mReady)
		{
			if (!entry || connected.Find(playerId) < 0 || entry.m_sMissionId != missionId)
			{
				stale.Insert(playerId);
				continue;
			}

			if (entry.m_sFaction == factionKey)
				n++;
		}

		// Collect first, then remove by key: a map must not change during its own foreach.
		foreach (int gone : stale)
		{
			m_mReady.Remove(gone);
		}

		return n;
	}

	//! The tally line `Ready -- <ready> of <total> on <factionName>`, where the total is the side's
	//! claimed slots (at least the ready count). Built on the server, which alone can count a side.
	//! @param factionKey the reader's faction key
	//! @param factionName the reader's faction display name
	//! @return the finished tally text
	//! @authority server
	static string BuildTally(string factionKey, string factionName)
	{
		int ready = CountReadyForFaction(factionKey);

		int total = ready;
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (spawn)
			total = spawn.CountClaimedForFaction(factionKey);

		if (total < ready)
			total = ready;

		return string.Format("Ready -- %1 of %2 on %3", ready, total, factionName);
	}
}
