//! Admin feature module - SERVER snapshot builder + wire Serialise/Parse. Split out of TBD_AdminData.c (UI reorg 2026-09-12); logic unchanged.
//!
//! Builds the admin snapshot on the server, moves it over one RPC, rebuilds it on the client.
//!
//! Wire format is line-based with tab-separated fields, matching the precedent already in the tree
//! (`TBD_MissionBrowserService`, `TBD_BriefingService`): one string per RPC, no schema to register,
//! greppable in a log when something goes wrong.
class TBD_AdminSnapshotService
{
	//! Defensive cap on one payload, matching `TBD_BriefingService.MAX_PAYLOAD_LINES`. A full server
	//! with a long audit trail must not become an unbounded reliable-channel string.
	//!
	//! Headroom check: 5 header records + ~20 validator findings + one line per connected player +
	//! 20 audit lines. A 128-slot event lands around 173, so the cap is slack, not a squeeze — and
	//! `Join` logs loudly if it is ever hit rather than truncating in silence.
	protected static const int MAX_PAYLOAD_LINES = 400;

	//! Newest audit entries shipped to the screen. The server console holds the rest.
	protected static const int AUDIT_LINES = 20;

	protected static const string FIELD_SEP = "\t";
	protected static const string LINE_SEP = "\n";

	//! Every field is written with this leading marker, so **no field is ever the empty string**.
	//!
	//! ── Why, and why it is not paranoia ────────────────────────────────────────────────────
	//! `string.Split(sep, out, trim)` is a NATIVE engine call. Whether it emits a token for an
	//! empty field between two separators is a RUNTIME property, and nothing in this lane can
	//! prove a runtime property — a compile probe answers "does this symbol exist", not "what does
	//! it do" (SLICE_WORKFLOW.md §What agents cannot do). If it drops empties, then a record like
	//! `P <id> <name> <faction> <group> <role> …` silently shifts every field left the moment a
	//! player has no slot, and an unslotted player would render with somebody else's data in the
	//! faction column. That is the exact class of bug an admin panel must not have.
	//!
	//! Marking every field removes the question instead of answering it: a marked empty value is
	//! the one-character string `.`, which no tokeniser can drop and no trim can erase. The cost is
	//! one byte per field and a wire that reads `P<TAB>.7<TAB>.Vasquez<TAB>.us_army…`.
	protected static const string FIELD_MARK = ".";

	// ── SERVER ──────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Build the snapshot this player is entitled to.
	//!
	//! @authority server — reads `TBD_SpawnManager`, `TBD_MissionLoader` and `TBD_MissionValidator`,
	//! none of which hold anything on a client.
	static TBD_AdminPayload BuildForAdmin(int playerId)
	{
		TBD_AdminPayload payload = new TBD_AdminPayload();

		// ── THE READ GATE. Fail closed: a refusal carries no data at all. ──
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

	//------------------------------------------------------------------------------------------------
	protected static void BuildMission(TBD_AdminPayload payload)
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc)
			return;

		payload.m_bMissionLoaded = TBD_MissionLoader.IsValid();

		if (!doc.meta)
			return;

