/**
 * @file TBD_SpawnIdentityGate.c
 * @brief The stage gate that refuses SAFE_START and LIVE while ONE LIFE cannot be enforced.
 *
 * Role: refuses a stage where a life can be spent while a connected player resolves to a NUMERIC
 * bind key, and refuses LOBBY while the slot loadouts are unplayable or still settling; holds the
 * admin waiver.  Position: TBD_FrameworkManager.SetStage asks TBD_SpawnManager.StageRefusalFor;
 * TBD_AdminService signs, revokes and reads the waiver through TBD_SpawnManager.
 * State: the waiver (who signed it) and the late-join warning latch, server only, session scoped.
 * Invariants: the check is a live scan of the players connected at the transition, never a latch,
 * because the first audit of a join can resolve a transient NUMERIC key that a second audit
 * upgrades; LOBBY and BRIEFING stay open so an operator can read the refusal and fix the server;
 * a NUMERIC join after the gate ran is reported, never refused.
 */

//! Identity and loadout stage gate of the spawn manager.
class TBD_SpawnIdentityGate : Managed
{
	static const string CH_IDENTITY = "Identity"; //!< log channel of the identity gate

	protected TBD_SpawnManager m_Spawn; //!< owning manager
	protected bool m_bIdentityOverride; //!< true once an admin accepted non-durable keys; default false
	protected string m_sIdentityOverrideBy; //!< who signed the waiver (TBD_AdminService label form); empty when none
	protected bool m_bLateNonDurableJoinWarned; //!< latch: a NUMERIC late join was already reported

	//! Bind the gate to its manager.
	void TBD_SpawnIdentityGate(TBD_SpawnManager spawn)
	{
		m_Spawn = spawn;
	}

	//! True for the stages where a life can be spent: SAFE_START and LIVE.
	static bool RequiresDurableIdentity(TBD_EGameStage stage)
	{
		return stage == TBD_EGameStage.SAFE_START || stage == TBD_EGameStage.LIVE;
	}

	//! Why `stage` may not be entered, already logged as a banner, or empty when it may. A world
	//! without a TBD_SpawnManager answers empty: the component roll call reports that at ERROR.
	//! @authority server
	static string StageRefusal(TBD_EGameStage stage)
	{
		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();

		if (stage == TBD_EGameStage.LOBBY && spawn)
		{
			string loadoutRefusal = spawn.GetLoadoutSettle().LobbyRefusalReason();
			if (!loadoutRefusal.IsEmpty())
				return loadoutRefusal;
		}

		if (!RequiresDurableIdentity(stage))
			return string.Empty;

		if (!spawn)
			return string.Empty;

		return spawn.GetIdentityGate().RefusalFor(stage);
	}

	//! The identity half of the gate for a stage that requires durable identity: empty when ONE LIFE
	//! is off, when no connected player is NUMERIC, or under the waiver (announced); else the reason.
	//! @authority server
	string RefusalFor(TBD_EGameStage stage)
	{
		if (TBD_Authority.IsClient())
			return string.Empty;
		if (!m_Spawn.IsOneLife())
			return string.Empty;

		int connected;
		int nameHash;
		array<int> numericPlayers = {};
		int numeric = CensusIdentityModes(connected, nameHash, numericPlayers);

		if (numeric == 0)
		{
			if (connected == 0)
				Print(string.Format("[TBD][%1] stage=%2 identity gate INCONCLUSIVE -- no players connected, so the host's key mode cannot be observed yet. It is checked again on every transition, and a NUMERIC join after this point is reported at WARNING.",
					CH_IDENTITY, typename.EnumToString(TBD_EGameStage, stage)), LogLevel.WARNING);
			return string.Empty;
		}

		string stageName = typename.EnumToString(TBD_EGameStage, stage);
		string who = FormatPlayerIdList(numericPlayers);

		if (m_bIdentityOverride)
		{
			TBD_Log.Banner(CH_IDENTITY, string.Format("%1 ENTERED WITH ONE LIFE UNENFORCEABLE -- admin override by %2 covers %3 player(s) on a NUMERIC key (%4). Deaths on this host do NOT survive a reconnect.",
				stageName, m_sIdentityOverrideBy, numeric, who), false);
			Print(string.Format("[TBD][%1] override=%2 stage=%3 numeric=%4 connected=%5 -- proceeding under an explicitly accepted waiver",
				CH_IDENTITY, m_sIdentityOverrideBy, stageName, numeric, connected), LogLevel.WARNING);
			return string.Empty;
		}

		string why = string.Format("ONE LIFE cannot be enforced on this host, so %1 is refused. %2 of %3 connected player(s) resolve to a NUMERIC 'player:<id>' key (%4)",
			stageName, numeric, connected, who);
		why = why + " -- that is a lease on a NUMBER, not an identity, so a death does not survive a reconnect and a recycled id can hand a dead man's number to a new joiner.";
		why = why + " CAUSE: this dedicated server returned no backend identity for them (vanilla synthesizes a name-hash uuid only when the session is NOT Dedicated, so there is no fallback here).";
		why = why + " FIX: vanilla names it itself -- 'Dedicated server is not correctly configured to connect to the BI backend', see the server config's publicAddress/publicPort. Launching with -config is what registers the backend room at all. Fix it and try again; the gate re-checks on every transition.";
		why = why + string.Format(" To run anyway and accept that ONE LIFE is unenforceable: '#tbd identity override %1'.", TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE);

		TBD_Log.Banner(CH_IDENTITY, string.Format("%1 REFUSED -- %2 of %3 connected player(s) have NO durable identity (%4). ONE LIFE would be a promise this host cannot keep.",
			stageName, numeric, connected, who), true);
		Print(string.Format("[TBD][%1] %2", CH_IDENTITY, why), LogLevel.ERROR);
		return why;
	}

