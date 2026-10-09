/**
 * @file TBD_WaypointFactory.c
 * @brief Spawns the ScenarioFramework waypoint entities for one squad's authored waypoints.
 *
 * Role: maps each `$defs/waypoint` onto a ScenarioFramework waypoint prefab and applies its
 * completion radius, completion type and movement speed: `type` selects the prefab (hold and
 * sentry defend; an unknown type moves); `x`/`z`/optional `y` place it (surface height when `y`
 * is absent); `radiusM` sets the completion radius; boarding types and `behaviour` select the
 * completion type (boarding and combat wait for all, careless and stealth for any); `speedMode`,
 * else the `behaviour` ceiling, adds a waypoint-origin movement speed setting; `vehicleUid` on
 * get_in / get_out moves the waypoint onto the roster vehicle and binds it with
 * `SCR_EntityWaypoint.SetEntity`, else boards by proximity; `cycle` wraps the other waypoints in
 * one `AIWaypointCycle` that reruns forever.  Position: called by `TBD_WaypointRuntime.ArmSquad`
 * on the server; logs on `TBD_WaypointRuntime.CH`.
 * State: none.  Invariants: waypoints are issued in document order; the prefab ResourceNames are
 * the ScenarioFramework defaults; the description, combat-mode and formation of Eden's waypoint
 * attributes are not on the wire (the last two are group attributes, `TBD_GroupState`).
 */

//! Stateless waypoint spawning for `TBD_WaypointRuntime`.
class TBD_WaypointFactory
{
	static const string TYPE_MOVE = "move"; //!< wire type `move`
	static const string TYPE_ATTACK = "attack"; //!< wire type `attack`
	static const string TYPE_DEFEND = "defend"; //!< wire type `defend`
	static const string TYPE_PATROL = "patrol"; //!< wire type `patrol`
	static const string TYPE_CYCLE = "cycle"; //!< wire type `cycle`: wraps the other waypoints
	static const string TYPE_HOLD = "hold"; //!< wire type `hold`: a defend waypoint
	static const string TYPE_GET_IN = "get_in"; //!< wire type `get_in`: boarding
	static const string TYPE_GET_OUT = "get_out"; //!< wire type `get_out`: boarding
	static const string TYPE_SAD = "seek_and_destroy"; //!< wire type `seek_and_destroy`
	static const string TYPE_SENTRY = "sentry"; //!< wire type `sentry`: a defend waypoint
	static const ResourceName PREFAB_MOVE = "{750A8D1695BD6998}Prefabs/AI/Waypoints/AIWaypoint_Move.et"; //!< ScenarioFramework move waypoint; also the unknown-type fallback
	static const ResourceName PREFAB_ATTACK = "{1B0E3436C30FA211}Prefabs/AI/Waypoints/AIWaypoint_Attack.et"; //!< ScenarioFramework attack waypoint
	static const ResourceName PREFAB_DEFEND = "{93291E72AC23930F}Prefabs/AI/Waypoints/AIWaypoint_Defend.et"; //!< ScenarioFramework defend waypoint (defend, hold, sentry)
	static const ResourceName PREFAB_PATROL = "{22A875E30470BD4F}Prefabs/AI/Waypoints/AIWaypoint_Patrol.et"; //!< ScenarioFramework patrol waypoint
	static const ResourceName PREFAB_CYCLE = "{35BD6541CBB8AC08}Prefabs/AI/Waypoints/AIWaypoint_Cycle.et"; //!< ScenarioFramework cycle waypoint
	static const ResourceName PREFAB_GET_IN = "{712F4795CF8B91C7}Prefabs/AI/Waypoints/AIWaypoint_GetIn.et"; //!< ScenarioFramework get-in waypoint
	static const ResourceName PREFAB_GET_OUT = "{C40316EE26846CAB}Prefabs/AI/Waypoints/AIWaypoint_GetOut.et"; //!< ScenarioFramework get-out waypoint
	static const ResourceName PREFAB_SAD = "{B3E7B8DC2BAB8ACC}Prefabs/AI/Waypoints/AIWaypoint_SearchAndDestroy.et"; //!< ScenarioFramework search-and-destroy waypoint

	protected static const float VEHICLE_XZ_M = 5; //!< metres: half width of the vehicle lookup box around the roster x/z
	protected static const float VEHICLE_Y_M = 300; //!< metres: half height of the vehicle lookup box around Y = 0

