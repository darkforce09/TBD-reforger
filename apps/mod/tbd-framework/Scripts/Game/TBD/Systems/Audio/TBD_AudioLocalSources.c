/**
 * @file TBD_AudioLocalSources.c
 * @brief The audio sources this client has spawned, one per armed emitter.
 *
 * Role: spawns and deletes the client's `TBD_AudioSourceEntity` instances.  Position: fed by the
 * modded `SCR_PlayerController` audio RPC (and in place on a listen host); cleared by
 * `TBD_AudioEmitter.Clear` at each world start and mission change.
 * State: `s_aLocalSources`, static, on the client.  Invariants: at most one source per emitter id;
 * a source without an authored `y` stands on the terrain surface.
 */

//! Registry of this machine's spawned audio sources.
class TBD_AudioLocalSources
{
	protected static ref array<TBD_AudioSourceEntity> s_aLocalSources; //!< spawned sources; null until the first spawn

	//! Delete every spawned source and forget them.
	static void Clear()
	{
		if (s_aLocalSources)
		{
			foreach (TBD_AudioSourceEntity src : s_aLocalSources)
			{
				if (src)
					SCR_EntityHelper.DeleteEntityAndChildren(src);
			}
		}

		s_aLocalSources = null;
	}

	//! Spawn one source for an emitter at (x, z), unless one with the same id exists.
	//! @param id emitter id; empty spawns nothing
	//! @param x world X in metres
	//! @param y metres above sea level, or `TBD_AudioEmitter.ABSENT` for the terrain surface
	//! @param z world Z in metres
	//! @param radiusM listener radius in metres
	//! @param loop true to repeat inside the radius
	//! @param sound `SCR_SoundEvent` name; empty spawns nothing
	//! Logs a warning and spawns nothing when the entity cannot be created.
	//! @authority client
	static void Spawn(string id, float x, float y, float z, float radiusM, bool loop, string sound)
	{
		if (id.IsEmpty() || sound.IsEmpty())
			return;

		if (!s_aLocalSources)
			s_aLocalSources = new array<TBD_AudioSourceEntity>();

		foreach (TBD_AudioSourceEntity existing : s_aLocalSources)
		{
			if (existing && existing.m_sId == id)
				return;
		}

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return;

		float spawnY = y;
		if (y == TBD_AudioEmitter.ABSENT)
			spawnY = world.GetSurfaceY(x, z);

		vector pos = Vector(x, spawnY, z);
		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;

		IEntity spawned = GetGame().SpawnEntity(TBD_AudioSourceEntity, world, params);
		TBD_AudioSourceEntity src = TBD_AudioSourceEntity.Cast(spawned);
		if (!src)
		{
			TBD_Log.Warn(TBD_AudioEmitter.CH, string.Format("could not spawn source id='%1'", id));
			if (spawned)
				SCR_EntityHelper.DeleteEntityAndChildren(spawned);
			return;
		}

		src.Configure(id, sound, radiusM, loop);
		s_aLocalSources.Insert(src);
		TBD_Log.Kv(TBD_AudioEmitter.CH, "source", string.Format("id='%1' radiusM=%2 loop=%3", id, radiusM, loop));
	}
}
