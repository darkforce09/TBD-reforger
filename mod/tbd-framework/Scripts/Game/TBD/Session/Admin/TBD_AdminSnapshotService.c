/**
 * @file TBD_AdminSnapshotService.c
 * @brief Builds the admin snapshot on the server and moves it over the wire as one string.
 *
 * Role: gathers what one admin may see into a TBD_AdminPayload, serialises it, and parses it back.
 * Position: the admin RPCs on SCR_PlayerController call BuildForAdmin and Serialise on the server
 * and Parse on the owning client; it reads TBD_MissionLoader, TBD_FrameworkManager,
 * TBD_MissionValidator, TBD_SpawnManager and TBD_AdminAudit, and encodes with TBD_WireCodec.
 * State: none; pure functions.  Invariants: a non-admin's payload carries the refusal and no data;
 * the wire holds at most MAX_PAYLOAD_LINES records, and a clip logs a warning; every field is
 * marked with TBD_WireCodec.FIELD_MARK so no field is empty; authored text and names are sanitised
 * when the payload is built, so the encoder writes fields as given; an empty or unparseable wire
 * parses as not authorised.
 */

//! Server-side builder and wire codec of the admin snapshot.
class TBD_AdminSnapshotService
{
	protected static const int MAX_PAYLOAD_LINES = 400; //!< records per payload, as in TBD_BriefingWire; a full 128-slot server with findings and audit lines needs about 173
	protected static const int AUDIT_LINES = 20; //!< newest audit entries shipped to the screen
	protected static const string CLIP_WARNING = "snapshot clipped at %1 lines -- raise MAX_PAYLOAD_LINES"; //!< clip warning text; %1 = MAX_PAYLOAD_LINES

	//! Build the snapshot this player is entitled to. A non-admin gets the refusal only, and the
	//! refusal is noted.
	//! @param playerId the requesting player
	//! @return the payload, never null
	//! @authority server
	static TBD_AdminPayload BuildForAdmin(int playerId)
	{
		TBD_AdminPayload payload = new TBD_AdminPayload();

		if (!TBD_AdminService.IsAdmin(playerId))
		{
			payload.m_bAuthorised = false;
			payload.m_sDeniedReason = "You are not a listed server admin.";
			TBD_AdminService.NoteDeniedAccess(playerId, "admin-menu read");
			return payload;
		}

		payload.m_bAuthorised = true;

		BuildMission(payload);
		BuildStage(payload);
		BuildValidation(payload);
		BuildPlayers(payload);
		BuildAudit(payload);

		return payload;
	}

