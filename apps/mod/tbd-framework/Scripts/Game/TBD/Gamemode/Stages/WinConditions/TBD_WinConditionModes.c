/**
 * @file TBD_WinConditionModes.c
 * @brief The per-mode checks of the authored win rule: arm-time warnings, extraction and VIP.
 *
 * Role: says once per world whether the mission can satisfy its `winConditions.mode`, and evaluates
 * the two modes nothing else observes: `extraction` and `vip`.  Position: called by
 * TBD_WinConditionEvaluator.Tick with the parsed rule; reads TBD_SpawnManager, TBD_MissionLoader
 * and TBD_ZoneRegistry.  State: none (the one-time VIP-missing warning is a TBD_AnnounceOnce key).
 * Invariants: a side with no living player never extracts; a player without a body counts as
 * living and outside; an unclaimed VIP seat fires nothing; an ambiguous winner is left empty.
 */

//! Mode vocabulary and per-mode evaluation of the authored win rule.
class TBD_WinConditionModes
{
	static const string MODE_ATTRITION  = "attrition"; //!< ends through TBD_FactionElimination
	static const string MODE_OBJECTIVE  = "objective"; //!< ends through TBD_ObjectiveRegistry's triggers
	static const string MODE_EXTRACTION = "extraction"; //!< evaluated here
	static const string MODE_VIP        = "vip"; //!< evaluated here
	static const string MODE_TIMEOUT    = "timeout"; //!< ends through TBD_RoundClock
	static const string VIP_MISSING_KEY = "Win.vipMissing"; //!< TBD_AnnounceOnce key of the unclaimed-VIP warning

	//! Log the rule and every reason it can never fire (a mission that never ends reads as a server
	//! fault). Called once per world at the first LIVE tick.
	//! @param rule the parsed rule
	//! @authority server
	static void ReportRule(notnull TBD_WinConditionsStruct rule)
	{
		string mode = rule.mode;
		TBD_Log.Kv(TBD_WinConditionEvaluator.CH, "rule", string.Format("mode=%1", mode));

		if (mode == MODE_OBJECTIVE)
		{
			bool anyObjectiveTrigger = false;
			for (int i = 0; i < TBD_ObjectiveKindBehaviour.Count() && !anyObjectiveTrigger; i++)
			{
				TBD_ObjectiveKindBehaviour behaviour = TBD_ObjectiveKindBehaviour.At(i);
				anyObjectiveTrigger = TBD_MissionLoader.HasEndTrigger(behaviour.EndTrigger());
			}

			// The objective triggers in the kind lookup's order CAPTURE, DESTROY, HOLD_UNTIL.
			if (!anyObjectiveTrigger)
			{
				string objectiveTriggers = string.Empty;
				for (int k = 0; k < TBD_ObjectiveKindBehaviour.Count(); k++)
				{
					if (k > 0)
						objectiveTriggers += " / ";

					objectiveTriggers += TBD_ObjectiveKindBehaviour.At(k).EndTrigger();
				}

				TBD_Log.Warn(TBD_WinConditionEvaluator.CH, string.Format("winConditions.mode is 'objective' but endOn declares none of %1 - the objective registry drives those three, so this rule can NEVER end the round.",
					objectiveTriggers));
			}

			return;
		}

		if (mode == MODE_TIMEOUT)
		{
			if (!TBD_MissionLoader.HasEndTrigger(TBD_MissionFlow.TRIGGER_TIME_LIMIT))
				TBD_Log.Warn(TBD_WinConditionEvaluator.CH, "winConditions.mode is 'timeout' but endOn does not declare 'time_limit' - the round clock only arms on that trigger, so this rule can NEVER end the round.");
			else
				TBD_Log.Kv(TBD_WinConditionEvaluator.CH, "timeout", string.Format("minutes=%1 driven by flow.timeLimitSeconds - no second timer is started here", rule.timeoutMinutes));

			return;
		}

		if (mode == MODE_EXTRACTION)
		{
			if (rule.extractionZoneId.IsEmpty())
				TBD_Log.Error(TBD_WinConditionEvaluator.CH, "winConditions.mode is 'extraction' with no extractionZoneId - schema-required, so this document should not have reached a server. Nothing to evaluate.");
			else if (!UsableZone(rule.extractionZoneId))
				TBD_Log.Warn(TBD_WinConditionEvaluator.CH, string.Format("winConditions.extractionZoneId '%1' matches no usable zone in this mission - this rule can NEVER end the round.", rule.extractionZoneId));

			return;
		}

		if (mode == MODE_VIP)
		{
			if (rule.vipSlotId.IsEmpty())
				TBD_Log.Error(TBD_WinConditionEvaluator.CH, "winConditions.mode is 'vip' with no vipSlotId - schema-required, so this document should not have reached a server. Nothing to evaluate.");
			else if (rule.extractionZoneId.IsEmpty())
				TBD_Log.Kv(TBD_WinConditionEvaluator.CH, "vip", string.Format("slotUid=%1 protect-only (no extractionZoneId authored, so only the VIP's death ends the round)", rule.vipSlotId));
			else if (!UsableZone(rule.extractionZoneId))
				TBD_Log.Warn(TBD_WinConditionEvaluator.CH, string.Format("winConditions.extractionZoneId '%1' matches no usable zone - the VIP can be killed but never extracted.", rule.extractionZoneId));
		}
	}

