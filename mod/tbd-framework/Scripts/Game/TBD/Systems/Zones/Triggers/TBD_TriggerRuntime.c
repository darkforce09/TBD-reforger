/**
 * @file TBD_TriggerRuntime.c
 * @brief The editor trigger registry of the loaded mission and its once-a-second evaluation.
 *
 * Role: builds the prepared triggers once per world, runs each one's ARMED, PENDING, FIRED state
 * machine while the round is LIVE, fires their effects and holds the runtime variant selection.
 * Position: ticked by `TBD_RuntimeHeartbeat` on the server in a framework world; prepares through
 * `TBD_TriggerCompiler`, reads `TBD_TriggerConditions`, fires through `TBD_TriggerEffects`; read by
 * the task state machine, audio emitters and spawn modules.
 * State: static registry, counts, built-for mission id, variant selection and player snapshot, for
 * the life of the script VM; reset by `Clear` at each world start.  Invariants: a registry built
 * for another mission is dropped, never evaluated; evaluation stands down, never guesses, when
 * presence cannot be read.
 */

//! The editor trigger registry and its evaluation.
//! @authority server
class TBD_TriggerRuntime
{
	static const string CH = "Trigger"; //!< log channel
	static const int TICK_MS = 1000; //!< evaluation cadence in milliseconds, called by the heartbeat
	static const float TICK_SECONDS = 1.0; //!< `TICK_MS` in seconds, the dwell accumulator step

	static const string ANNOUNCE_ARMED_KEY = "Trigger.armed"; //!< `TBD_AnnounceOnce` key of the armed or idle line
	static const string ANNOUNCE_NO_SNAPSHOT_KEY = "Trigger.noSnapshot"; //!< `TBD_AnnounceOnce` key of the stood-down line

	protected static ref array<ref TBD_Trigger> s_aTriggers; //!< every prepared trigger; null until built
	protected static bool s_bBuilt; //!< true once `Build` succeeded for this world
	protected static int s_iArmedCount; //!< triggers that can fire
	protected static int s_iInertCount; //!< triggers that can never fire
	protected static string s_sBuiltForMission; //!< mission id the registry was built for

	protected static ref array<string> s_aSelectedVariants; //!< variants selected by `set_variant`
	protected static bool s_bVariantSelectionInForce; //!< false until the first `set_variant`

	protected static ref TBD_TriggerPlayerSnapshot s_Snapshot; //!< this tick's player snapshot

	//! Whether the registry is built for this world.
	//! @return true once `Build` has succeeded since the last `Clear`
	static bool IsBuilt()
	{
		return s_bBuilt;
	}

	//! How many triggers can fire; 0 for a mission that authors none.
	//! @return the armed count
	static int GetArmedCount()
	{
		return s_iArmedCount;
	}

	//! How many authored triggers can never fire; each was reported by id at load.
	//! @return the inert count
	static int GetInertCount()
	{
		return s_iInertCount;
	}

	//! Every prepared trigger, INERT ones included.
	//! @return the registry, or null until `Build` has run
	static array<ref TBD_Trigger> GetAll()
	{
		return s_aTriggers;
	}

	//! The prepared trigger with this id.
	//! @param triggerId an `editorTriggers[].id`
	//! @return the trigger, or null when the registry is unbuilt or no trigger has that id
	static TBD_Trigger FindById(string triggerId)
	{
		if (!s_aTriggers)
			return null;

		foreach (TBD_Trigger trigger : s_aTriggers)
		{
			if (trigger && trigger.m_sId == triggerId)
				return trigger;
		}

		return null;
	}

	//! Whether the trigger with this id is FIRED right now. A repeating trigger reads false again
	//! once it re-arms.
	//! @param triggerId an `editorTriggers[].id`
	//! @param unknownId set true when the registry is built and no trigger has that id, so the
	//! caller can report its own dangling reference; false otherwise
	//! @return true when the trigger's state is FIRED
	static bool HasFired(string triggerId, out bool unknownId)
	{
		unknownId = false;
		if (!s_aTriggers)
			return false;

		TBD_Trigger trigger = FindById(triggerId);
		if (!trigger)
		{
			unknownId = true;
			return false;
		}

		return trigger.m_eState == TBD_ETriggerState.FIRED;
	}

	//! Drop the registry, the variant selection and the snapshot. Runs at every world start,
	//! since statics outlive a world when a mission restarts in-process.
	static void Clear()
	{
		s_aTriggers = null;
		s_bBuilt = false;
		s_iArmedCount = 0;
		s_iInertCount = 0;
		s_sBuiltForMission = string.Empty;
		s_aSelectedVariants = null;
		s_bVariantSelectionInForce = false;
		s_Snapshot = null;
		TBD_AnnounceOnce.Rearm(ANNOUNCE_ARMED_KEY);
		TBD_AnnounceOnce.Rearm(ANNOUNCE_NO_SNAPSHOT_KEY);
	}

