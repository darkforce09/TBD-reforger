//! T-181.11.2 — what the admin screen is allowed to know, and how it gets there.
//! UI reorg 2026-09-12: models only. Server snapshot builder + wire live in `TBD_AdminSnapshotService.c`.
//!
//! ── The fact this design is built on ────────────────────────────────────────────────────────
//! **A client holds no mission document and no slot assignment.** `TBD_FrameworkManager.OnPostInit`
//! returns early for `RplMode.Client` before `TBD_MissionLoader.BeginLoad()`, and
//! `TBD_SpawnManager`'s `m_mPlayerSlot` is a plain map, not an `RplProp`. So the admin screen
//! cannot read who is alive, who holds which seat, or whether the mission validated — it can only
//! render what the server chose to send it. Same constraint the briefing screen lives under
//! (`TBD_BriefingData.c`), same answer: build on the server, ship one string, rebuild on the client.
//!
//! ── Why that is the right shape for an ADMIN screen specifically ────────────────────────────
//! Because it makes the permission check structural rather than cosmetic. There is no
//! locally-cached roster a non-admin client could render if it patched out a widget check: the
//! snapshot for a non-admin contains a refusal string and **nothing else** — no mission, no player
//! list, no audit trail. The bytes never leave the server.
//!
//! `BuildForAdmin` is therefore the read-side twin of `TBD_AdminService.Execute`: one gate for
//! doing, one gate for seeing, both resolved from `SCR_PlayerListedAdminManagerComponent` on the
//! authority.

//! One connected player, as the admin needs to see them.
class TBD_AdminPlayerRow
{
	int m_iPlayerId;
	string m_sName;
	string m_sFaction;  //!< mission faction key of their claimed seat, empty when unslotted
	string m_sGroup;
	string m_sRole;
	bool m_bHasSlot;
	bool m_bDead;       //!< ONE LIFE: they have spent it. The only state Respawn can recover.
	bool m_bInWorld;    //!< they currently control an entity — i.e. they actually have a body
	bool m_bIsAdmin;    //!< on the server admin list themselves
}

//! One line of the audit trail, as shipped to the screen.
class TBD_AdminAuditRow
{
	string m_sTime;
	string m_sText;
	bool m_bDenied;

	//------------------------------------------------------------------------------------------------
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
	//! False = the server refused. Everything below is then empty, by construction.
	bool m_bAuthorised;
	string m_sDeniedReason;

	// ── Mission ───────────────────────────────────────────────────────────────────────────────
	bool m_bMissionLoaded;
	string m_sMissionName;
	string m_sTerrain;

	// ── Stage ─────────────────────────────────────────────────────────────────────────────────
	bool m_bStageReady;  //!< the framework answered at all; false = no game mode component yet
	string m_sStage;
	string m_sNextStage; //!< empty = the round is already at the last stage, nothing to force

	// ── Validation (T-181.14) ─────────────────────────────────────────────────────────────────
	bool m_bValidationRun;
	bool m_bValidationPassed;
	int m_iValidationErrors;
	int m_iValidationWarnings;
	ref array<string> m_aValidationLines;

	// ── People ────────────────────────────────────────────────────────────────────────────────
	int m_iConnected;
	int m_iSpent;
	ref array<ref TBD_AdminPlayerRow> m_aPlayers;

	// ── Audit, newest first ───────────────────────────────────────────────────────────────────
	int m_iAuditTotal;
	ref array<ref TBD_AdminAuditRow> m_aAudit;

	//------------------------------------------------------------------------------------------------
	void TBD_AdminPayload()
	{
		m_aValidationLines = {};
		m_aPlayers = {};
		m_aAudit = {};
	}

	//------------------------------------------------------------------------------------------------
	//! The row for a player id, or null. The screen resolves its selection through this so a
	//! refresh that reorders or drops rows can never leave a stale action pointed at the wrong
	//! person — the id is the identity, the row index is not.
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
