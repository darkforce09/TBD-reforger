/**
 * @file TBD_MissionFlow.c
 * @brief Typed answers from the mission's `flow` block: the briefing, safe start and round lengths.
 *
 * Role: reads `flow` off the live mission document and resolves each duration to a value or UNSET.
 * Position: fed by TBD_MissionLoader.GetMission; read by TBD_MissionFlowReport, TBD_RoundClock,
 * TBD_JipPolicy, TBD_WinConditionEvaluator and the mission validator.
 * State: none; every call reads the live document, so nothing outlives an in-process world restart.
 * Invariants: presence is tested against TBD_MissionFlowStruct.ABSENT, never with a null test (the
 * JSON reader allocates `flow` even when the key is absent); an authored `0` stays distinct from
 * UNSET; a negative value resolves to UNSET with source SRC_INVALID.
 */

//! The one reader of the mission's `flow` block. Clients and a server before the load hold no
//! document, so every accessor guards `doc` and reports ABSENT or UNSET.
class TBD_MissionFlow
{
	static const string CH_FLOW = "Flow"; //!< log channel of every `[TBD][Flow]` line

	//! @contract mission.schema.json#/$defs/winConditions/properties/endOn
	static const string TRIGGER_TIME_LIMIT = "time_limit"; //!< endOn value armed by TBD_RoundClock
	static const string TRIGGER_FACTION_ELIMINATED = "faction_eliminated"; //!< endOn value armed by TBD_FactionElimination

	static const int UNSET = -1; //!< no usable authored duration; negative so it never collides with an authored `0`

	static const string SRC_AUTHORED = "authored"; //!< ResolveSeconds source: the mission authored the value
	static const string SRC_DEFAULT  = "default"; //!< ResolveSeconds source: the key is absent
	static const string SRC_INVALID  = "INVALID"; //!< ResolveSeconds source: the authored value is negative

	//! The raw block, or null when no mission document exists. A non-null block says nothing about
	//! whether `flow` was authored; callers test its fields against ABSENT.
	//! @return the document's flow block, or null
	protected static TBD_MissionFlowStruct Block()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc)
			return null;

		return doc.flow;
	}

	//! Authored `flow.briefingSeconds` as written.
	//! @return the raw value, or TBD_MissionFlowStruct.ABSENT when absent or no document exists
	static int RawBriefingSeconds()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return TBD_MissionFlowStruct.ABSENT;

		return flow.briefingSeconds;
	}

	//! Authored `flow.safeStartSeconds` as written.
	//! @return the raw value, or TBD_MissionFlowStruct.ABSENT when absent or no document exists
	static int RawSafeStartSeconds()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return TBD_MissionFlowStruct.ABSENT;

		return flow.safeStartSeconds;
	}

	//! Authored `flow.timeLimitSeconds` as written.
	//! @return the raw value, or TBD_MissionFlowStruct.ABSENT when absent or no document exists
	static int RawTimeLimitSeconds()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return TBD_MissionFlowStruct.ABSENT;

		return flow.timeLimitSeconds;
	}

	//! Authored `flow.jip` as written.
	//! @return the raw string, or empty when absent or no document exists
	static string RawJip()
	{
		TBD_MissionFlowStruct flow = Block();
		if (!flow)
			return string.Empty;

		return flow.jip;
	}

	//! Resolve one raw duration. A negative is neither clamped nor defaulted: the schema declares
	//! `minimum: 0`, so it comes back UNSET with source SRC_INVALID for the caller to report.
	//! @param raw a Raw*Seconds value
	//! @param source set to SRC_AUTHORED, SRC_DEFAULT or SRC_INVALID
	//! @return the authored seconds, or UNSET
	static int ResolveSeconds(int raw, out string source)
	{
		if (raw == TBD_MissionFlowStruct.ABSENT)
		{
			source = SRC_DEFAULT;
			return UNSET;
		}

		if (raw < 0)
		{
			source = SRC_INVALID;
			return UNSET;
		}

		source = SRC_AUTHORED;
		return raw;
	}

	//! Authored BRIEFING length; advisory, nothing advances on it (TBD_MissionFlowReport.AnnounceBriefing).
	//! @return seconds, or UNSET
	static int BriefingSeconds()
	{
		string source;
		return ResolveSeconds(RawBriefingSeconds(), source);
	}

	//! Authored safe start countdown length.
	//! @return seconds, or UNSET
	static int SafeStartSeconds()
	{
		string source;
		return ResolveSeconds(RawSafeStartSeconds(), source);
	}

	//! Authored round length; `0` is an explicit "no limit" and differs from UNSET.
	//! @return seconds, or UNSET
	static int TimeLimitSeconds()
	{
		string source;
		return ResolveSeconds(RawTimeLimitSeconds(), source);
	}

	//! Forwarder to [TBD_JipPolicy.AllowsJoinAtStage].
	//! @param stage the round stage a player arrives in
	//! @return true when the authored policy permits a join at `stage`
	static bool AllowsJoinAtStage(TBD_EGameStage stage) { return TBD_JipPolicy.AllowsJoinAtStage(stage); }

	//! Forwarder to [TBD_JipPolicy.Name].
	//! @return the resolved policy as the schema spells it
	static string JipPolicyName() { return TBD_JipPolicy.Name(); }
}