	//! Select `variantId` for the rest of the round. The first call puts variant selection in
	//! force (logged once): from then on a trigger with a `variantId` runs only while that variant is
	//! selected. A trigger with no `variantId` always runs.
	//! @param variantId the variant to add to the selected set
	//! @return how many variants are selected
	static int SelectVariant(string variantId)
	{
		if (!s_aSelectedVariants)
			s_aSelectedVariants = new array<string>();

		if (!s_bVariantSelectionInForce)
		{
			s_bVariantSelectionInForce = true;
			TBD_Log.Event(CH, "variant selection is now IN FORCE - from here a trigger with a `variantId` runs only while that variant is selected; triggers with no `variantId` are unaffected");
		}

		if (s_aSelectedVariants.Find(variantId) == -1)
			s_aSelectedVariants.Insert(variantId);

		return s_aSelectedVariants.Count();
	}

	//! Prepare every authored trigger; only the first call after `Clear` does work. A mission that
	//! authors no `editorTriggers` builds an empty registry so the wire is not re-read each second.
	//! @return false while there is nothing to build from (no held mission document, or an unbuilt
	//! zone registry), so the caller keeps waiting instead of caching an empty answer
	static bool Build()
	{
		if (s_bBuilt)
			return true;

		// Without the zone registry a `zoneId` cannot be resolved; wait rather than widen it.
		if (!TBD_ZoneRegistry.IsBuilt())
			return false;

		string missionId = TBD_MissionLoader.GetMissionId();
		array<ref TBD_EditorTriggerStruct> raw = TBD_TriggerCompiler.ReadWire();
		if (!raw)
		{
			// No `editorTriggers` key is the common case and not an error.
			if (!TBD_MissionLoader.GetRawJson().IsEmpty())
			{
				s_aTriggers = new array<ref TBD_Trigger>();
				s_bBuilt = true;
				s_sBuiltForMission = missionId;
				return true;
			}

			return false;
		}

		s_aTriggers = new array<ref TBD_Trigger>();
		s_aSelectedVariants = new array<string>();
		s_iArmedCount = 0;
		s_iInertCount = 0;

		foreach (int index, TBD_EditorTriggerStruct rawTrigger : raw)
		{
			if (!rawTrigger)
			{
				TBD_Log.Warn(CH, string.Format("editorTriggers[%1] is null - skipped", index));
				continue;
			}

			TBD_Trigger trigger = TBD_TriggerCompiler.Prepare(rawTrigger, index);
			s_aTriggers.Insert(trigger);

			if (trigger.m_eState == TBD_ETriggerState.INERT)
			{
				s_iInertCount++;
				TBD_Log.Warn(CH, string.Format("%1 INERT: %2", trigger.LogKey(), trigger.m_sInertReason));
			}
			else
			{
				s_iArmedCount++;
				TBD_TriggerCompiler.LogPrepared(trigger);
			}
		}

		s_bBuilt = true;
		s_sBuiltForMission = missionId;

		TBD_Log.Kv(CH, "built", string.Format("mission='%1' triggers=%2 armed=%3 inert=%4 cadence=%5ms",
			missionId, raw.Count(), s_iArmedCount, s_iInertCount, TICK_MS));

		return true;
	}

	//! One evaluation pass, called every `TICK_MS` by `TBD_RuntimeHeartbeat`: builds the registry
	//! when needed, drops it when the loaded mission changed, resets every dwell outside LIVE, and
	//! stands down (one error line) when the player snapshot cannot be captured.
	//! @authority server
	static void Tick()
	{
		if (!s_bBuilt)
		{
			if (!Build())
				return;

			AnnounceOnce();
		}

		// Zone ids mean nothing outside the document they came from: a stale registry is dropped.
		string missionId = TBD_MissionLoader.GetMissionId();
		if (missionId != s_sBuiltForMission)
		{
			TBD_Log.Warn(CH, string.Format("the loaded mission is '%1' but the trigger registry was built for '%2' - rebuilding rather than firing another mission's triggers",
				missionId, s_sBuiltForMission));
			Clear();
			return;
		}

		if (!s_aTriggers || s_aTriggers.Count() == 0)
			return;

		// Only LIVE evaluates; any other stage drops every dwell so none resumes half-way.
		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
		{
			ResetDwell();
			return;
		}

		if (s_iArmedCount == 0)
			return;

		// No snapshot is no evidence, not an empty world: stand down.
		if (!s_Snapshot)
			s_Snapshot = new TBD_TriggerPlayerSnapshot();

		if (!s_Snapshot.Capture())
		{
			if (TBD_AnnounceOnce.Claim(ANNOUNCE_NO_SNAPSHOT_KEY))
			{
				TBD_Log.Error(CH, "trigger evaluation STOOD DOWN - no PlayerManager or no TBD_SpawnManager on this world, so presence cannot be read. Treating that as 'nobody is anywhere' would make every not_present trigger fire.");
			}

			ResetDwell();
			return;
		}

		TBD_AnnounceOnce.Rearm(ANNOUNCE_NO_SNAPSHOT_KEY);

		foreach (TBD_Trigger trigger : s_aTriggers)
		{
			if (trigger)
				Evaluate(trigger);
		}
	}

