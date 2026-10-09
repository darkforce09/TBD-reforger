/**
 * @file TBD_AdminPayload.c
 * @brief The admin snapshot: everything one admin may see, with its player and audit rows.
 *
 * Role: models the snapshot the admin screen renders.  Position: TBD_AdminSnapshotService builds it
 * on the server and serialises it; the owning client parses it back and TBD_AdminClient hands it to
 * TBD_AdminScreen. A client holds no mission document and no slot map, so this is the screen's only
 * source.
 * State: none beyond the fields of one snapshot.  Invariants: a payload with m_bAuthorised false
 * carries the refusal reason and nothing else; the screen resolves its selection by player id
 * through FindPlayer, never by row index.
 */

//! One connected player, as the admin needs to see them.
class TBD_AdminPlayerRow
{
	int m_iPlayerId; //!< the player id
	string m_sName; //!< player name; `player <id>` when the engine has none
	string m_sFaction; //!< mission faction key of their claimed seat, empty when unslotted
	string m_sGroup; //!< group callsign of their seat, empty when unslotted
	string m_sRole; //!< role of their seat, empty when unslotted
	bool m_bHasSlot; //!< they hold a seat
	bool m_bDead; //!< they have spent their one life; the state Respawn recovers
	bool m_bInWorld; //!< they control an entity, so they have a body
	bool m_bIsAdmin; //!< they are on the server admin list
}

//! One line of the audit trail, as shipped to the screen.
class TBD_AdminAuditRow
{
	string m_sTime; //!< server-local HH:MM:SS of the attempt
	string m_sText; //!< the audit line
	bool m_bDenied; //!< the attempt was refused

	//! Build a row from its three fields.
	void TBD_AdminAuditRow(string time, string text, bool denied)
	{
		m_sTime = time;
		m_sText = text;
		m_bDenied = denied;
	}
}

//! Everything one admin is permitted to see. Built on the server, shipped as one string.
class TBD_AdminPayload
{
	bool m_bAuthorised; //!< false = the server refused; every field below is then empty
	string m_sDeniedReason; //!< why the server refused, empty when authorised

	bool m_bMissionLoaded; //!< a mission document is loaded and valid
	string m_sMissionName; //!< mission name, empty when none is loaded
	string m_sTerrain; //!< mission terrain key, empty when none is loaded

	bool m_bStageReady; //!< the framework answered; false = no game mode component yet
	string m_sStage; //!< current stage name, or `NOT READY`
	string m_sNextStage; //!< stage a force-advance lands on; empty at the last stage

	bool m_bValidationRun; //!< the validator has run on the loaded mission
	bool m_bValidationPassed; //!< the validator passed it
	int m_iValidationErrors; //!< validator error count
	int m_iValidationWarnings; //!< validator warning count
	ref array<string> m_aValidationLines; //!< validator report lines, errors first

	int m_iConnected; //!< connected players
	int m_iSpent; //!< connected players whose life is spent
	ref array<ref TBD_AdminPlayerRow> m_aPlayers; //!< one row per connected player

	int m_iAuditTotal; //!< audit entries held on the server
	ref array<ref TBD_AdminAuditRow> m_aAudit; //!< newest audit entries, newest first

	//! An empty, unauthorised payload with empty lists.
	void TBD_AdminPayload()
	{
		m_aValidationLines = {};
		m_aPlayers = {};
		m_aAudit = {};
	}

	//! The row for a player id. The screen resolves its selection through this, so a refresh that
	//! reorders or drops rows cannot aim an action at the wrong person.
	//! @param playerId the player to find
	//! @return the row, or null when that player is not in the snapshot
	TBD_AdminPlayerRow FindPlayer(int playerId)
	{
		foreach (TBD_AdminPlayerRow row : m_aPlayers)
		{
			if (row && row.m_iPlayerId == playerId)
				return row;
		}

		return null;
	}
}
