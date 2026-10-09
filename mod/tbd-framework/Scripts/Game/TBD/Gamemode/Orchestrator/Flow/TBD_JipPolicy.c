/**
 * @file TBD_JipPolicy.c
 * @brief Resolves `flow.jip` and answers whether a player may join at a given stage.
 *
 * Role: turns the authored join policy into TBD_EJipPolicy, its schema name and a per-stage answer.
 * Position: fed by TBD_MissionFlow.RawJip; read by the join door in TBD_SpawnJoinAudit and by
 * TBD_MissionFlowReport.
 * State: none.  Invariants: an absent or unrecognised value resolves to ALWAYS; every method is silent,
 * because the join path calls it; the permitted-stage label is derived from AllowsJoinAtStage.
 */

//! Join-in-progress policy of the loaded mission.
class TBD_JipPolicy
{
	//! The policy in force. An unrecognised or absent value resolves to ALWAYS silently; the load
	//! report (TBD_MissionFlowReport) names an unrecognised value once.
	//! @return the resolved policy
	static TBD_EJipPolicy Resolved()
	{
		return FromString(TBD_MissionFlow.RawJip());
	}

	//! Map a schema string onto the enum.
	//! @param raw the `flow.jip` string
	//! @return DISABLED or UNTIL_SAFESTART_END when named, else ALWAYS
	static TBD_EJipPolicy FromString(string raw)
	{
		if (raw == "disabled")
			return TBD_EJipPolicy.DISABLED;

		if (raw == "until_safestart_end")
			return TBD_EJipPolicy.UNTIL_SAFESTART_END;

		return TBD_EJipPolicy.ALWAYS;
	}

	//! Whether `raw` is an implemented value; the load report names any other value.
	//! @param raw the `flow.jip` string
	//! @return true for `disabled`, `until_safestart_end` and `always`
	static bool IsKnownString(string raw)
	{
		return raw == "disabled" || raw == "until_safestart_end" || raw == "always";
	}

	//! The resolved policy as the schema spells it, for logs and the join door's refusal label.
	//! @return `disabled`, `until_safestart_end` or `always`
	static string Name()
	{
		TBD_EJipPolicy policy = Resolved();

		if (policy == TBD_EJipPolicy.DISABLED)
			return "disabled";

		if (policy == TBD_EJipPolicy.UNTIL_SAFESTART_END)
			return "until_safestart_end";

		return "always";
	}

	//! Whether the authored policy permits a player arriving at `stage` into the world. Answers the
	//! author's rule only; one life, spent lives and slot bodies stay TBD_SpawnManager's guards.
	//! LOADING is permitted because TBD_SpawnJoinAudit.IsStageDeployable already refuses it.
	//! @param stage the round stage the player arrives in
	//! @return true when a join is permitted
	static bool AllowsJoinAtStage(TBD_EGameStage stage)
	{
		TBD_EJipPolicy policy = Resolved();

		if (policy == TBD_EJipPolicy.ALWAYS)
			return true;

		// Every player arrives during LOBBY, so no policy closes it.
		if (stage == TBD_EGameStage.LOADING || stage == TBD_EGameStage.LOBBY)
			return true;

		// DISABLED closes the roster once the sides start planning.
		if (policy == TBD_EJipPolicy.DISABLED)
			return false;

		// UNTIL_SAFESTART_END: open through planning and warmup, shut from LIVE on.
		return stage == TBD_EGameStage.BRIEFING || stage == TBD_EGameStage.SAFE_START;
	}

	//! The stages a join is permitted in, built by asking AllowsJoinAtStage for LOBBY to LIVE.
	//! @return a comma-separated stage list, or `none`
	static string JoinsPermittedLabel()
	{
		string label = string.Empty;

		for (int i = TBD_EGameStage.LOBBY; i <= TBD_EGameStage.LIVE; i++)
		{
			if (!AllowsJoinAtStage(i))
				continue;

			if (!label.IsEmpty())
				label += ",";

			label += typename.EnumToString(TBD_EGameStage, i);
		}

		if (label.IsEmpty())
			return "none";

		return label;
	}
}
