/**
 * @file TBD_AudioSourceEntity.c
 * @brief Client-side positional sound source for one authored audio emitter.
 *
 * Role: plays an emitter's sound event while the local listener stands inside its radius.
 * Position: spawned by typename (no prefab) by `TBD_AudioLocalSources.Spawn`; configured once.
 * State: per-entity play gate (one-shot flag, next loop time); lives on the client that spawned it.
 * Invariants: `SCR_UISoundEntity` plays 2D, so the radius gate is what makes the source positional;
 * a zero radius or empty sound never plays.
 */

//! Entity class of `TBD_AudioSourceEntity`; carries no data.
[EntityEditorProps(category: "TBD/Gamemode", description: "TBD positional audio source")]
class TBD_AudioSourceEntityClass : GenericEntityClass
{
};

//! Local client sound source gated by listener distance.
//! @authority client
class TBD_AudioSourceEntity : GenericEntity
{
	static const int LOOP_MS = 4000; //!< milliseconds between two plays of a looping sound

	string m_sId; //!< emitter id; `TBD_AudioLocalSources` dedupes on it
	string m_sSound; //!< `SCR_SoundEvent` name; empty never plays
	float m_fRadiusM; //!< listener radius in metres; 0 never plays
	bool m_bLoop; //!< true repeats every `LOOP_MS` inside the radius
	bool m_bOneShotPlayed; //!< set after a one-shot plays; default false
	float m_fNextPlayMs; //!< world time in ms of the next loop play; default 0

	//! Subscribe to frame events so `EOnFrame` runs.
	void TBD_AudioSourceEntity(IEntitySource src, IEntity parent)
	{
		SetEventMask(EntityEvent.FRAME);
		SetFlags(EntityFlags.ACTIVE, true);
	}

	//! Set the sound, radius and loop mode and reset the play gate.
	//! @param id emitter id
	//! @param sound `SCR_SoundEvent` name
	//! @param radiusM listener radius in metres
	//! @param loop true to repeat inside the radius
	void Configure(string id, string sound, float radiusM, bool loop)
	{
		m_sId = id;
		m_sSound = sound;
		m_fRadiusM = radiusM;
		m_bLoop = loop;
		m_bOneShotPlayed = false;
		m_fNextPlayMs = 0;
	}

	//! Play the sound when the listener is inside the radius: every `LOOP_MS` when looping, once
	//! otherwise. Does nothing without a sound, a radius or a controlled listener entity.
	//! @authority client
	override protected void EOnFrame(IEntity owner, float timeSlice)
	{
		if (m_sSound.IsEmpty() || m_fRadiusM <= 0)
			return;

		IEntity listener = ListenerEntity();
		if (!listener)
			return;

		float dist = vector.Distance(listener.GetOrigin(), GetOrigin());
		bool inRange = dist <= m_fRadiusM;
		if (!inRange)
			return;

		float now = GetGame().GetWorld().GetWorldTime();
		if (m_bLoop)
		{
			if (now >= m_fNextPlayMs)
			{
				SCR_UISoundEntity.SoundEvent(m_sSound);
				m_fNextPlayMs = now + LOOP_MS;
			}
			return;
		}

		if (m_bOneShotPlayed)
			return;

		SCR_UISoundEntity.SoundEvent(m_sSound);
		m_bOneShotPlayed = true;
	}

	//! The local player's controlled entity.
	//! @return the entity, or null without a player controller or a controlled entity
	protected IEntity ListenerEntity()
	{
		PlayerController pc = GetGame().GetPlayerController();
		if (!pc)
			return null;
		return pc.GetControlledEntity();
	}
}