	//! Log once per world, when the registry is built, either the `armed` summary or that no
	//! trigger can fire.
	protected static void AnnounceOnce()
	{
		if (!TBD_AnnounceOnce.Claim(ANNOUNCE_ARMED_KEY))
			return;

		if (s_iArmedCount == 0)
		{
			TBD_Log.Event(CH, "this mission authors no trigger this build can fire - editorTriggers[] is absent, empty, or every entry is inert");
			return;
		}

		TBD_Log.Kv(CH, "armed", string.Format("triggers=%1 inert=%2 cadence=%3ms detectRadius=%4m",
			s_iArmedCount, s_iInertCount, TICK_MS, TBD_TriggerConditions.DETECT_RADIUS_M));
	}

	//! Return every PENDING trigger to ARMED with no dwell; called while the round is not LIVE.
	protected static void ResetDwell()
	{
		if (!s_aTriggers)
			return;

		foreach (TBD_Trigger trigger : s_aTriggers)
		{
			if (!trigger || trigger.m_eState != TBD_ETriggerState.PENDING)
				continue;

			trigger.m_eState = TBD_ETriggerState.ARMED;
			trigger.m_fHeldSeconds = 0;
		}
	}

	//! Advance one trigger's state machine by one tick. A trigger whose variant is not selected
	//! drops its dwell; a spent one-shot is skipped; a FIRED `repeat` trigger re-arms once its
	//! condition stops holding (a `timer` re-arms at once, making it periodic).
	//! @param trigger the trigger to advance
	protected static void Evaluate(notnull TBD_Trigger trigger)
	{
		if (trigger.m_eState == TBD_ETriggerState.INERT)
			return;

		if (!TBD_MissionVariants.IsActive(trigger.m_sVariantId, s_aSelectedVariants, s_bVariantSelectionInForce, false))
		{
			if (trigger.m_eState == TBD_ETriggerState.PENDING)
			{
				trigger.m_eState = TBD_ETriggerState.ARMED;
				trigger.m_fHeldSeconds = 0;
			}
			return;
		}

		if (trigger.m_eState == TBD_ETriggerState.FIRED && !trigger.m_bRepeat)
			return;

		bool holds = TBD_TriggerConditions.Holds(trigger, s_Snapshot);

		if (trigger.m_eState == TBD_ETriggerState.FIRED)
		{
			// A timer condition always holds, so a repeating timer re-arms without waiting to fall.
			if (!holds || trigger.m_eCondition == TBD_ETriggerCondition.TIMER)
			{
				trigger.m_eState = TBD_ETriggerState.ARMED;
				trigger.m_fHeldSeconds = 0;
				TBD_Log.Kv(CH, "rearmed", string.Format("id=%1 fires=%2", trigger.m_sId, trigger.m_iFireCount));
			}

			return;
		}

		if (!holds)
		{
			if (trigger.m_eState == TBD_ETriggerState.PENDING)
			{
				TBD_Log.Kv(CH, "lapsed", string.Format("id=%1 held=%2s of %3s",
					trigger.m_sId, trigger.m_fHeldSeconds, trigger.m_fTimeoutSeconds));
			}

			trigger.m_eState = TBD_ETriggerState.ARMED;
			trigger.m_fHeldSeconds = 0;
			return;
		}

		if (trigger.m_eState == TBD_ETriggerState.ARMED)
		{
			trigger.m_eState = TBD_ETriggerState.PENDING;
			trigger.m_fHeldSeconds = 0;

			TBD_Log.Kv(CH, "holding", string.Format("id=%1 condition=%2 owner='%3' timeout=%4s",
				trigger.m_sId,
				typename.EnumToString(TBD_ETriggerCondition, trigger.m_eCondition),
				trigger.m_sOwnerSide,
				trigger.m_fTimeoutSeconds));
		}
		else
		{
			trigger.m_fHeldSeconds += TICK_SECONDS;
		}

		if (trigger.m_fHeldSeconds < trigger.m_fTimeoutSeconds)
			return;

		Fire(trigger);
	}

	//! Latch the trigger FIRED, log the fire banner and run every usable effect in authored order
	//! (a `hint` written before `end_mission` is delivered before the round stops).
	//! @param trigger the trigger whose dwell completed
	protected static void Fire(notnull TBD_Trigger trigger)
	{
		trigger.m_eState = TBD_ETriggerState.FIRED;
		trigger.m_iFireCount++;
		trigger.m_fHeldSeconds = 0;

		TBD_Log.Banner(CH, string.Format("TRIGGER FIRED: %1 (condition=%2 owner='%3' dwell=%4s fires=%5)",
			trigger.m_sId,
			typename.EnumToString(TBD_ETriggerCondition, trigger.m_eCondition),
			trigger.m_sOwnerSide,
			trigger.m_fTimeoutSeconds,
			trigger.m_iFireCount), false);

		if (!trigger.m_aEffects)
			return;

		foreach (int index, TBD_TriggerEffect effect : trigger.m_aEffects)
		{
			if (!effect || !effect.m_bUsable)
				continue;

			TBD_TriggerEffects.Run(trigger, effect, index);
		}
	}
}
