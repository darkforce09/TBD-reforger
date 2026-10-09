/**
 * @file TBD_DebriefScoreboard.c
 * @brief Builds the DEBRIEF scoreboard rows from the connected players.
 *
 * Role: one `TBD_DebriefRow` per connected player with name, slot faction and role, deaths and
 * kills.  Position: called by `TBD_FrameworkManager` on the authority when it packs the debrief
 * board; reads `PlayerManager`, `TBD_SpawnManager` and `TBD_MatchTelemetryTally`.
 * State: none.  Invariants: runs only on the authority, and a remote client gets no rows; deaths
 * are 0 or 1 from the one-life record; a player with no name is listed as "Player <id>".
 */

//! Scoreboard row builder for the DEBRIEF overlay.
class TBD_DebriefScoreboard
{
	//! Clears `outRows`, then appends one row per connected player: name ("Player <id>" when the
	//! name is empty), faction and role from the assigned slot, deaths 1 when
	//! `TBD_SpawnManager.IsPlayerDead` and 0 otherwise, and kills from
	//! `TBD_MatchTelemetryTally.GetKills`. Leaves `outRows` empty on a remote client or with no
	//! player manager; a missing spawn manager leaves the slot and death fields at their defaults.
	//! @authority server
	static void Fill(notnull array<ref TBD_DebriefRow> outRows)
	{
		outRows.Clear();

		if (TBD_Authority.IsClient())
			return;

		PlayerManager players = GetGame().GetPlayerManager();
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		if (!players)
			return;

		array<int> ids = {};
		int count = players.GetPlayers(ids);
		for (int i = 0; i < count; i++)
		{
			int playerId = ids[i];
			TBD_DebriefRow row = new TBD_DebriefRow();
			row.m_sName = players.GetPlayerName(playerId);
			if (row.m_sName.IsEmpty())
				row.m_sName = string.Format("Player %1", playerId);

			TBD_MissionSlotStruct slot;
			if (sm)
				slot = sm.GetAssignedSlot(playerId);
			if (slot)
			{
				row.m_sFaction = slot.faction;
				row.m_sRole = slot.role;
			}

			row.m_iDeaths = 0;
			if (sm && sm.IsPlayerDead(playerId))
				row.m_iDeaths = 1;

			row.m_iKills = TBD_MatchTelemetryTally.GetKills(playerId);

			outRows.Insert(row);
		}
	}
}