	//! Spawn the squad's waypoints in document order and add them to `group`; a `cycle` entry
	//! instead wraps every other waypoint in one cycle waypoint that reruns forever.
	//! @return false when nothing was issued (no waypoint spawned, or a cycle with nothing to wrap)
	//! @authority server
	static bool IssueWaypoints(SCR_AIGroup group, TBD_WaypointSquad squad)
	{
		array<AIWaypoint> issued = new array<AIWaypoint>();
		AIWaypointCycle cycleWp = null;

		foreach (int index, TBD_WaypointWireStruct wire : squad.waypoints)
		{
			if (!wire)
			{
				TBD_Log.Warn(TBD_WaypointRuntime.CH, string.Format("%1:%2 waypoints[%3] is null - skipped",
					squad.faction, squad.callsign, index));
				continue;
			}

			if (wire.type == TYPE_CYCLE)
			{
				if (!cycleWp)
					cycleWp = SpawnCycleWaypoint(wire);
				continue;
			}

			AIWaypoint wp = SpawnOneWaypoint(wire, index);
			if (!wp)
				continue;

			issued.Insert(wp);
		}

		if (cycleWp)
		{
			if (issued.Count() < 1)
			{
				TBD_Log.Warn(TBD_WaypointRuntime.CH, string.Format("%1:%2 type=cycle has no sibling waypoints to wrap",
					squad.faction, squad.callsign));
				return false;
			}

			cycleWp.SetWaypoints(issued);
			cycleWp.SetRerunCounter(-1);
			group.AddWaypoint(cycleWp);
			return true;
		}

		if (issued.Count() < 1)
			return false;

		foreach (AIWaypoint wp : issued)
		{
			if (wp)
				group.AddWaypoint(wp);
		}

		return true;
	}

	//! @return the cycle waypoint spawned at `wire`'s position, or null
	protected static AIWaypointCycle SpawnCycleWaypoint(TBD_WaypointWireStruct wire)
	{
		AIWaypoint wp = SpawnPrefabAt(PREFAB_CYCLE, wire);
		return AIWaypointCycle.Cast(wp);
	}

	//! Spawn one non-cycle waypoint: its type's prefab (move for an unknown type), on the roster
	//! vehicle for a boarding type with a resolvable `vehicleUid`, with completion and speed applied.
	//! @param index the waypoint's array index, named on warnings
	//! @return the waypoint, or null when its prefab does not spawn
	protected static AIWaypoint SpawnOneWaypoint(TBD_WaypointWireStruct wire, int index)
	{
		ResourceName prefab = PrefabForType(wire.type);
		if (prefab.IsEmpty())
		{
			TBD_Log.Warn(TBD_WaypointRuntime.CH, string.Format("waypoints[%1] type='%2' is not a known waypoint kind - using move",
				index, wire.type));
			prefab = PREFAB_MOVE;
		}

		ref TBD_WaypointWireStruct at = wire;
		IEntity vehicle;
		if (IsBoardingType(wire.type) && !wire.vehicleUid.IsEmpty())
		{
			vehicle = FindVehicleByUid(wire.vehicleUid);
			if (vehicle)
			{
				// Sit the boarding waypoint on the authored vehicle.
				vector o = vehicle.GetOrigin();
				ref TBD_WaypointWireStruct attached = new TBD_WaypointWireStruct();
				attached.type = wire.type;
				attached.x = o[0];
				attached.y = o[1];
				attached.z = o[2];
				attached.vehicleUid = wire.vehicleUid;
				attached.radiusM = wire.radiusM;
				attached.behaviour = wire.behaviour;
				attached.speedMode = wire.speedMode;
				at = attached;
			}
			else
			{
				TBD_Log.Warn(TBD_WaypointRuntime.CH, string.Format("waypoints[%1] vehicleUid='%2' did not resolve - boarding by proximity at authored x/z",
					index, wire.vehicleUid));
			}
		}

		AIWaypoint wp = SpawnPrefabAt(prefab, at);
		if (!wp)
			return null;

		ApplyCompletion(wp, wire);
		ApplySpeedAndBehaviour(wp, wire);

		if (vehicle)
		{
			SCR_EntityWaypoint attached = SCR_EntityWaypoint.Cast(wp);
			if (attached)
				attached.SetEntity(vehicle);
		}

		return wp;
	}

	//! @return true for `get_in` and `get_out`
	protected static bool IsBoardingType(string type)
	{
		if (type == TYPE_GET_IN)
			return true;
		if (type == TYPE_GET_OUT)
			return true;
		return false;
	}