	//! The zone with this id when it is usable; an unusable zone answers false to every containment
	//! test, so it is reported as missing rather than silently never firing.
	//! @param zoneId a `zones[].id`
	//! @return the usable zone, or null
	protected static TBD_Zone UsableZone(string zoneId)
	{
		TBD_Zone zone = TBD_ZoneRegistry.FindById(zoneId);
		if (!zone || !zone.IsUsable())
			return null;

		return zone;
	}

	//! `extraction`: a side has every living player inside the extraction zone. A side with no living
	//! player is skipped; a zone naming a faction extracts only that faction; a living player with
	//! no body counts as outside.
	//! @param rule the parsed rule
	//! @param winnerFaction set to the extracting side, empty when none
	//! @return true when a side extracted
	//! @authority server
	static bool EvaluateExtraction(notnull TBD_WinConditionsStruct rule, out string winnerFaction)
	{
		winnerFaction = string.Empty;

		TBD_Zone zone = UsableZone(rule.extractionZoneId);
		if (!zone)
			return false;

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		PlayerManager players = GetGame().GetPlayerManager();
		if (!sm || !players)
			return false;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		map<string, int> living = new map<string, int>();
		map<string, int> inside = new map<string, int>();

		foreach (int playerId : connected)
		{
			TBD_MissionSlotStruct slot = sm.GetAssignedSlot(playerId);
			if (!slot || slot.faction.IsEmpty())
				continue;
			if (sm.IsPlayerDead(playerId))
				continue;

			int alive = 0;
			living.Find(slot.faction, alive);
			living.Set(slot.faction, alive + 1);

			IEntity body = players.GetPlayerControlledEntity(playerId);
			if (!body)
				continue;

			vector origin = body.GetOrigin();
			if (!zone.Contains(origin[0], origin[2]))
				continue;

			int here = 0;
			inside.Find(slot.faction, here);
			inside.Set(slot.faction, here + 1);
		}

		foreach (string faction, int alive : living)
		{
			if (alive < 1)
				continue;
			if (!zone.m_sFaction.IsEmpty() && zone.m_sFaction != faction)
				continue;

			int here = 0;
			inside.Find(faction, here);
			if (here < alive)
				continue;

			winnerFaction = faction;
			return true;
		}

		return false;
	}

	//! `vip`: the player on seat `vipSlotId` (the stable `slots[].uid`) died (`vip_down`, the one
	//! other fielded side wins) or reached the extraction zone (`vip_extracted`, their side wins).
	//! An unclaimed seat fires nothing and is warned about once.
	//! @param rule the parsed rule
	//! @param winnerFaction set to the winning side, empty when ambiguous
	//! @param reason set to `vip_down` or `vip_extracted`
	//! @return true when the rule fired
	//! @authority server
	static bool EvaluateVip(notnull TBD_WinConditionsStruct rule, out string winnerFaction, out string reason)
	{
		winnerFaction = string.Empty;
		reason = string.Empty;

		if (rule.vipSlotId.IsEmpty())
			return false;

		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		PlayerManager players = GetGame().GetPlayerManager();
		if (!sm || !players)
			return false;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		int vipPlayerId = -1;
		string vipFaction;

		foreach (int playerId : connected)
		{
			TBD_MissionSlotStruct slot = sm.GetAssignedSlot(playerId);
			if (!slot || slot.uid != rule.vipSlotId)
				continue;

			vipPlayerId = playerId;
			vipFaction = slot.faction;
			break;
		}

		if (vipPlayerId < 0)
		{
			if (TBD_AnnounceOnce.Claim(VIP_MISSING_KEY))
			{
				TBD_Log.Warn(TBD_WinConditionEvaluator.CH, string.Format("winConditions.vipSlotId '%1' is held by nobody on the server - this rule cannot fire until that seat is claimed.", rule.vipSlotId));
			}
			return false;
		}

		if (sm.IsPlayerDead(vipPlayerId))
		{
			winnerFaction = SoleOtherSide(vipFaction);
			reason = "vip_down";
			return true;
		}

		if (rule.extractionZoneId.IsEmpty())
			return false;

		TBD_Zone zone = UsableZone(rule.extractionZoneId);
		if (!zone)
			return false;

		IEntity body = players.GetPlayerControlledEntity(vipPlayerId);
		if (!body)
			return false;

		vector origin = body.GetOrigin();
		if (!zone.Contains(origin[0], origin[2]))
			return false;

		winnerFaction = vipFaction;
		reason = "vip_extracted";
		return true;
	}

	//! The one side other than `factionKey` with claimed slots; with more than one the winner is
	//! ambiguous and left empty rather than invented.
	//! @param factionKey the losing side
	//! @return the other side's key, or empty unless exactly one exists
	protected static string SoleOtherSide(string factionKey)
	{
		TBD_SpawnManager sm = TBD_SpawnManager.GetInstance();
		array<ref TBD_MissionFactionStruct> factions = TBD_MissionLoader.GetFactions();
		if (!sm || !factions)
			return string.Empty;

		string only;
		int found;

		foreach (TBD_MissionFactionStruct f : factions)
		{
			if (!f || f.key.IsEmpty() || f.key == factionKey)
				continue;
			if (sm.CountClaimedForFaction(f.key) == 0)
				continue;

			only = f.key;
			found++;
		}

		if (found != 1)
			return string.Empty;

		return only;
	}
}
