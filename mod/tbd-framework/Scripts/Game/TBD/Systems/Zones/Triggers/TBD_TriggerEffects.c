/**
 * @file TBD_TriggerEffects.c
 * @brief Dispatches a fired trigger's effects and runs the player-facing ones.
 *
 * Role: routes each usable effect to its runner, and runs `hint` and `play_sound` for the audience
 * `params.audience` selects.  Position: called by `TBD_TriggerRuntime.Fire`; hands world effects
 * to `TBD_TriggerWorldEffects`, round-flow effects to `TBD_TriggerFlowEffects`, sounds to
 * `TBD_TriggerSoundRelay` and hints to `TBD_PlayerChat`.
 * State: none.  Invariants: every effect writes one log line naming the trigger; an `owner` or
 * `enemy` audience without an owner side addresses nobody rather than everybody.
 */

//! Effect dispatch and the player-facing effects.
//! @authority server
class TBD_TriggerEffects
{
	//! Run one usable effect through its runner. An effect kind with no runner is logged as an
	//! error and does nothing.
	//! @param trigger the firing trigger
	//! @param effect the prepared effect
	//! @param index its position in `effects[]`, for the log
	static void Run(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect, int index)
	{
		if (effect.m_eKind == TBD_ETriggerEffect.HINT)
		{
			Hint(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.PLAY_SOUND)
		{
			PlaySound(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SPAWN)
		{
			TBD_TriggerWorldEffects.Spawn(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.DELETE)
		{
			TBD_TriggerWorldEffects.Delete(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_OBJECTIVE)
		{
			TBD_TriggerFlowEffects.SetObjective(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.SET_VARIANT)
		{
			TBD_TriggerFlowEffects.SetVariant(trigger, effect);
			return;
		}

		if (effect.m_eKind == TBD_ETriggerEffect.END_MISSION)
		{
			TBD_TriggerFlowEffects.EndMission(trigger, effect);
			return;
		}

		// The validator marks an unknown kind unusable; an enum entry with no runner still logs.
		TBD_Log.Error(TBD_TriggerRuntime.CH, string.Format("trigger '%1' effects[%2] has kind %3 with no runner - nothing happened",
			trigger.m_sId, index, typename.EnumToString(TBD_ETriggerEffect, effect.m_eKind)));
	}

	//! `hint`: one private chat line, `TBD: <text>`, to each player in the audience; the text is
	//! also logged so what players were told can be reconstructed.
	//! @param trigger the firing trigger
	//! @param effect the prepared `hint` effect
	protected static void Hint(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		if (!GetGame().GetPlayerManager())
			return;

		array<int> audience = ResolveAudience(trigger, effect.m_sAudience);
		string msg = "TBD: ";
		msg += effect.m_sText;

		int sent = 0;
		foreach (int playerId : audience)
		{
			if (TBD_PlayerChat.Tell(playerId, msg))
				sent++;
		}

		TBD_Log.Kv(TBD_TriggerRuntime.CH, "hint", string.Format("id=%1 audience='%2' players=%3 sent=%4 text='%5'",
			trigger.m_sId, effect.m_sAudience, audience.Count(), sent, effect.m_sText));
	}

	//! `play_sound`: raise the `SCR_SoundEvent` named by `params.sound` as a 2D UI sound on each
	//! client in the audience, through `TBD_TriggerSoundRelay`. An unknown event name raises
	//! nothing on the client, so the authored name is logged here.
	//! @param trigger the firing trigger
	//! @param effect the prepared `play_sound` effect
	protected static void PlaySound(notnull TBD_Trigger trigger, notnull TBD_TriggerEffect effect)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return;

		array<int> audience = ResolveAudience(trigger, effect.m_sAudience);

		int sent = TBD_TriggerSoundRelay.PushToAudience(players, audience, effect.m_sSound);

		TBD_Log.Kv(TBD_TriggerRuntime.CH, "playSound", string.Format("id=%1 sound='%2' audience='%3' players=%4 sent=%5",
			trigger.m_sId, effect.m_sSound, effect.m_sAudience, audience.Count(), sent));
	}

	//! The connected players `audience` selects, from the live player list so dead players are
	//! still told. Empty or `all` = everybody; `owner`/`enemy` = the activation's owner side or
	//! every other named side; anything else = that faction key.
	//! @param trigger the firing trigger, for its owner side and id
	//! @param audience the authored `params.audience`
	//! @return the selected player ids; empty when sides cannot be resolved (logged)
	protected static array<int> ResolveAudience(notnull TBD_Trigger trigger, string audience)
	{
		array<int> selected = new array<int>();

		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return selected;

		array<int> connected = new array<int>();
		players.GetPlayers(connected);

		if (audience.IsEmpty() || audience == TBD_TriggerVocabulary.AUD_ALL)
		{
			foreach (int everyone : connected)
			{
				selected.Insert(everyone);
			}

			return selected;
		}

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
		{
			TBD_Log.Warn(TBD_TriggerRuntime.CH, string.Format("trigger '%1' wants audience '%2' but there is no TBD_SpawnManager to resolve sides - nobody is addressed",
				trigger.m_sId, audience));
			return selected;
		}

		// owner and enemy are relative to the activation's side; without one, nobody is addressed.
		string wanted = audience;
		bool invert = false;
		if (audience == TBD_TriggerVocabulary.AUD_OWNER || audience == TBD_TriggerVocabulary.AUD_ENEMY)
		{
			if (trigger.m_sOwnerSide.IsEmpty())
			{
				TBD_Log.Warn(TBD_TriggerRuntime.CH, string.Format("trigger '%1' wants audience '%2' but authors no activation.ownerSide - nobody is addressed",
					trigger.m_sId, audience));
				return selected;
			}

			wanted = trigger.m_sOwnerSide;
			invert = audience == TBD_TriggerVocabulary.AUD_ENEMY;
		}

		foreach (int playerId : connected)
		{
			string factionKey = TBD_PlayerFaction.Of(spawn, playerId);

			if (invert)
			{
				if (!factionKey.IsEmpty() && factionKey != wanted)
					selected.Insert(playerId);
			}
			else if (factionKey == wanted)
			{
				selected.Insert(playerId);
			}
		}

		return selected;
	}
}