	//! Fill the mission name and terrain and whether the loaded mission is valid.
	//! @authority server
	protected static void BuildMission(TBD_AdminPayload payload)
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc)
			return;

		payload.m_bMissionLoaded = TBD_MissionLoader.IsValid();

		if (!doc.meta)
			return;

		payload.m_sMissionName = TBD_WireCodec.Sanitise(doc.meta.name);
		payload.m_sTerrain = TBD_WireCodec.Sanitise(doc.meta.terrain);
	}

	//! Fill the current stage and the one a force-advance would land on; an empty next stage tells
	//! the screen there is nothing to offer. `NOT READY` when the framework is missing.
	//! @authority server
	protected static void BuildStage(TBD_AdminPayload payload)
	{
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
		{
			// Distinct from DEBRIEF: "nothing left to advance to" and "the stage machine is not up
			// yet" are different problems.
			payload.m_sStage = "NOT READY";
			return;
		}

		payload.m_bStageReady = true;

		TBD_EGameStage stage = framework.GetStage();
		payload.m_sStage = typename.EnumToString(TBD_EGameStage, stage);

		if (stage >= TBD_EGameStage.DEBRIEF)
			return;

		int next = stage;
		next = next + 1;
		payload.m_sNextStage = typename.EnumToString(TBD_EGameStage, next);
	}

	//! Fill the validator verdict and its report lines. A mission the validator rejected never leaves
	//! LOADING, and this is where an admin sees why.
	//! @authority server
	protected static void BuildValidation(TBD_AdminPayload payload)
	{
		payload.m_bValidationRun = TBD_MissionValidator.HasRun();
		payload.m_bValidationPassed = TBD_MissionValidator.Passed();
		payload.m_iValidationErrors = TBD_MissionValidator.GetErrorCount();
		payload.m_iValidationWarnings = TBD_MissionValidator.GetWarningCount();

		if (!payload.m_bValidationRun)
			return;

		array<string> report = TBD_MissionValidator.BuildReportLines();
		if (!report)
			return;

		foreach (string line : report)
		{
			payload.m_aValidationLines.Insert(TBD_WireCodec.Sanitise(line));
		}
	}

	//! Fill one row per connected player: seat, one-life state, body, admin flag. m_bInWorld is read
	//! from the controlled entity rather than inferred from m_bDead, because a dead player needs
	//! Respawn and a live player without a body needs Deploy.
	//! @authority server
	protected static void BuildPlayers(TBD_AdminPayload payload)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> ids = {};
		players.GetPlayers(ids);

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();

		foreach (int id : ids)
		{
			TBD_AdminPlayerRow row = new TBD_AdminPlayerRow();
			row.m_iPlayerId = id;
			row.m_sName = TBD_WireCodec.Sanitise(players.GetPlayerName(id));
			if (row.m_sName.IsEmpty())
				row.m_sName = string.Format("player %1", id);

			row.m_bIsAdmin = TBD_AdminService.IsAdmin(id);
			row.m_bInWorld = players.GetPlayerControlledEntity(id) != null;

			if (spawn)
			{
				row.m_bDead = spawn.IsPlayerDead(id);

				TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(id);
				if (slot)
				{
					row.m_bHasSlot = true;
					row.m_sFaction = TBD_WireCodec.Sanitise(slot.faction);
					row.m_sGroup = TBD_WireCodec.Sanitise(slot.groupCallsign);
					row.m_sRole = TBD_WireCodec.Sanitise(slot.role);
				}
			}

			payload.m_aPlayers.Insert(row);
			payload.m_iConnected++;

			if (row.m_bDead)
				payload.m_iSpent++;
		}
	}

	//! Fill the newest AUDIT_LINES audit entries, newest first.
	//! @authority server
	protected static void BuildAudit(TBD_AdminPayload payload)
	{
		array<ref TBD_AdminAuditEntry> entries = TBD_AdminAudit.GetEntries();
		payload.m_iAuditTotal = entries.Count();

		int taken = 0;
		for (int i = entries.Count() - 1; i >= 0 && taken < AUDIT_LINES; i--)
		{
			TBD_AdminAuditEntry entry = entries[i];
			if (!entry)
				continue;

			payload.m_aAudit.Insert(new TBD_AdminAuditRow(entry.m_sTime, TBD_WireCodec.Sanitise(entry.m_sText), entry.m_bDenied));
			taken++;
		}
	}

	//! Flatten a payload to one string. Field 0 is the record kind; every field after it carries
	//! TBD_WireCodec.FIELD_MARK. Record types:
	//!   `A` authorised (0/1) / denial reason  -- when 0 this is the ONLY record present
	//!   `M` mission   loaded / name / terrain
	//!   `S` stage     current / next ("" = last stage) / stage machine ready
	//!   `V` validate  hasRun / passed / errors / warnings
	//!   `F` finding   one validator report line
	//!   `C` counts    connected / spent / auditTotal
	//!   `P` player    id / name / faction / group / role / hasSlot / dead / inWorld / isAdmin
	//!   `L` audit     time / text / denied
	//! @param payload the snapshot; null gives the empty string
	//! @return the wire string
	static string Serialise(TBD_AdminPayload payload)
	{
		if (!payload)
			return string.Empty;

		array<string> lines = {};

		lines.Insert(TBD_WireCodec.Record2("A", TBD_WireCodec.Flag(payload.m_bAuthorised), payload.m_sDeniedReason, false));

		if (!payload.m_bAuthorised)
			return TBD_WireCodec.Join(lines, MAX_PAYLOAD_LINES, TBD_AdminAudit.CH_ADMIN, CLIP_WARNING);

		lines.Insert(TBD_WireCodec.Record3("M", TBD_WireCodec.Flag(payload.m_bMissionLoaded), payload.m_sMissionName, payload.m_sTerrain, false));
		lines.Insert(TBD_WireCodec.Record3("S", payload.m_sStage, payload.m_sNextStage, TBD_WireCodec.Flag(payload.m_bStageReady), false));
		lines.Insert(TBD_WireCodec.Record4("V", TBD_WireCodec.Flag(payload.m_bValidationRun), TBD_WireCodec.Flag(payload.m_bValidationPassed),
			payload.m_iValidationErrors.ToString(), payload.m_iValidationWarnings.ToString(), false));
		lines.Insert(TBD_WireCodec.Record3("C", payload.m_iConnected.ToString(), payload.m_iSpent.ToString(),
			payload.m_iAuditTotal.ToString(), false));

		foreach (string finding : payload.m_aValidationLines)
		{
			lines.Insert(TBD_WireCodec.Record1("F", finding, false));
		}

		foreach (TBD_AdminPlayerRow row : payload.m_aPlayers)
		{
			lines.Insert(RecordPlayer(row));
		}

		foreach (TBD_AdminAuditRow audit : payload.m_aAudit)
		{
			lines.Insert(TBD_WireCodec.Record3("L", audit.m_sTime, audit.m_sText, TBD_WireCodec.Flag(audit.m_bDenied), false));
		}

		return TBD_WireCodec.Join(lines, MAX_PAYLOAD_LINES, TBD_AdminAudit.CH_ADMIN, CLIP_WARNING);
	}

	//! Rebuild a payload on the client. A malformed line is skipped, not fatal, so the panel renders
	//! what arrived.
	//! @param wire the string Serialise produced
	//! @return the payload; an empty or unparseable wire is not authorised
	//! @authority owner
	static TBD_AdminPayload Parse(string wire)
	{
		TBD_AdminPayload payload = new TBD_AdminPayload();

		if (wire.IsEmpty())
		{
			payload.m_sDeniedReason = "No answer from the server.";
			return payload;
		}

		array<string> lines = {};
		wire.Split(TBD_WireCodec.LINE_SEP, lines, false);

		foreach (string line : lines)
		{
			array<string> f = {};
			line.Split(TBD_WireCodec.FIELD_SEP, f, false);
			if (f.IsEmpty())
				continue;

			string kind = f[0];

			if (kind == "A" && f.Count() >= 3)
			{
				payload.m_bAuthorised = TBD_WireCodec.IsSet(f[1]);
				payload.m_sDeniedReason = TBD_WireCodec.Unmark(f[2]);
			}
			else if (kind == "M" && f.Count() >= 4)
			{
				payload.m_bMissionLoaded = TBD_WireCodec.IsSet(f[1]);
				payload.m_sMissionName = TBD_WireCodec.Unmark(f[2]);
				payload.m_sTerrain = TBD_WireCodec.Unmark(f[3]);
			}
			else if (kind == "S" && f.Count() >= 4)
			{
				payload.m_sStage = TBD_WireCodec.Unmark(f[1]);
				payload.m_sNextStage = TBD_WireCodec.Unmark(f[2]);
				payload.m_bStageReady = TBD_WireCodec.IsSet(f[3]);
			}
			else if (kind == "V" && f.Count() >= 5)
			{
				payload.m_bValidationRun = TBD_WireCodec.IsSet(f[1]);
				payload.m_bValidationPassed = TBD_WireCodec.IsSet(f[2]);
				payload.m_iValidationErrors = TBD_WireCodec.Unmark(f[3]).ToInt();
				payload.m_iValidationWarnings = TBD_WireCodec.Unmark(f[4]).ToInt();
			}
			else if (kind == "C" && f.Count() >= 4)
			{
				payload.m_iConnected = TBD_WireCodec.Unmark(f[1]).ToInt();
				payload.m_iSpent = TBD_WireCodec.Unmark(f[2]).ToInt();
				payload.m_iAuditTotal = TBD_WireCodec.Unmark(f[3]).ToInt();
			}
			else if (kind == "F" && f.Count() >= 2)
			{
				payload.m_aValidationLines.Insert(TBD_WireCodec.Unmark(f[1]));
			}
			else if (kind == "P" && f.Count() >= 10)
			{
				TBD_AdminPlayerRow row = new TBD_AdminPlayerRow();
				row.m_iPlayerId = TBD_WireCodec.Unmark(f[1]).ToInt();
				row.m_sName = TBD_WireCodec.Unmark(f[2]);
				row.m_sFaction = TBD_WireCodec.Unmark(f[3]);
				row.m_sGroup = TBD_WireCodec.Unmark(f[4]);
				row.m_sRole = TBD_WireCodec.Unmark(f[5]);
				row.m_bHasSlot = TBD_WireCodec.IsSet(f[6]);
				row.m_bDead = TBD_WireCodec.IsSet(f[7]);
				row.m_bInWorld = TBD_WireCodec.IsSet(f[8]);
				row.m_bIsAdmin = TBD_WireCodec.IsSet(f[9]);
				payload.m_aPlayers.Insert(row);
			}
			else if (kind == "L" && f.Count() >= 4)
			{
				payload.m_aAudit.Insert(new TBD_AdminAuditRow(TBD_WireCodec.Unmark(f[1]), TBD_WireCodec.Unmark(f[2]), TBD_WireCodec.IsSet(f[3])));
			}
		}

		return payload;
	}

	//! The `P` record of one player. Appended in steps: a nine-field `+` chain trips the engine's
	//! `Formula too complex` ceiling.
	//! @param row the player row, already sanitised
	//! @return the record
	protected static string RecordPlayer(TBD_AdminPlayerRow row)
	{
		string line = "P";
		line = line + TBD_WireCodec.Field(row.m_iPlayerId.ToString(), false);
		line = line + TBD_WireCodec.Field(row.m_sName, false);
		line = line + TBD_WireCodec.Field(row.m_sFaction, false);
		line = line + TBD_WireCodec.Field(row.m_sGroup, false);
		line = line + TBD_WireCodec.Field(row.m_sRole, false);
		line = line + TBD_WireCodec.Field(TBD_WireCodec.Flag(row.m_bHasSlot), false);
		line = line + TBD_WireCodec.Field(TBD_WireCodec.Flag(row.m_bDead), false);
		line = line + TBD_WireCodec.Field(TBD_WireCodec.Flag(row.m_bInWorld), false);
		line = line + TBD_WireCodec.Field(TBD_WireCodec.Flag(row.m_bIsAdmin), false);
		return line;
	}
}
