/**
 * @file TBD_SpectatorHostFactory.c
 * @brief Builds and vets the inert entity a dead player possesses as a spectator streaming host.
 *
 * Role: spawns a host by type name (the default) or from the configured prefab, neutralises its
 * physics, refuses any candidate that could be a body, and reports whether a host replicates.
 * Position: TBD_SpectatorHost.Start and Shutdown configure and reset it;
 * TBD_SpectatorHostLifecycle.EnsureHost calls SpawnHostEntity, IsAcceptableHost and IsReplicated,
 * and DropConfiguredPrefab after a refused prefab.
 * State: the configured prefab and its load-failure log latch (static, server).
 * Invariants: a candidate that is a ChimeraCharacter or carries a DamageManagerComponent or a
 * CharacterControllerComponent is refused, so a possessed host can never be damaged, destroyed,
 * healed or stood up as a body; a prefab that fails to load or spawn falls back to the
 * prefab-free host and is reported once; the factory never possesses anything.
 */

//! Server factory for spectator streaming host entities. Static.
class TBD_SpectatorHostFactory
{
	protected static ResourceName s_sHostPrefab; //!< configured host prefab; empty = the prefab-free host
	protected static bool s_bPrefabFailureLogged; //!< latch: one prefab load or spawn failure line per round; default false

	//! Store the operator's host prefab and clear the failure latch. TBD_SpectatorHost.Start calls it.
	//! @param hostPrefab optional prefab whose root class is TBD_SpectatorHostEntity; empty = prefab-free
	static void Configure(ResourceName hostPrefab)
	{
		s_sHostPrefab = hostPrefab;
		s_bPrefabFailureLogged = false;
	}

	//! Forget the configured prefab and clear the latch. TBD_SpectatorHost.Shutdown calls it.
	static void Reset()
	{
		s_sHostPrefab = string.Empty;
		s_bPrefabFailureLogged = false;
	}

	//! Drop a configured prefab whose candidate was refused, with one ERROR line, so the
	//! prefab-free host is used for the rest of the round.
	//! @authority server
	static void DropConfiguredPrefab()
	{
		Print(string.Format("[TBD][spectator] ignoring m_sHostPrefab %1 for the rest of this round -- falling back to the built-in prefab-free host", s_sHostPrefab), LogLevel.ERROR);
		s_sHostPrefab = string.Empty;
	}

	//! Spawn a host candidate at a position. The default route spawns TBD_SpectatorHostEntity by
	//! type name, with no prefab and so no resourceDatabase.rdb dependency (the route
	//! TBD_SpectatorCamera uses). Such an entity has no RplComponent and exists on the server only;
	//! a configured prefab whose root class is TBD_SpectatorHostEntity is the route to a replicated
	//! host, and it falls back to the type-name route when it will not load or spawn.
	//! @param position the anchor position, world metres
	//! @param fromPrefab receives true when the configured prefab produced the entity, so a refused
	//! candidate drops the prefab instead of standing the feature down
	//! @return the spawned candidate, or null when there is no world or nothing spawned
	//! @authority server
	static IEntity SpawnHostEntity(vector position, out bool fromPrefab)
	{
		fromPrefab = false;

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = position;

		if (!s_sHostPrefab.IsEmpty())
		{
			IEntity configured = SpawnHostPrefab(params, world);
			if (configured)
			{
				fromPrefab = true;
				return configured;
			}
		}

		IEntity spawned = GetGame().SpawnEntity(TBD_SpectatorHostEntity, world, params);
		if (!spawned)
			return null;

		NeutralisePhysics(spawned);
		return spawned;
	}

