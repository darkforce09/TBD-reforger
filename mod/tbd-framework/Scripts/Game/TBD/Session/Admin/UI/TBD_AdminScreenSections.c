/**
 * @file TBD_AdminScreenSections.c
 * @brief The four list sections of the admin screen: MISSION, STAGE, PLAYERS and ADMIN ACTIONS.
 *
 * Role: writes the admin screen's list rows from a snapshot and the screen's disclosure and arming
 * flags, and owns the row-tag scheme.  Position: TBD_AdminScreen.Rebuild calls the Emit methods in
 * order and decodes picks with the TAG constants; data comes from TBD_AdminPayload.
 * State: none; static functions.  Invariants: player rows carry TAG_PLAYER_BASE + playerId, so a
 * tag decodes to a player without a side table a rebuild could desynchronise; inert rows carry
 * TAG_INERT; every colour is a TBD_EUIState, never a literal; the ASCII `->`, `+` and `-` stand in
 * for glyphs the UI font may lack.
 */

//! Row emitters and row tags of the admin screen list.
class TBD_AdminScreenSections
{
	static const int TAG_INERT = -1; //!< tag of a row a pick does nothing with
	static const int TAG_VALIDATION = 1; //!< tag of the validation verdict row
	static const int TAG_STAGE = 2; //!< tag of the force-stage row
	static const int TAG_AUDIT = 3; //!< tag of the audit-trail disclosure row
	static const int TAG_PLAYER_BASE = 1000; //!< player rows carry this plus the player id

