//! T-936.1 - the AUTHORED win rule: the Enfusion half of `mission.schema.json#/$defs/winConditions`.
//!
//! == What was missing, and what this is ======================================================
//! `winConditions.endOn` has always been read - `TBD_ObjectiveRegistry.EvaluateEndTriggers` drives
//! three of the five triggers and `TBD_FrameworkManager.TickWinConditions` the other two.
//! `winConditions.mode` was read by exactly one line in the whole tree:
//! `TBD_BriefingData.BuildWinConditions` prints `Humanise(mode)` on the briefing screen. So the
//! mission could SAY it was a VIP mission and the round still ended on attrition. This file is the
//! reader that makes the mode mean something, for the two modes that need runtime observation.
//!
//! == The division of labour, and why it is not five branches ==================================
//! Five modes, three of which already have a runtime:
//!   * `attrition` - `TBD_FrameworkManager.TickWinConditions` counts the sides that still have a
//!     living player. Nothing here.
//!   * `objective` - `TBD_ObjectiveRegistry.EvaluateEndTriggers` owns all three objective
//!     triggers. Nothing here except the arm-time check that the mission declared one of them.
//!   * `timeout`   - `TBD_FrameworkManager.ArmRoundClock` runs the ONE round clock, off
//!     `flow.timeLimitSeconds`, which the compile writes from `timeoutMinutes`. Nothing here
//!     except the arm-time check, and deliberately NO timer: two timers for one deadline is the
//!     T-946.19 defect, where both pass the same stale-timer defence and every authored duration
//!     is silently halved.
//!   * `extraction` and `vip` - nothing observes a player standing in a zone or a named player
//!     dying, so those two are evaluated HERE.
//!
//! == Ends the round exactly once =============================================================
//! `s_bEnded` is a LATCH, set before `SetStage` is called and cleared only by `Clear()` on the way
//! into a new world. A heartbeat with no latch is the wave-241 defect (T-946.19): the tick fires
//! every 2 s, and a condition that stays true - a dead VIP stays dead - would call `SetStage(END)`
//! on every one of them. The tick also stops re-arming once the latch is set, so the cost after an
//! ending is zero rather than one no-op evaluation per tick.
//!
//! == Server-side only ========================================================================
//! Clients hold NO mission document (`TBD_FrameworkManager.OnPostInit` returns early for
//! `RplMode.Client` before `BeginLoad()`), so a client would read an empty rule and conclude the
//! mission authored none. The heartbeat refuses to start on a client; `Read()` returns false there
//! anyway, which is the correct answer for a machine that is not the authority.
//!
//! == Static, and therefore explicitly cleared ================================================
//! Statics OUTLIVE A WORLD inside one process (recorded landmine - `SelectMissionByNumber` restarts
//! the scenario in-process). Cleared in `OnGameStart` on the way IN, which is strictly stronger
//! than a teardown hook: it does not depend on the previous world having shut down tidily. Without
//! it, mission B would inherit mission A's `s_bEnded` and could never end at all.
//!
//! == Why the wire is parsed a SECOND time ====================================================
//! `TBD_MissionWinConditionsStruct` (Backend/TBD_MissionLoader.c) declares `mode` and `endOn` and
//! nothing else, and `JsonLoadContext` maps JSON keys onto NAMED class fields only - a key no class
//! declares is invisible at runtime, not rejected, not logged, simply absent. So the three params
//! this file needs (`extractionZoneId`, `vipSlotId`, `timeoutMinutes`) are read by a second
//! `JsonLoadContext` pass over `TBD_MissionLoader.GetRawJson()`. That is the pattern
//! `Objectives/TBD_ObjectiveRules.c` established and `Zones/TBD_TriggerRuntime.c` reuses, for the
//! same two reasons: the vocabulary stays next to the code that interprets it, and `Backend/**`
//! belongs to another slice's lane. The cost is one extra parse of a document that is at most 8 MB
//! and is parsed exactly ONCE per world.
//! @contract mission.schema.json#/$defs/winConditions