	//! The configured-prefab route. Returns null, having logged why once per round, so the caller
	//! falls back to the prefab-free host.
	//! @param params spawn parameters carrying the world transform
	//! @param world the world to spawn into
	//! @return the spawned entity with physics neutralised, or null
	//! @authority server
	protected static IEntity SpawnHostPrefab(notnull EntitySpawnParams params, notnull BaseWorld world)
	{
		Resource resource = Resource.Load(s_sHostPrefab);
		if (!resource || !resource.IsValid())
		{
			if (!s_bPrefabFailureLogged)
			{
				s_bPrefabFailureLogged = true;
				Print(string.Format("[TBD][spectator] streaming host prefab %1 will not load (missing, or not in resourceDatabase.rdb) -- falling back to the prefab-free host", s_sHostPrefab), LogLevel.WARNING);
			}

			return null;
		}

		IEntity spawned = GetGame().SpawnEntityPrefab(resource, world, params);
		if (!spawned)
		{
			if (!s_bPrefabFailureLogged)
			{
				s_bPrefabFailureLogged = true;
				Print(string.Format("[TBD][spectator] streaming host prefab %1 loaded but failed to spawn -- falling back to the prefab-free host", s_sHostPrefab), LogLevel.WARNING);
			}

			return null;
		}

		NeutralisePhysics(spawned);
		return spawned;
	}

	//! One life, enforced on the entity rather than on the paths that reach it: refuse a candidate
	//! that is a ChimeraCharacter, carries a DamageManagerComponent, or carries a
	//! CharacterControllerComponent. An entity a dead player possesses that can be killed is a
	//! second death path, and one that can be healed or stood up is a respawn; this also stops a
	//! character prefab in m_sHostPrefab from becoming a second door into the world. With no damage
	//! manager the host is also inert through the safestart hold: TBD_SafestartProtection.RestoreOne
	//! takes its no-damage-manager early return.
	//! @param host the spawned candidate
	//! @param playerId the player it is for, written into the refusal line
	//! @return true when acceptable; false after two ERROR lines naming the reason
	//! @authority server
	static bool IsAcceptableHost(notnull IEntity host, int playerId)
	{
		string refusal;

		if (ChimeraCharacter.Cast(host))
			refusal = "it is a ChimeraCharacter -- a character can be killed, and a killed character spends a life";
		else if (host.FindComponent(DamageManagerComponent))
			refusal = "it carries a DamageManagerComponent -- a dead player would be possessing something that can be damaged, destroyed or healed, which is a second death path under ONE LIFE";
		else if (host.FindComponent(CharacterControllerComponent))
			refusal = "it carries a CharacterControllerComponent -- that is a playable body, not an anchor";

		if (refusal.IsEmpty())
			return true;

		Print(string.Format("[TBD][spectator] player=%1 streaming host candidate REFUSED: %2", playerId, refusal), LogLevel.ERROR);
		Print("[TBD][spectator] the spectator streaming host must be an inert entity. Check m_sHostPrefab on TBD_SpectatorComponent; leave it EMPTY for the built-in prefab-free host.", LogLevel.ERROR);
		return false;
	}

	//! Stop a host from falling, drifting or pushing anybody: no gravity, zero mass, no simulation,
	//! the CharNoCollide layer. A no-op for the prefab-free host, which has no Physics; it matters
	//! on the configured-prefab route. There is no damage half, because a host with a damage manager
	//! is refused rather than pacified.
	//! @param host the spawned entity
	//! @authority server
	protected static void NeutralisePhysics(notnull IEntity host)
	{
		Physics physics = host.GetPhysics();
		if (!physics)
			return;

		physics.EnableGravity(false);
		physics.SetMass(0);
		physics.ChangeSimulationState(SimulationState.NONE);
		physics.SetInteractionLayer(EPhysicsLayerDefs.CharNoCollide);
	}

	//! Is this host visible to the network at all? The ISSUED line logs the answer, which tells an
	//! operator whether the prefab-free (server-only) host is in use or a replicated prefab host.
	//! @param host the issued host
	//! @return true when it carries an RplComponent with a valid id
	static bool IsReplicated(notnull IEntity host)
	{
		RplComponent rpl = RplComponent.Cast(host.FindComponent(RplComponent));
		if (!rpl)
			return false;

		return rpl.Id().IsValid();
	}
}