	//! The MISSION section: name and terrain, then the validator verdict, with the findings under it
	//! when disclosed. A mission the validator rejected never leaves LOADING, and this row says why.
	//! @param list the list being rebuilt
	//! @param payload an authorised snapshot
	//! @param validationExpanded the findings are disclosed
	static void EmitMission(TBD_ListBox list, notnull TBD_AdminPayload payload, bool validationExpanded)
	{
		string name = payload.m_sMissionName;
		if (name.IsEmpty())
			name = "none loaded";
		else if (!payload.m_sTerrain.IsEmpty())
			name = string.Format("%1 - %2", name, payload.m_sTerrain);

		list.AddSection("MISSION", name);

		if (!payload.m_bValidationRun)
		{
			list.AddItem("    Validation", "not run yet", TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		string verdict = string.Format("PASSED -- %1 warning(s)", payload.m_iValidationWarnings);
		TBD_EUIState state = TBD_EUIState.NORMAL;

		if (!payload.m_bValidationPassed)
		{
			verdict = string.Format("FAILED -- %1 error(s), %2 warning(s)",
				payload.m_iValidationErrors, payload.m_iValidationWarnings);
			state = TBD_EUIState.DANGER;
		}

		bool hasFindings = !payload.m_aValidationLines.IsEmpty();
		if (hasFindings)
			verdict = string.Format("%1  %2", verdict, DisclosureMark(validationExpanded));

		list.AddItem("    Validation", verdict, TAG_VALIDATION, state, hasFindings);

		if (!hasFindings || !validationExpanded)
			return;

		foreach (string finding : payload.m_aValidationLines)
		{
			list.AddItem("        " + finding, string.Empty, TAG_INERT, state, false);
		}
	}

	//! The STAGE section: the current stage and the force-advance row, armed by one pick and
	//! confirmed by a second because it moves the round for everybody and cannot be undone.
	//! @param list the list being rebuilt
	//! @param payload an authorised snapshot
	//! @param stageArmed the first pick has armed the force
	static void EmitStage(TBD_ListBox list, notnull TBD_AdminPayload payload, bool stageArmed)
	{
		list.AddSection("STAGE", payload.m_sStage);

		if (!payload.m_bStageReady)
		{
			list.AddItem("    Force stage", "the stage machine is not up yet", TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		if (payload.m_sNextStage.IsEmpty())
		{
			list.AddItem("    Force stage", "already at the last stage", TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		// ASCII arrow: a glyph the UI font lacks would draw as a box on the one control that moves
		// the whole round.
		string label = string.Format("    Force stage -> %1", payload.m_sNextStage);

		if (stageArmed)
		{
			list.AddItem(label, "ARMED -- pick again to move the whole round", TAG_STAGE, TBD_EUIState.DANGER, true);
			return;
		}

		list.AddItem(label, "irreversible - pick twice", TAG_STAGE, TBD_EUIState.NORMAL, true);
	}

	//! The PLAYERS section: one pickable row per connected player, the rows the primary action acts on.
	//! @param list the list being rebuilt
	//! @param payload an authorised snapshot
	static void EmitPlayers(TBD_ListBox list, notnull TBD_AdminPayload payload)
	{
		list.AddSection("PLAYERS", string.Format("%1 connected - %2 lives spent",
			payload.m_iConnected, payload.m_iSpent));

		if (payload.m_aPlayers.IsEmpty())
		{
			list.AddItem("    Nobody is connected.", string.Empty, TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		foreach (TBD_AdminPlayerRow row : payload.m_aPlayers)
		{
			list.AddItem("    " + row.m_sName, DescribePlayer(row), PlayerTag(row.m_iPlayerId),
				PlayerState(row), true);
		}
	}

	//! A player row's detail: admin tag, seat, then the state an admin acts on.
	//! @return e.g. `ADMIN - us_army - ALPHA/RFL - LIFE SPENT`
	protected static string DescribePlayer(TBD_AdminPlayerRow row)
	{
		string detail;

		if (row.m_bIsAdmin)
			detail = "ADMIN";

		if (row.m_bHasSlot)
		{
			string seat = string.Format("%1 - %2/%3", row.m_sFaction, row.m_sGroup, row.m_sRole);
			if (detail.IsEmpty())
				detail = seat;
			else
				detail = detail + " - " + seat;
		}
		else
		{
			if (detail.IsEmpty())
				detail = "no slot";
			else
				detail = detail + " - no slot";
		}

		string status = "in world";
		if (row.m_bDead)
			status = "LIFE SPENT";
		else if (!row.m_bInWorld)
			status = "NO BODY";

		return detail + " - " + status;
	}

	//! A player row's colour: DANGER for a spent life, TAKEN for no body, NORMAL otherwise.
	protected static TBD_EUIState PlayerState(TBD_AdminPlayerRow row)
	{
		if (row.m_bDead)
			return TBD_EUIState.DANGER;

		if (!row.m_bInWorld)
			return TBD_EUIState.TAKEN;

		return TBD_EUIState.NORMAL;
	}

	//! The ADMIN ACTIONS section: the audit count, a disclosure row, and the newest entries under it
	//! when disclosed, refusals in DANGER.
	//! @param list the list being rebuilt
	//! @param payload an authorised snapshot
	//! @param auditExpanded the trail is disclosed
	static void EmitAudit(TBD_ListBox list, notnull TBD_AdminPayload payload, bool auditExpanded)
	{
		list.AddSection("ADMIN ACTIONS", string.Format("%1 this session", payload.m_iAuditTotal));

		if (payload.m_aAudit.IsEmpty())
		{
			list.AddItem("    No admin actions yet.", string.Empty, TAG_INERT, TBD_EUIState.NORMAL, false);
			return;
		}

		list.AddItem("    Show the audit trail", DisclosureMark(auditExpanded), TAG_AUDIT,
			TBD_EUIState.NORMAL, true);

		if (!auditExpanded)
			return;

		foreach (TBD_AdminAuditRow audit : payload.m_aAudit)
		{
			TBD_EUIState state = TBD_EUIState.NORMAL;
			if (audit.m_bDenied)
				state = TBD_EUIState.DANGER;

			list.AddItem("        " + audit.m_sTime, audit.m_sText, TAG_INERT, state, false);
		}
	}

	//! The row tag of a player.
	//! @param playerId the player
	//! @return TAG_PLAYER_BASE + playerId, or -1 for a non-positive id
	static int PlayerTag(int playerId)
	{
		if (playerId <= 0)
			return -1;

		return TAG_PLAYER_BASE + playerId;
	}

	//! The disclosure mark of a row with more behind it, plain ASCII so the UI font has it.
	//! @return `-` when expanded, `+` when collapsed
	protected static string DisclosureMark(bool expanded)
	{
		if (expanded)
			return "-";

		return "+";
	}
}