	//! @return the waypoint prefab for a wire type, or empty for an unknown type
	protected static ResourceName PrefabForType(string type)
	{
		if (type == TYPE_MOVE)
			return PREFAB_MOVE;
		if (type == TYPE_ATTACK)
			return PREFAB_ATTACK;
		if (type == TYPE_DEFEND)
			return PREFAB_DEFEND;
		if (type == TYPE_HOLD)
			return PREFAB_DEFEND;
		if (type == TYPE_SENTRY)
			return PREFAB_DEFEND;
		if (type == TYPE_PATROL)
			return PREFAB_PATROL;
		if (type == TYPE_GET_IN)
			return PREFAB_GET_IN;
		if (type == TYPE_GET_OUT)
			return PREFAB_GET_OUT;
		if (type == TYPE_SAD)
			return PREFAB_SAD;
		if (type == TYPE_CYCLE)
			return PREFAB_CYCLE;
		return ResourceName.Empty;
	}

	//! Spawn a waypoint prefab at the wire's x/z and authored `y`, else the surface height.
	//! @return the waypoint, or null when the prefab does not load or is not an `AIWaypoint`
	protected static AIWaypoint SpawnPrefabAt(ResourceName prefab, TBD_WaypointWireStruct wire)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			TBD_Log.Error(TBD_WaypointRuntime.CH, "waypoint prefab failed to load: " + prefab);
			return null;
		}

		float x = wire.x;
		float z = wire.z;
		float spawnY = GetGame().GetWorld().GetSurfaceY(x, z);
		if (wire.HasJsonY())
			spawnY = wire.y;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = Vector(x, spawnY, z);

		IEntity ent = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		return AIWaypoint.Cast(ent);
	}

	//! Set the completion radius when authored, and the completion type: all for boarding types and
	//! `combat`, any for `careless`, `stealth` and every other waypoint.
	protected static void ApplyCompletion(AIWaypoint wp, TBD_WaypointWireStruct wire)
	{
		if (wire.HasRadius())
			wp.SetCompletionRadius(wire.radiusM);

		EAIWaypointCompletionType completion = EAIWaypointCompletionType.Any;
		if (IsBoardingType(wire.type))
			completion = EAIWaypointCompletionType.All;

		if (wire.behaviour == TBD_AIWireEnums.BEHAVIOUR_COMBAT)
			completion = EAIWaypointCompletionType.All;
		if (wire.behaviour == TBD_AIWireEnums.BEHAVIOUR_CARELESS)
			completion = EAIWaypointCompletionType.Any;
		if (wire.behaviour == TBD_AIWireEnums.BEHAVIOUR_STEALTH)
			completion = EAIWaypointCompletionType.Any;

		wp.SetCompletionType(completion);
	}

	//! Add a waypoint-origin movement speed setting from `speedMode`, else the `behaviour` ceiling;
	//! nothing when neither selects a speed or the waypoint is not scripted.
	protected static void ApplySpeedAndBehaviour(AIWaypoint wp, TBD_WaypointWireStruct wire)
	{
		SCR_AIWaypoint scripted = SCR_AIWaypoint.Cast(wp);
		if (!scripted)
			return;

		EMovementType speed;
		bool haveSpeed = TBD_AIWireEnums.SpeedFromWire(wire.speedMode, wire.behaviour, speed);
		if (!haveSpeed)
			return;

		SCR_AIGroupCharactersMovementSpeedSetting setting = SCR_AIGroupCharactersMovementSpeedSetting.Create(
			SCR_EAISettingOrigin.WAYPOINT, speed);
		if (setting)
			scripted.AddSetting(setting);
	}

	//! The world vehicle of the roster row with `uid`: the first vehicle within `VEHICLE_XZ_M` of the
	//! row's authored x/z.
	//! @return the vehicle, or null for an empty or unknown uid or no vehicle there
	protected static IEntity FindVehicleByUid(string uid)
	{
		if (uid.IsEmpty())
			return null;

		array<ref TBD_MissionVehicleStruct> roster = TBD_MissionLoader.GetVehicles();
		if (!roster)
			return null;

		TBD_MissionVehicleStruct found;
		foreach (TBD_MissionVehicleStruct veh : roster)
		{
			if (!veh)
				continue;
			if (veh.uid == uid)
			{
				found = veh;
				break;
			}
		}

		if (!found)
			return null;

		return TBD_EntityQuery.FirstVehicleNear(found.x, found.z, VEHICLE_XZ_M, VEHICLE_Y_M);
	}
}
