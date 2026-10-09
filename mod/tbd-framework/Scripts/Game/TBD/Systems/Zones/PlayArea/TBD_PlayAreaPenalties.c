/**
 * @file TBD_PlayAreaPenalties.c
 * @brief What a player in violation is told, and what happens when their grace runs out.
 *
 * Role: the warning message with its countdown, and the `none`, `warn` and `kill` penalties.
 * Position: called by `TBD_PlayAreaComponent.AccumulateViolation`; tells players through
 * `TBD_PlayerChat` and ends a character through `SCR_CharacterDamageManagerComponent.Kill`.
 * State: none.  Invariants: `none` never messages the player; `kill` goes through the engine's own
 * kill, self-instigated, and refuses a body without a damage manager rather than improvise.
 */

//! Play-area warnings and penalties.
//! @authority server
class TBD_PlayAreaPenalties
{
	//! Tell the player they are outside the play area or inside a protected area, which zone, and
	//! the whole seconds left (plus the one-life notice under `kill`); marks the row warned. Says
	//! nothing under `none`. The text is built in steps, since a long `+` chain is `Formula too
	//! complex`.
	//! @param playerId the player
	//! @param state the player's violation row
	//! @param zone the violated zone
	//! @param secondsRemaining grace left in seconds
	static void WarnPlayer(int playerId, notnull TBD_PlayAreaViolation state, notnull TBD_Zone zone, float secondsRemaining)
	{
		if (zone.m_ePenalty == TBD_EZonePenalty.NONE)
			return;

		state.m_bWarned = true;

		int whole = Math.Round(secondsRemaining);
		if (whole < 0)
			whole = 0;

		string what = "outside the play area";
		if (zone.m_sType == TBD_ZoneRegistry.TYPE_BASE_PROTECTION)
			what = "inside a protected area";

		string msg = "TBD: you are ";
		msg += what;
		msg += " (";
		msg += zone.DisplayName();
		msg += ") -- return within ";
		msg += whole.ToString();
		msg += "s.";

		if (zone.m_ePenalty == TBD_EZonePenalty.KILL)
			msg += " ONE LIFE: you will be killed and cannot respawn.";

		TBD_PlayerChat.Tell(playerId, msg);
	}

	//! Apply the zone's penalty once the grace expired: `none` logs, `warn` logs and tells the
	//! player again, `kill` ends the character.
	//! @param playerId the player
	//! @param body the player's controlled entity
	//! @param zone the violated zone
	static void ApplyPenalty(int playerId, notnull IEntity body, notnull TBD_Zone zone)
	{
		if (zone.m_ePenalty == TBD_EZonePenalty.NONE)
		{
			TBD_Log.Kv(TBD_ZoneRegistry.CH, "expired", string.Format("player=%1 zone=%2 penalty=none -- logged only",
				playerId, zone.LogKey()));
			return;
		}

		if (zone.m_ePenalty == TBD_EZonePenalty.WARN)
		{
			TBD_Log.Warn(TBD_ZoneRegistry.CH, string.Format("player=%1 zone=%2 grace expired -- penalty=warn, no action taken",
				playerId, zone.LogKey()));
			TBD_PlayerChat.Tell(playerId, "TBD: you are still out of the play area. Return to the AO.");
			return;
		}

		KillForViolation(playerId, body, zone);
	}

	//! End the character under one life through the engine's own
	//! `SCR_CharacterDamageManagerComponent.Kill`, self-instigated, so the death reaches
	//! `TBD_SpawnManager.OnPlayerKilled` like any other; only an admin `#tbd respawn` brings the
	//! player back. A body with no damage manager is logged and not killed.
	//! @param playerId the player
	//! @param body the player's controlled entity
	//! @param zone the violated zone
	protected static void KillForViolation(int playerId, notnull IEntity body, notnull TBD_Zone zone)
	{
		SCR_CharacterDamageManagerComponent damage = SCR_CharacterDamageManagerComponent.Cast(
			body.FindComponent(SCR_CharacterDamageManagerComponent));

		if (!damage)
		{
			TBD_Log.Error(TBD_ZoneRegistry.CH, string.Format("player=%1 zone=%2 penalty=kill but the body has no SCR_CharacterDamageManagerComponent -- NOT killed",
				playerId, zone.LogKey()));
			return;
		}

		TBD_Log.Banner(TBD_ZoneRegistry.CH, string.Format(
			"ONE LIFE SPENT: player=%1 killed for leaving %2 (grace %3s expired) -- admin '#tbd respawn %1' is the only way back",
			playerId, zone.LogKey(), zone.m_fGraceSeconds), true);

		TBD_PlayerChat.Tell(playerId, "TBD: you left the play area. Your one life is spent -- contact an admin.");

		damage.Kill(Instigator.CreateInstigator(body));
	}
}