//------------------------------------------------------------------------------------------------
//! The params half of `$defs/winConditions`. Field names must equal the JSON keys - JsonLoadContext
//! maps by name.
//!
//! `endOn` is deliberately NOT declared. `TBD_MissionLoader.HasEndTrigger` already reads it off the
//! primary parse and is the ONE authority on which triggers a mission declared; a second copy here
//! would be a second answer to the same question, which is the T-346 shape (the bug is the
//! DISAGREEMENT, not either site's rule).
class TBD_WinConditionsStruct
{
	//! Sentinel for "key absent from JSON", the `TBD_ObjectiveRulesStruct.ABSENT_INT` device.
	//! `JsonLoadContext` leaves a missing key at its field initializer, so an initializer no sane
	//! author could type doubles as the presence flag. It must not be 0 or any authorable minute
	//! count; `$defs/winConditions.timeoutMinutes` has `minimum: 1`, so -1 is safe.
	static const int ABSENT_INT = -1;

	string mode;              //!< attrition | objective | extraction | vip | timeout. Empty = absent.
	string extractionZoneId;  //!< `zones[].id`. Required by extraction, optional on vip. Empty = absent.
	string vipSlotId;         //!< `slots[].uid` of the protected player. Empty = absent.
	int timeoutMinutes = ABSENT_INT; //!< Round length. Read for reporting only - see the header.
}

//------------------------------------------------------------------------------------------------
//! The document root for the win-rule second pass: declares `winConditions` and nothing else, so
//! this reader stays blind to every other top-level key.
class TBD_WinConditionDocStruct
{
	ref TBD_WinConditionsStruct winConditions;
}

//------------------------------------------------------------------------------------------------
//! Reads the authored win rule once, evaluates the two modes nothing else observes, and ends the
//! round exactly once.
class TBD_WinConditionEvaluator
{
	//! Log channel. A literal rather than a `TBD_Log.CH_*` constant for the reason
	//! `TBD_ObjectiveRegistry.CH` gives: `Core/TBD_Log.c` belongs to another slice's lane, and
	//! keeping the string in one place here preserves the greppable-tag property the constants
	//! exist for. Fold it into `TBD_Log` when that file is next open.
	static const string CH = "Win";

	//! `$defs/winConditions.mode`, the five the editor authors. `mission.schema.json`'s enum also
	//! grandfathers two hand-authored golden values; anything not on this list is MODE_UNKNOWN to
	//! this file and the round ends on `endOn` alone, which is the pre-T-936.1 behaviour those
	//! documents were written for.
	static const string MODE_ATTRITION  = "attrition";
	static const string MODE_OBJECTIVE  = "objective";
	static const string MODE_EXTRACTION = "extraction";
	static const string MODE_VIP        = "vip";
	static const string MODE_TIMEOUT    = "timeout";

	//! `winConditions.endOn` values this file only ever ASKS about - it never fires them.
	static const string TRIGGER_TIME_LIMIT     = "time_limit";
	static const string TRIGGER_ALL_CAPTURED   = "all_objectives_captured";
	static const string TRIGGER_DESTROYED      = "objective_destroyed";
	static const string TRIGGER_HOLD_EXPIRED   = "hold_expired";

	//! Heartbeat period. The same 2 s `TBD_FrameworkManager.TickWinConditions` runs at, because the
	//! two answer the same question at the same granularity and a faster one here would only make
	//! the two disagree about which fired first.
	static const int TICK_MS = 2000;

	protected static ref TBD_WinConditionsStruct s_Rule;
	protected static bool s_bRead;
	protected static bool s_bEnded;
	protected static bool s_bAnnounced;
	protected static bool s_bVipMissingReported;

	//------------------------------------------------------------------------------------------------
	//! Reset for a new world. See the header on why this runs on the way IN.
	static void Clear()
	{
		s_Rule = null;
		s_bRead = false;
		s_bEnded = false;
		s_bAnnounced = false;
		s_bVipMissingReported = false;
	}

	//------------------------------------------------------------------------------------------------
	//! Has the round already been ended by this evaluator? The latch, readable for tests and logs.
	static bool HasEnded()
	{
		return s_bEnded;
	}