	//! Count how the connected players' bind keys resolve right now.
	//! @param connectedOut set to the connected player count
	//! @param nameHashOut set to the count on a NAME-HASH key
	//! @param numericPlayers filled with the ids on a NUMERIC key
	//! @return the count on a NUMERIC key
	//! @authority server
	int CensusIdentityModes(out int connectedOut, out int nameHashOut, notnull array<int> numericPlayers)
	{
		connectedOut = 0;
		nameHashOut = 0;
		numericPlayers.Clear();

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return 0;

		array<int> ids = {};
		int count = players.GetPlayers(ids);
		connectedOut = count;

		int numeric = 0;
		for (int i = 0; i < count; i++)
		{
			string mode = TBD_SpawnIdentityKeys.KeyModeLabel(m_Spawn.GetIdentity().PlayerBindKey(ids[i]));
			if (mode == "NUMERIC")
			{
				numeric++;
				numericPlayers.Insert(ids[i]);
			}
			else if (mode == "NAME-HASH")
			{
				nameHashOut++;
			}
		}
		return numeric;
	}

	//! `player=3,7,11`, capped at six ids so a full server cannot flood chat; "none" when empty.
	static string FormatPlayerIdList(notnull array<int> ids)
	{
		if (ids.IsEmpty())
			return "none";

		string joined;
		int shown = ids.Count();
		if (shown > 6)
			shown = 6;

		for (int i = 0; i < shown; i++)
		{
			if (i > 0)
				joined = joined + ",";
			joined = joined + ids[i].ToString();
		}

		if (ids.Count() > shown)
			joined = joined + string.Format(",...+%1", ids.Count() - shown);

		return "player=" + joined;
	}

	//! Sign the waiver. Session scoped: a restarted scenario enforces again.
	//! @param byAdmin the signer's label
	//! @param phrase must equal TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE
	//! @return true only for the exact phrase
	//! @authority server
	bool AcceptNonDurableIdentity(string byAdmin, string phrase)
	{
		if (phrase != TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE)
			return false;

		m_bIdentityOverride = true;
		m_sIdentityOverrideBy = byAdmin;

		TBD_Log.Banner(CH_IDENTITY, string.Format("ONE LIFE ENFORCEMENT WAIVED by %1 -- SAFE_START/LIVE may now be entered on a host with no durable player identity. Deaths will NOT survive a reconnect.",
			byAdmin), true);
		return true;
	}

	//! Revoke the waiver. Always allowed.
	//! @authority server
	void RequireDurableIdentity(string byAdmin)
	{
		m_bIdentityOverride = false;
		m_sIdentityOverrideBy = string.Empty;
		Print(string.Format("[TBD][%1] one-life identity enforcement RE-ARMED by %2 -- SAFE_START/LIVE are refused again while any connected player is on a NUMERIC key.",
			CH_IDENTITY, byAdmin));
	}

	//! One greppable line answering whether this host can run a real event.
	//! @authority server
	string IdentityStatusLine()
	{
		int connected;
		int nameHash;
		array<int> numericPlayers = {};
		int numeric = CensusIdentityModes(connected, nameHash, numericPlayers);

		string oneLife = "OFF";
		if (m_Spawn.IsOneLife())
			oneLife = "ON";

		string gate = "OPEN";
		if (numeric > 0 && m_Spawn.IsOneLife() && !m_bIdentityOverride)
			gate = "BLOCKED";

		string waiver = "none";
		if (m_bIdentityOverride)
			waiver = m_sIdentityOverrideBy;

		string line = string.Format("TBD identity: oneLife=%1 gate=%2 connected=%3", oneLife, gate, connected);
		line = line + string.Format(" backend=%1 nameHash=%2 numeric=%3", connected - nameHash - numeric, nameHash, numeric);
		line = line + string.Format(" waiver=%1 (%2)", waiver, FormatPlayerIdList(numericPlayers));

		if (gate == "BLOCKED")
			line = line + string.Format(" -- SAFE_START/LIVE refused; fix the server's backend identity, or '#tbd identity override %1'.", TBD_SpawnManager.IDENTITY_OVERRIDE_PHRASE);

		return line;
	}

	//! Report, once per session, a join on a NUMERIC key after the gate already ran. Log only: the
	//! deploy boundary stays the one place a deploy is refused.
	//! @authority server
	void NoteLateNonDurableJoin(int playerId)
	{
		if (m_bLateNonDurableJoinWarned)
			return;
		m_bLateNonDurableJoinWarned = true;

		TBD_Log.Banner(CH_IDENTITY, string.Format("player=%1 joined at stage %2 on a NUMERIC key -- the identity gate ran before they connected, so it could not see them. ONE LIFE IS NOT ENFORCEABLE for this player.",
			playerId, typename.EnumToString(TBD_EGameStage, m_Spawn.GetStage())), true);
	}
}