		payload.m_sMissionName = Sanitise(doc.meta.name);
		payload.m_sTerrain = Sanitise(doc.meta.terrain);
	}

	//------------------------------------------------------------------------------------------------
	//! Current stage, and the one a force-advance would land on. `m_sNextStage` empty is what tells
	//! the screen there is nothing to offer — the screen never computes the transition itself.
	protected static void BuildStage(TBD_AdminPayload payload)
	{
		TBD_FrameworkManager framework = TBD_FrameworkManager.GetInstance();
		if (!framework)
		{
			// Distinguished from DEBRIEF on purpose: "there is nothing left to advance to" and
			// "the stage machine is not up yet" are different problems and want different words.
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

	//------------------------------------------------------------------------------------------------
	//! T-181.14 — a mission the validator rejected is otherwise INVISIBLE from in-game: the stage
	//! machine simply never leaves LOADING and nothing on screen says why. An admin panel is
	//! exactly where that has to surface.
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
			payload.m_aValidationLines.Insert(Sanitise(line));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Who is connected, whose seat is whose, and who has spent their life.
	//!
	//! `m_bInWorld` is asked of the player controller rather than inferred from `m_bDead`, because
	//! the two failures this screen exists to fix are DIFFERENT: a dead player needs Respawn, and a
	//! player who is alive but has no body needs Deploy. Collapsing them into one flag would hide
	//! the second failure entirely.
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
			row.m_sName = Sanitise(players.GetPlayerName(id));
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
					row.m_sFaction = Sanitise(slot.faction);
					row.m_sGroup = Sanitise(slot.groupCallsign);
					row.m_sRole = Sanitise(slot.role);
				}
			}

			payload.m_aPlayers.Insert(row);
			payload.m_iConnected++;

			if (row.m_bDead)
				payload.m_iSpent++;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Newest first — an admin opening the screen after something went wrong wants the last action,
	//! not the first one of the session.
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

			payload.m_aAudit.Insert(new TBD_AdminAuditRow(entry.m_sTime, Sanitise(entry.m_sText), entry.m_bDenied));
			taken++;
		}
	}

	// ── WIRE ────────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! Flatten a payload to one string. Field 0 is the record kind; every field after it carries
	//! `FIELD_MARK` (see the note on that constant). Record types:
	//!   `A` authorised (0/1) / denial reason  — when 0 this is the ONLY record present
	//!   `M` mission   loaded / name / terrain
	//!   `S` stage     current / next ("" = last stage) / stage machine ready
	//!   `V` validate  hasRun / passed / errors / warnings
	//!   `F` finding   one validator report line
	//!   `C` counts    connected / spent / auditTotal
	//!   `P` player    id / name / faction / group / role / hasSlot / dead / inWorld / isAdmin
	//!   `L` audit     time / text / denied
	static string Serialise(TBD_AdminPayload payload)
	{
		if (!payload)
			return string.Empty;

		array<string> lines = {};

		lines.Insert(Record2("A", Flag(payload.m_bAuthorised), payload.m_sDeniedReason));

		if (!payload.m_bAuthorised)
			return Join(lines);

		lines.Insert(Record3("M", Flag(payload.m_bMissionLoaded), payload.m_sMissionName, payload.m_sTerrain));
		lines.Insert(Record3("S", payload.m_sStage, payload.m_sNextStage, Flag(payload.m_bStageReady)));
		lines.Insert(Record4("V", Flag(payload.m_bValidationRun), Flag(payload.m_bValidationPassed),
			payload.m_iValidationErrors.ToString(), payload.m_iValidationWarnings.ToString()));
		lines.Insert(Record3("C", payload.m_iConnected.ToString(), payload.m_iSpent.ToString(),
			payload.m_iAuditTotal.ToString()));

		foreach (string finding : payload.m_aValidationLines)
		{
			lines.Insert(Record1("F", finding));
		}

		foreach (TBD_AdminPlayerRow row : payload.m_aPlayers)
		{
			lines.Insert(RecordPlayer(row));
		}

		foreach (TBD_AdminAuditRow audit : payload.m_aAudit)
		{
			lines.Insert(Record3("L", audit.m_sTime, audit.m_sText, Flag(audit.m_bDenied)));
		}

		return Join(lines);
	}

	//------------------------------------------------------------------------------------------------
	//! Rebuild a payload on the client. A malformed line is skipped rather than fatal — a panel
	//! that renders most of itself beats a blank screen (design law: nothing blocking).
	//!
	//! Note the default: a payload that arrives empty or unparseable is **not authorised**. The
	//! client's failure mode is "show nothing", never "assume the server said yes".
	static TBD_AdminPayload Parse(string wire)
	{
		TBD_AdminPayload payload = new TBD_AdminPayload();

		if (wire.IsEmpty())
		{
			payload.m_sDeniedReason = "No answer from the server.";
			return payload;
		}

		array<string> lines = {};
		wire.Split(LINE_SEP, lines, false);

		foreach (string line : lines)
		{
			array<string> f = {};
			line.Split(FIELD_SEP, f, false);
			if (f.IsEmpty())
				continue;

			string kind = f[0];

			if (kind == "A" && f.Count() >= 3)
			{
				payload.m_bAuthorised = IsSet(f[1]);
				payload.m_sDeniedReason = Unmark(f[2]);
			}
			else if (kind == "M" && f.Count() >= 4)
			{
				payload.m_bMissionLoaded = IsSet(f[1]);
				payload.m_sMissionName = Unmark(f[2]);
				payload.m_sTerrain = Unmark(f[3]);
			}
			else if (kind == "S" && f.Count() >= 4)
			{
				payload.m_sStage = Unmark(f[1]);
				payload.m_sNextStage = Unmark(f[2]);
				payload.m_bStageReady = IsSet(f[3]);
			}
			else if (kind == "V" && f.Count() >= 5)
			{
				payload.m_bValidationRun = IsSet(f[1]);
				payload.m_bValidationPassed = IsSet(f[2]);
				payload.m_iValidationErrors = Unmark(f[3]).ToInt();
				payload.m_iValidationWarnings = Unmark(f[4]).ToInt();
			}
			else if (kind == "C" && f.Count() >= 4)
			{
				payload.m_iConnected = Unmark(f[1]).ToInt();
				payload.m_iSpent = Unmark(f[2]).ToInt();
				payload.m_iAuditTotal = Unmark(f[3]).ToInt();
			}
			else if (kind == "F" && f.Count() >= 2)
			{
				payload.m_aValidationLines.Insert(Unmark(f[1]));
			}
			else if (kind == "P" && f.Count() >= 10)
			{
				TBD_AdminPlayerRow row = new TBD_AdminPlayerRow();
				row.m_iPlayerId = Unmark(f[1]).ToInt();
				row.m_sName = Unmark(f[2]);
				row.m_sFaction = Unmark(f[3]);
				row.m_sGroup = Unmark(f[4]);
				row.m_sRole = Unmark(f[5]);
				row.m_bHasSlot = IsSet(f[6]);
				row.m_bDead = IsSet(f[7]);
				row.m_bInWorld = IsSet(f[8]);
				row.m_bIsAdmin = IsSet(f[9]);
				payload.m_aPlayers.Insert(row);
			}
			else if (kind == "L" && f.Count() >= 4)
			{
				payload.m_aAudit.Insert(new TBD_AdminAuditRow(Unmark(f[1]), Unmark(f[2]), IsSet(f[3])));
			}
		}

		return payload;
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────────────────

	//------------------------------------------------------------------------------------------------
	//! MEASURED (T-181.11.2): a nine-field record written as one `+` chain fails to compile with
	//! `Formula too complex` — and the *second* diagnostic on the same line is a misleading
	//! `Incompatible parameter 'FIELD_SEP'`, which sends you hunting a type problem that is not
	//! there. Enfusion has an expression-complexity ceiling; the fix is to append in steps.
	protected static string RecordPlayer(TBD_AdminPlayerRow row)
	{
		string line = "P";
		line = line + Field(row.m_iPlayerId.ToString());
		line = line + Field(row.m_sName);
		line = line + Field(row.m_sFaction);
		line = line + Field(row.m_sGroup);
		line = line + Field(row.m_sRole);
		line = line + Field(Flag(row.m_bHasSlot));
		line = line + Field(Flag(row.m_bDead));
		line = line + Field(Flag(row.m_bInWorld));
		line = line + Field(Flag(row.m_bIsAdmin));
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
			TBD_Log.Warn(TBD_AdminAudit.CH_ADMIN,
				string.Format("snapshot clipped at %1 lines — raise MAX_PAYLOAD_LINES", MAX_PAYLOAD_LINES));
		}

		return result;
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record1(string kind, string a)
	{
		return kind + Field(a);
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record2(string kind, string a, string b)
	{
		return kind + Field(a) + Field(b);
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record3(string kind, string a, string b, string c)
	{
		return kind + Field(a) + Field(b) + Field(c);
	}

	//------------------------------------------------------------------------------------------------
	protected static string Record4(string kind, string a, string b, string c, string d)
	{
		return kind + Field(a) + Field(b) + Field(c) + Field(d);
	}

	//------------------------------------------------------------------------------------------------
	//! `<TAB>.<value>` — separator, marker, value. The marker is what guarantees a non-empty token
	//! for an empty value; see FIELD_MARK.
	protected static string Field(string value)
	{
		return FIELD_SEP + FIELD_MARK + value;
	}

	//------------------------------------------------------------------------------------------------
	//! Strip the marker back off a parsed field. A field shorter than the marker is treated as
	//! empty rather than as an error — the parser's job is to render what arrived, not to refuse.
	protected static string Unmark(string field)
	{
		int length = field.Length();
		if (length <= 1)
			return string.Empty;

		return field.Substring(1, length - 1);
	}

	//------------------------------------------------------------------------------------------------
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
	//! Strip the field and line separators out of any text that came from authored data or a player
	//! name, so a name containing a tab cannot shift every field of its record.
	//!
	//! MEASURED: `string.Replace` mutates the receiver IN PLACE and returns the replacement COUNT,
	//! not the new string — `s = s.Replace(a, b)` does not compile.
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