	//------------------------------------------------------------------------------------------------
	//! Parse. Idempotent: only the first call after a `Clear()` does work.
	static bool Read()
	{
		if (s_bRead)
			return s_Rule != null;

		s_bRead = true;
		s_Rule = null;

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
			return false;

		TBD_WinConditionDocStruct doc = new TBD_WinConditionDocStruct();
		if (!ctx.ReadValue("", doc))
			return false;

		// NOT `if (doc.winConditions)`. That is ALWAYS true after a successful parse - JsonLoadContext
		// ALLOCATES a nested `ref <class>` field even when the JSON key is ABSENT (measured on a live
		// world boot; TBD_MissionLoader.c:31-42 and TBD_SpawnManager.c:1266-1286 record it). The
		// presence test has to be a scalar sentinel, and `mode` is schema-REQUIRED inside the block,
		// so an empty `mode` means the block itself was not there.
		if (!doc.winConditions || doc.winConditions.mode.IsEmpty())
			return false;

		s_Rule = doc.winConditions;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	//! The authored mode, or empty when this mission authored no win rule.
	static string Mode()
	{
		if (!Read())
			return string.Empty;

		return s_Rule.mode;
	}

	//------------------------------------------------------------------------------------------------
	//! One evaluation. Called by the heartbeat below; safe to call at any stage.
	//! @authority server - the mission document lives where the rule is evaluated.
	static void Tick()
	{
		if (s_bEnded)
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
			return;

		string mode = Mode();
		if (mode.IsEmpty())
			return;

		AnnounceOnce(mode);

		string winner;

		if (mode == MODE_EXTRACTION)
		{
			if (EvaluateExtraction(winner))
				EndRound("extraction", winner);
			return;
		}

		if (mode == MODE_VIP)
		{
			string reason;
			if (EvaluateVip(winner, reason))
				EndRound(reason, winner);
			return;
		}

		// attrition / objective / timeout / a grandfathered golden mode: the existing runtimes own
		// the ending. Doing nothing here is the whole point - see the header's division of labour.
	}

	//------------------------------------------------------------------------------------------------
	//! One block of lines per world, the first time the round is LIVE, saying what this rule needs
	//! and whether the mission can satisfy it.
	//!
	//! Every line below is a condition under which the AUTHORED RULE CAN NEVER FIRE. That is worth
	//! a warning rather than silence for the reason `TBD_ObjectiveRegistry` warns about an endOn
	//! trigger with no zone to drive it: a mission that simply never ends reads as a server fault,
	//! and the operator has no way to tell it from one.
	protected static void AnnounceOnce(string mode)
	{
		if (s_bAnnounced)
			return;

		s_bAnnounced = true;
		TBD_Log.Kv(CH, "rule", string.Format("mode=%1", mode));

		if (mode == MODE_OBJECTIVE)
		{
			bool anyObjectiveTrigger = TBD_MissionLoader.HasEndTrigger(TRIGGER_ALL_CAPTURED);
			if (!anyObjectiveTrigger)
				anyObjectiveTrigger = TBD_MissionLoader.HasEndTrigger(TRIGGER_DESTROYED);
			if (!anyObjectiveTrigger)
				anyObjectiveTrigger = TBD_MissionLoader.HasEndTrigger(TRIGGER_HOLD_EXPIRED);

			if (!anyObjectiveTrigger)
				TBD_Log.Warn(CH, "winConditions.mode is 'objective' but endOn declares none of all_objectives_captured / objective_destroyed / hold_expired - the objective registry drives those three, so this rule can NEVER end the round.");

			return;
		}

		if (mode == MODE_TIMEOUT)
		{
			if (!TBD_MissionLoader.HasEndTrigger(TRIGGER_TIME_LIMIT))
				TBD_Log.Warn(CH, "winConditions.mode is 'timeout' but endOn does not declare 'time_limit' - the round clock only arms on that trigger, so this rule can NEVER end the round.");
			else
				TBD_Log.Kv(CH, "timeout", string.Format("minutes=%1 driven by flow.timeLimitSeconds - no second timer is started here", s_Rule.timeoutMinutes));

			return;
		}

		if (mode == MODE_EXTRACTION)
		{
			if (s_Rule.extractionZoneId.IsEmpty())
				TBD_Log.Error(CH, "winConditions.mode is 'extraction' with no extractionZoneId - schema-required, so this document should not have reached a server. Nothing to evaluate.");
			else if (!FindZone(s_Rule.extractionZoneId))
				TBD_Log.Warn(CH, string.Format("winConditions.extractionZoneId '%1' matches no usable zone in this mission - this rule can NEVER end the round.", s_Rule.extractionZoneId));

			return;
		}

		if (mode == MODE_VIP)
		{
			if (s_Rule.vipSlotId.IsEmpty())
				TBD_Log.Error(CH, "winConditions.mode is 'vip' with no vipSlotId - schema-required, so this document should not have reached a server. Nothing to evaluate.");
			else if (s_Rule.extractionZoneId.IsEmpty())
				TBD_Log.Kv(CH, "vip", string.Format("slotUid=%1 protect-only (no extractionZoneId authored, so only the VIP's death ends the round)", s_Rule.vipSlotId));
			else if (!FindZone(s_Rule.extractionZoneId))
				TBD_Log.Warn(CH, string.Format("winConditions.extractionZoneId '%1' matches no usable zone - the VIP can be killed but never extracted.", s_Rule.extractionZoneId));
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The usable zone with this id, or null.
	//!
	//! `IsUsable()` is asked here rather than at the containment call for the reason
	//! `TBD_Zone.Contains` documents: an unusable zone (no shape, or a polygon with fewer than three
	//! vertices) answers `false` to everything, so treating it as "found" would turn a broken zone
	//! into a rule that silently never fires instead of one the log names.
	protected static TBD_Zone FindZone(string zoneId)
	{
		if (zoneId.IsEmpty())
			return null;

		array<ref TBD_Zone> zones = TBD_ZoneRegistry.GetAll();
		if (!zones)
			return null;

		foreach (TBD_Zone zone : zones)
		{
			if (!zone || !zone.IsUsable())
				continue;
			if (zone.m_sId == zoneId)
				return zone;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! `mode: extraction` - a side has every one of its LIVING players inside the extraction zone.
	//!
	//! Two guards that are the whole difference between a rule and a bug:
	//!
	//!   * A side with ZERO living players trivially satisfies "all of them are inside", so it is
	//!     skipped. Without that, the first side wiped out would WIN the extraction, which is the
	//!     exact opposite of the rule.
	//!   * A zone that names a faction (`zones[].faction`, slugged by `flatten.rs` into the same
	//!     vocabulary as `slots[].faction`) extracts only that faction. A zone that names none is
	//!     open to whichever side gets all of its living players there first.
	//!
	//! A living player with no BODY (mid-deploy, spectating) is counted as living and NOT inside,
	//! so a side cannot extract while one of its players is between bodies. That is the strict
	//! reading on purpose: the generous one would end the round early, and under one life an
	//! early ending cannot be undone.
	protected static bool EvaluateExtraction(out string winnerFaction)
	{
		winnerFaction = string.Empty;

		TBD_Zone zone = FindZone(s_Rule.extractionZoneId);
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

	//------------------------------------------------------------------------------------------------
	//! `mode: vip` - the named player died (their side loses) or reached the extraction zone (their
	//! side wins).
	//!
	//! `reason` is the label the ending is logged under, so a round that ended on a VIP death and
	//! one that ended on a VIP extraction are distinguishable in a server log without inferring it
	//! from the winner.
	//!
	//! The VIP is found by `slots[].uid`, the STABLE slot identity, not by `slots[].id` - the id is
	//! re-derived on every compile (`{faction}:{callsign}:{role}:{index}`) and would break the
	//! moment a squad was renamed, which is precisely the property `uid` exists to provide.
	//!
	//! A VIP who has not claimed in yet is not "alive" or "dead" - they are absent, so nothing
	//! fires and the fact is reported ONCE. Ending the round because the VIP had not connected
	//! would hand the win to the other side at kickoff.
	protected static bool EvaluateVip(out string winnerFaction, out string reason)
	{
		winnerFaction = string.Empty;
		reason = string.Empty;

		if (s_Rule.vipSlotId.IsEmpty())
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
			if (!slot || slot.uid != s_Rule.vipSlotId)
				continue;

			vipPlayerId = playerId;
			vipFaction = slot.faction;
			break;
		}

		if (vipPlayerId < 0)
		{
			if (!s_bVipMissingReported)
			{
				s_bVipMissingReported = true;
				TBD_Log.Warn(CH, string.Format("winConditions.vipSlotId '%1' is held by nobody on the server - this rule cannot fire until that seat is claimed.", s_Rule.vipSlotId));
			}
			return false;
		}

		if (sm.IsPlayerDead(vipPlayerId))
		{
			winnerFaction = SoleOtherSide(vipFaction);
			reason = "vip_down";
			return true;
		}

		if (s_Rule.extractionZoneId.IsEmpty())
			return false;

		TBD_Zone zone = FindZone(s_Rule.extractionZoneId);
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

	//------------------------------------------------------------------------------------------------
	//! The one side that is not `factionKey` and has claimed slots, or empty when there is not
	//! exactly one.
	//!
	//! "The VIP's side loses" names a loser, not a winner. With two sides fielded the winner is
	//! unambiguous; with three it is not, and inventing one would publish a result the mission never
	//! stated. An empty winner is legal - `TBD_ObjectiveRegistry.EvaluateEndTriggers` already
	//! returns one for a destroy objective whose zone named no faction - and it is the honest answer.
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

	//------------------------------------------------------------------------------------------------
	//! End the round. THE LATCH LIVES HERE, and it is set BEFORE `SetStage` so a re-entrant call
	//! from inside the stage transition cannot get past it either.
	//!
	//! See the header: the tick runs every 2 s and every condition it evaluates STAYS TRUE once met
	//! (a dead VIP stays dead), so without this the round would be ended over and over.
	protected static void EndRound(string reason, string winnerFaction)
	{
		if (s_bEnded)
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return;

		s_bEnded = true;

		string winnerLabel = winnerFaction;
		if (winnerLabel.IsEmpty())
			winnerLabel = "(none named)";

		TBD_Log.Event(CH, string.Format("[TBD][Win] %1 - winner=%2", reason, winnerLabel));
		fm.SetStage(TBD_EGameStage.END);
	}
}

//------------------------------------------------------------------------------------------------
//! T-936.1 - the win-rule heartbeat. Same shape, and the same reasoning, as
//! `TBD_TriggerRuntime`'s block: a game-mode COMPONENT this slice cannot add to
//! `Prefabs/Systems/TBD_GameMode.et` would never be instantiated, and a runtime that can never fire
//! is worse than an absent one - it reads as shipped in every grep and is dead in every round.
modded class SCR_BaseGameMode
{
	//! T-946.19 - set once the heartbeat is scheduled. See `TBD_WinConditionOnStart`.
	protected bool m_bTBD_WinConditionTickArmed;

	//------------------------------------------------------------------------------------------------
	//! @authority server - the win rule is evaluated where the mission document lives.
	//!
	//! Statics outlive a world inside one process, so the evaluator is cleared HERE, at the start of
	//! each world, rather than in a teardown hook this class does not have. Clearing on the way in is
	//! strictly stronger: it does not depend on the previous world having shut down tidily, and
	//! without it mission B would inherit mission A's "already ended" latch and could never end.
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_WinConditionEvaluator.Clear();

		if (RplSession.Mode() == RplMode.Client)
			return;

		// The fence that keeps this out of vanilla scenarios that merely have the mod loaded. It is
		// the published test every vanilla-touching modded block in this addon already asks, and it
		// resolves off the live game mode's components, which exist from construction.
		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		// T-946.19 - schedule AT MOST ONE tick per game mode instance. The stale-timer defence below
		// is `GetGame().GetGameMode() != this`, which two timers on the SAME instance both pass: if
		// OnGameStart ever ran twice here, the rule would be evaluated twice per beat. A latch costs
		// one bool and removes the whole class.
		if (m_bTBD_WinConditionTickArmed)
			return;

		m_bTBD_WinConditionTickArmed = true;
		GetGame().GetCallqueue().CallLater(TBD_WinConditionTick, TBD_WinConditionEvaluator.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	//! One evaluation, then re-arm.
	//!
	//! One-shot and self-re-arming rather than a repeating CallLater, for the reason
	//! `TBD_TriggerRuntime`'s twin records: `ScriptCallQueue.Remove` cancels BY FUNCTION and this
	//! class has no teardown hook to call it from, so a repeating timer would survive a world
	//! teardown and fire forever against a dead game mode. A one-shot that re-arms only while it is
	//! still the LIVE game mode's timer stops on its own the moment the world is replaced.
	//!
	//! It also stops re-arming once the round has been ended by the evaluator: after an ending there
	//! is nothing left to evaluate, and a timer that keeps waking to do nothing is a cost with no
	//! reader.
	void TBD_WinConditionTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_WinConditionEvaluator.Tick();

		if (TBD_WinConditionEvaluator.HasEnded())
			return;

		GetGame().GetCallqueue().CallLater(TBD_WinConditionTick, TBD_WinConditionEvaluator.TICK_MS, false);
	}
}
