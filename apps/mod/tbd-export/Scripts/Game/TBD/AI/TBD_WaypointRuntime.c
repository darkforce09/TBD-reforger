//! T-677 - waypoint runtime: ordered movement orders for waypointed groups.
//!
//! == What was missing ========================================================================
//! T-706 put `group.waypoints[]` on the wire. Nothing read it. `TBD_SpawnManager` deactivates
//! every slot body's AI agent at spawn, so a waypoint had no subject to command. This file is
//! the reader AND the runtime. T-678 (group combat / behaviour / formation / speed defaults)
//! ships AFTER this slice and is not implemented here.
//!
//! == Why a second JsonLoadContext pass =======================================================
//! `TBD_MissionOrbatGroupStruct` in Backend/TBD_MissionLoader.c declares no `waypoints` field.
//! Enfusion maps JSON keys onto NAMED class fields only - a key no class declares is invisible
//! at runtime, not rejected, not logged, simply absent. This file runs its own pass over
//! `TBD_MissionLoader.GetRawJson()` with a root that declares `orbat.*.groups[].waypoints` and
//! nothing else. Same pattern as `Objectives/TBD_ObjectiveRules.c` and
//! `Zones/TBD_TriggerRuntime.c`: the vocabulary stays next to the code that interprets it, and
//! Backend/TBD_MissionLoader.c stays out of this slice's owns list (T-682 is editing it).
//!
//! == Presence, and the nested-ref landmine ===================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key is
//! ABSENT. `if (group.waypoints)` is therefore NOT a presence test if that field were a nested
//! class; it is an ARRAY, so presence is a null-or-Count() test. Numeric fields that can be
//! authored as 0 (`radiusM`) carry an ABSENT sentinel. `y` uses the same Y_ABSENT as
//! `TBD_MissionSlotStruct`.
//!
//! == The nine ATTR-FIELD-WP semantics, as the T-706 wire actually carries them ===============
//! Eden listed nine attribute ids. T-706's `$defs/waypoint` is the contract this reader may
//! consume (this slice must not touch packages/tbd-schema):
//!   ATTR-FIELD-WP-TYPE       -> `type` (required). Mapped onto a ScenarioFramework waypoint prefab.
//!   ATTR-FIELD-WP-ORDER      -> array index. Waypoints are issued in document order.
//!   ATTR-FIELD-WP-POSITION   -> `x` / `z` / optional `y` (metres ASL; same policy as slot.y).
//!   ATTR-FIELD-WP-SPEED      -> `speedMode` limited|normal|full, applied as a waypoint movement
//!                               speed setting (SCR_AIGroupCharactersMovementSpeedSetting).
//!   ATTR-FIELD-WP-BEHAVIOUR  -> `behaviour` careless|safe|aware|combat|stealth. Applied as a
//!                               waypoint-scoped movement-speed ceiling when `speedMode` is
//!                               absent (the engine has no separate behaviour-setting class;
//!                               group-level behaviour is T-678). When both are authored,
//!                               `speedMode` wins for speed and behaviour still selects the
//!                               completion type (stealth/careless = Any, else All).
//!   ATTR-FIELD-WP-CONDITION  -> `radiusM` completion radius. Type also selects
//!                               EAIWaypointCompletionType (boarding = All, move-like = Any).
//!   ATTR-FIELD-WP-DESCRIPTION, ATTR-FIELD-WP-COMBAT-MODE, ATTR-FIELD-WP-FORMATION
//!                             are NOT on `$defs/waypoint`. Combat-mode and formation live on
//!                             the GROUP (T-678). Description has no wire key. They cannot be
//!                             invented here.
//!
//! == Interaction ids, runtime not editor =====================================================
//! RIGHT-MODE-004 / KEY-WP-001 / ACTION-WP-QUICK-001 are editor placement verbs. At runtime they
//! are the same object: the ordered `waypoints[]` array, issued in order.
//! CONN-WP-ATTACH-001: `vehicleUid` on get_in / get_out binds SCR_EntityWaypoint.SetEntity to
//! the authored roster vehicle (looked up by uid, then by world position at that uid's x/z).
//! Absent uid -> proximity: the waypoint is spawned at its own x/z.
//! CONN-WP-ACT-001: the compiled document has no trigger-to-waypoint link. Activation here is
//! the LIVE stage transition - the only activation the wire can express. Waypoints are not
//! issued during LOBBY / BRIEFING / SAFE_START.
//! CONN-RAND-START-001: type `cycle` wraps the non-cycle waypoints via AIWaypointCycle
//! (infinite rerun). The T-706 wire has no random-start flag, so authored order is kept.
//!
//! == AI spawn gate ===========================================================================
//! `TBD_SpawnManager.SpawnSlotBody` still parks every body (CRF DisableBodyAI) unless the seat
//! belongs to a waypointed group AND the round is already LIVE (a respawn of an AI seat). This
//! runtime then ActivateAI's unclaimed seats of waypointed groups at LIVE via
//! `SCR_AIGroup.AddAIEntityToGroup`. Claimed / possessed seats stay player-controlled.
//! Unwaypointed groups are never enabled here.
//!
//! == Prefabs =================================================================================
//! Resource names are ScenarioFramework defaults (SCR_ScenarioFrameworkWaypoint*.c and
//! SCR_ScenarioFrameworkSlotAI.c Group_Base), not guessed GUIDs.
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. It cannot run a round. Whether a waypointed group
//! actually walks its path on a dedicated server is a human checklist item.
//! @contract mission.schema.json#/$defs/waypoint

//------------------------------------------------------------------------------------------------
//! One `$defs/waypoint` object. Field names are the JSON keys.
class TBD_WaypointWireStruct
{
	static const float Y_ABSENT = -1000000;
	static const float RADIUS_ABSENT = -1;

	string type;        //!< Required. Schema enum: move|attack|defend|patrol|cycle|hold|get_in|get_out|seek_and_destroy|sentry.
	float x;            //!< World X metres. Required.
	float z;            //!< World Z metres. Required.
	float y = -1000000; //!< Optional ASL height. Y_ABSENT when the key was omitted.
	string vehicleUid;  //!< Optional. get_in / get_out target. Empty = proximity.
	float radiusM = -1; //!< Optional completion radius. RADIUS_ABSENT when omitted. 0 is authored.
	string behaviour;   //!< Optional. careless|safe|aware|combat|stealth.
	string speedMode;   //!< Optional. limited|normal|full.

	//------------------------------------------------------------------------------------------------
	bool HasJsonY()
	{
		return y != Y_ABSENT;
	}

	//------------------------------------------------------------------------------------------------
	bool HasRadius()
	{
		return radiusM != RADIUS_ABSENT;
	}
}

//------------------------------------------------------------------------------------------------
//! The group fields this pass needs. `callsign` is the join key onto flattened slots.
class TBD_WaypointGroupWireStruct
{
	string callsign;
	ref array<ref TBD_WaypointWireStruct> waypoints;
}

//------------------------------------------------------------------------------------------------
//! One orbat faction's groups.
class TBD_WaypointFactionWireStruct
{
	ref array<ref TBD_WaypointGroupWireStruct> groups;
}

//------------------------------------------------------------------------------------------------
//! Root of the second parse. Declares `orbat` and nothing else.
class TBD_WaypointDocStruct
{
	ref map<string, ref TBD_WaypointFactionWireStruct> orbat;
}

//------------------------------------------------------------------------------------------------
//! One waypointed squad at runtime: the wire plus the engine group we form for it.
class TBD_WaypointSquad
{
	string faction;
	string callsign;
	ref array<ref TBD_WaypointWireStruct> waypoints;
	SCR_AIGroup group;
	bool armed;
}

//------------------------------------------------------------------------------------------------
//! Server-side waypoint reader + commander.
class TBD_WaypointRuntime
{
	static const string CH = "Waypoint";
	static const int TICK_MS = 1000;

	static const string TYPE_MOVE     = "move";
	static const string TYPE_ATTACK   = "attack";
	static const string TYPE_DEFEND   = "defend";
	static const string TYPE_PATROL   = "patrol";
	static const string TYPE_CYCLE    = "cycle";
	static const string TYPE_HOLD     = "hold";
	static const string TYPE_GET_IN   = "get_in";
	static const string TYPE_GET_OUT  = "get_out";
	static const string TYPE_SAD      = "seek_and_destroy";
	static const string TYPE_SENTRY   = "sentry";

	static const string SPEED_LIMITED = "limited";
	static const string SPEED_NORMAL  = "normal";
	static const string SPEED_FULL    = "full";

	static const string BH_CARELESS = "careless";
	static const string BH_SAFE     = "safe";
	static const string BH_AWARE    = "aware";
	static const string BH_COMBAT   = "combat";
	static const string BH_STEALTH  = "stealth";

	//! ScenarioFramework defaults. GUID+path must match both trees (lockstep keeps literals).
	static const ResourceName PREFAB_GROUP   = "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et";
	static const ResourceName PREFAB_MOVE    = "{750A8D1695BD6998}Prefabs/AI/Waypoints/AIWaypoint_Move.et";
	static const ResourceName PREFAB_ATTACK  = "{1B0E3436C30FA211}Prefabs/AI/Waypoints/AIWaypoint_Attack.et";
	static const ResourceName PREFAB_DEFEND  = "{93291E72AC23930F}Prefabs/AI/Waypoints/AIWaypoint_Defend.et";
	static const ResourceName PREFAB_PATROL  = "{22A875E30470BD4F}Prefabs/AI/Waypoints/AIWaypoint_Patrol.et";
	static const ResourceName PREFAB_CYCLE   = "{35BD6541CBB8AC08}Prefabs/AI/Waypoints/AIWaypoint_Cycle.et";
	static const ResourceName PREFAB_GET_IN  = "{712F4795CF8B91C7}Prefabs/AI/Waypoints/AIWaypoint_GetIn.et";
	static const ResourceName PREFAB_GET_OUT = "{C40316EE26846CAB}Prefabs/AI/Waypoints/AIWaypoint_GetOut.et";
	static const ResourceName PREFAB_SAD     = "{B3E7B8DC2BAB8ACC}Prefabs/AI/Waypoints/AIWaypoint_SearchAndDestroy.et";

	protected static const float VEHICLE_XZ_M = 5;
	protected static const float VEHICLE_Y_M = 300;

	protected static ref array<ref TBD_WaypointSquad> s_aSquads;
	protected static bool s_bParsed;
	protected static string s_sParsedForMission;
	protected static bool s_bAnnounced;

	//! Scratch for the vehicle AABB query. Static because QueryEntitiesByAABB takes a function.
	protected static IEntity s_VehicleHit;
	protected static string s_VehicleUidWanted;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aSquads = null;
		s_bParsed = false;
		s_sParsedForMission = string.Empty;
		s_bAnnounced = false;
		s_VehicleHit = null;
		s_VehicleUidWanted = string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;

		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SquadKey(string faction, string callsign)
	{
		return faction + ":" + callsign;
	}

	//------------------------------------------------------------------------------------------------
	//! True when this ORBAT group authored at least one waypoint. Used by SpawnManager to open
	//! the AI gate for waypointed groups only.
	static bool GroupHasWaypoints(string faction, string callsign)
	{
		if (callsign.IsEmpty())
			return false;

		if (!EnsureParsed())
			return false;

		if (!s_aSquads)
			return false;

		string key = SquadKey(faction, callsign);
		foreach (TBD_WaypointSquad squad : s_aSquads)
		{
			if (!squad)
				continue;
			if (SquadKey(squad.faction, squad.callsign) == key)
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn-time gate. LIVE + waypointed -> leave AI able (respawn of an AI seat). Every other
	//! spawn still runs DisableBodyAI, including waypointed seats during lobby / safestart.
	static bool ShouldEnableAIAtSpawn(TBD_MissionSlotStruct slot)
	{
		if (!slot)
			return false;

		if (!GroupHasWaypoints(slot.faction, slot.groupCallsign))
			return false;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm)
			return false;

		return fm.GetStage() == TBD_EGameStage.LIVE;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool EnsureParsed()
	{
		string missionId = CurrentMissionId();
		if (s_bParsed && missionId == s_sParsedForMission)
			return true;

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
		{
			TBD_Log.Error(CH, "the mission document did not parse as JSON on the waypoint pass - no waypoint is armed this round");
			s_aSquads = new array<ref TBD_WaypointSquad>();
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		TBD_WaypointDocStruct doc = new TBD_WaypointDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			TBD_Log.Error(CH, "the mission document parsed but its root would not read on the waypoint pass - no waypoint is armed this round");
			s_aSquads = new array<ref TBD_WaypointSquad>();
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		s_aSquads = new array<ref TBD_WaypointSquad>();
		if (doc.orbat)
		{
			foreach (string factionKey, TBD_WaypointFactionWireStruct faction : doc.orbat)
			{
				CollectFaction(factionKey, faction);
			}
		}

		s_bParsed = true;
		s_sParsedForMission = missionId;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void CollectFaction(string factionKey, TBD_WaypointFactionWireStruct faction)
	{
		if (!faction || !faction.groups)
			return;

		foreach (TBD_WaypointGroupWireStruct group : faction.groups)
		{
			if (!group)
				continue;
			if (group.callsign.IsEmpty())
				continue;

			// ARRAY presence: null or empty Count. Do NOT treat the array ref as a nested-class
			// allocate-on-absent landmine - Count() is the test this file is allowed.
			if (!group.waypoints)
				continue;
			if (group.waypoints.Count() < 1)
				continue;

			TBD_WaypointSquad squad = new TBD_WaypointSquad();
			squad.faction = factionKey;
			squad.callsign = group.callsign;
			squad.waypoints = group.waypoints;
			squad.armed = false;
			s_aSquads.Insert(squad);
		}
	}

	//------------------------------------------------------------------------------------------------
	static void Tick()
	{
		if (!EnsureParsed())
			return;

		string missionId = CurrentMissionId();
		if (missionId != s_sParsedForMission)
		{
			TBD_Log.Warn(CH, string.Format("the loaded mission is '%1' but the waypoint registry was built for '%2' - rebuilding",
				missionId, s_sParsedForMission));
			Clear();
			return;
		}

		if (!s_bAnnounced)
		{
			int n = 0;
			if (s_aSquads)
				n = s_aSquads.Count();
			TBD_Log.Event(CH, string.Format("parsed waypointed groups=%1 mission='%2'", n, missionId));
			s_bAnnounced = true;
		}

		if (!s_aSquads || s_aSquads.Count() < 1)
			return;

		TBD_FrameworkManager fm = TBD_FrameworkManager.GetInstance();
		if (!fm || fm.GetStage() != TBD_EGameStage.LIVE)
			return;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn || !spawn.AreSlotBodiesMaterialized())
			return;

		foreach (TBD_WaypointSquad squad : s_aSquads)
		{
			if (!squad)
				continue;
			if (squad.armed)
			{
				AbsorbNewMembers(squad, spawn);
				continue;
			}

			ArmSquad(squad, spawn);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void ArmSquad(TBD_WaypointSquad squad, TBD_SpawnManager spawn)
	{
		array<IEntity> members = CollectUnclaimedBodies(squad, spawn);
		if (!members || members.Count() < 1)
			return;

		vector origin = members[0].GetOrigin();
		SCR_AIGroup group = SpawnGroup(origin, members[0]);
		if (!group)
		{
			TBD_Log.Error(CH, string.Format("failed to spawn AI group for %1:%2 - seats stay parked",
				squad.faction, squad.callsign));
			return;
		}

		foreach (IEntity body : members)
		{
			if (!body)
				continue;
			group.AddAIEntityToGroup(body);
		}

		if (!IssueWaypoints(group, squad))
		{
			TBD_Log.Error(CH, string.Format("group %1:%2 formed but no waypoint entity spawned - subjects exist with nothing to follow",
				squad.faction, squad.callsign));
			return;
		}

		squad.group = group;
		squad.armed = true;
		TBD_Log.Event(CH, string.Format("armed %1:%2 members=%3 waypoints=%4",
			squad.faction, squad.callsign, members.Count(), squad.waypoints.Count()));
	}

	//------------------------------------------------------------------------------------------------
	protected static void AbsorbNewMembers(TBD_WaypointSquad squad, TBD_SpawnManager spawn)
	{
		if (!squad.group)
			return;

		array<IEntity> members = CollectUnclaimedBodies(squad, spawn);
		if (!members)
			return;

		foreach (IEntity body : members)
		{
			if (!body)
				continue;

			AIControlComponent control = AIControlComponent.Cast(body.FindComponent(AIControlComponent));
			if (!control)
				continue;

			AIAgent agent = control.GetAIAgent();
			if (agent && agent.GetParentGroup() == squad.group)
				continue;

			squad.group.AddAIEntityToGroup(body);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static array<IEntity> CollectUnclaimedBodies(TBD_WaypointSquad squad, TBD_SpawnManager spawn)
	{
		array<ref TBD_MissionSlotStruct> slots = TBD_MissionLoader.GetSlots();
		if (!slots)
			return null;

		array<IEntity> members = new array<IEntity>();
		foreach (TBD_MissionSlotStruct slot : slots)
		{
			if (!slot)
				continue;
			if (slot.faction != squad.faction)
				continue;
			if (slot.groupCallsign != squad.callsign)
				continue;
			if (SlotIsPlayerClaimed(spawn, slot))
				continue;

			IEntity body = spawn.GetSlotBody(slot.Key());
			if (!body)
				continue;

			members.Insert(body);
		}

		return members;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool SlotIsPlayerClaimed(TBD_SpawnManager spawn, TBD_MissionSlotStruct slot)
	{
		if (!spawn || !slot)
			return false;

		array<int> ids = {};
		GetGame().GetPlayerManager().GetPlayers(ids);
		foreach (int id : ids)
		{
			TBD_MissionSlotStruct assigned = spawn.GetAssignedSlot(id);
			if (!assigned)
				continue;
			if (assigned.Key() == slot.Key())
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static SCR_AIGroup SpawnGroup(vector origin, IEntity member)
	{
		Resource resource = Resource.Load(PREFAB_GROUP);
		if (!resource || !resource.IsValid())
		{
			TBD_Log.Error(CH, "Group_Base prefab failed to load");
			return null;
		}

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = origin;

		IEntity ent = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		SCR_AIGroup group = SCR_AIGroup.Cast(ent);
		if (!group)
			return null;

		if (member)
		{
			FactionAffiliationComponent fac = FactionAffiliationComponent.Cast(member.FindComponent(FactionAffiliationComponent));
			if (fac)
			{
				Faction faction = fac.GetAffiliatedFaction();
				if (!faction)
					faction = fac.GetDefaultAffiliatedFaction();
				if (faction)
					group.SetFaction(faction);
			}
		}

		return group;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool IssueWaypoints(SCR_AIGroup group, TBD_WaypointSquad squad)
	{
		array<AIWaypoint> issued = new array<AIWaypoint>();
		AIWaypointCycle cycleWp = null;

		foreach (int index, TBD_WaypointWireStruct wire : squad.waypoints)
		{
			if (!wire)
			{
				TBD_Log.Warn(CH, string.Format("%1:%2 waypoints[%3] is null - skipped",
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
				TBD_Log.Warn(CH, string.Format("%1:%2 type=cycle has no sibling waypoints to wrap",
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

	//------------------------------------------------------------------------------------------------
	protected static AIWaypointCycle SpawnCycleWaypoint(TBD_WaypointWireStruct wire)
	{
		AIWaypoint wp = SpawnPrefabAt(PREFAB_CYCLE, wire);
		return AIWaypointCycle.Cast(wp);
	}

	//------------------------------------------------------------------------------------------------
	protected static AIWaypoint SpawnOneWaypoint(TBD_WaypointWireStruct wire, int index)
	{
		ResourceName prefab = PrefabForType(wire.type);
		if (prefab.IsEmpty())
		{
			TBD_Log.Warn(CH, string.Format("waypoints[%1] type='%2' is not a known waypoint kind - using move",
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
				// CONN-WP-ATTACH-001: sit the boarding waypoint ON the authored vehicle.
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
				TBD_Log.Warn(CH, string.Format("waypoints[%1] vehicleUid='%2' did not resolve - boarding by proximity at authored x/z",
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

	//------------------------------------------------------------------------------------------------
	protected static bool IsBoardingType(string type)
	{
		if (type == TYPE_GET_IN)
			return true;
		if (type == TYPE_GET_OUT)
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
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

	//------------------------------------------------------------------------------------------------
	protected static AIWaypoint SpawnPrefabAt(ResourceName prefab, TBD_WaypointWireStruct wire)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
		{
			TBD_Log.Error(CH, "waypoint prefab failed to load: " + prefab);
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

	//------------------------------------------------------------------------------------------------
	protected static void ApplyCompletion(AIWaypoint wp, TBD_WaypointWireStruct wire)
	{
		if (wire.HasRadius())
			wp.SetCompletionRadius(wire.radiusM);

		EAIWaypointCompletionType completion = EAIWaypointCompletionType.Any;
		if (IsBoardingType(wire.type))
			completion = EAIWaypointCompletionType.All;

		if (wire.behaviour == BH_COMBAT)
			completion = EAIWaypointCompletionType.All;
		if (wire.behaviour == BH_CARELESS)
			completion = EAIWaypointCompletionType.Any;
		if (wire.behaviour == BH_STEALTH)
			completion = EAIWaypointCompletionType.Any;

		wp.SetCompletionType(completion);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplySpeedAndBehaviour(AIWaypoint wp, TBD_WaypointWireStruct wire)
	{
		SCR_AIWaypoint scripted = SCR_AIWaypoint.Cast(wp);
		if (!scripted)
			return;

		EMovementType speed;
		bool haveSpeed = SpeedFromWire(wire, speed);
		if (!haveSpeed)
			return;

		SCR_AIGroupCharactersMovementSpeedSetting setting = SCR_AIGroupCharactersMovementSpeedSetting.Create(
			SCR_EAISettingOrigin.WAYPOINT, speed);
		if (setting)
			scripted.AddSetting(setting);
	}

	//------------------------------------------------------------------------------------------------
	//! speedMode wins. When it is absent, behaviour selects a speed ceiling so ATTR-FIELD-WP-BEHAVIOUR
	//! is not a dead parsed field: careless/safe/stealth walk, aware runs, combat sprints.
	protected static bool SpeedFromWire(TBD_WaypointWireStruct wire, out EMovementType speed)
	{
		if (wire.speedMode == SPEED_LIMITED)
		{
			speed = EMovementType.WALK;
			return true;
		}
		if (wire.speedMode == SPEED_NORMAL)
		{
			speed = EMovementType.RUN;
			return true;
		}
		if (wire.speedMode == SPEED_FULL)
		{
			speed = EMovementType.SPRINT;
			return true;
		}

		if (wire.behaviour == BH_CARELESS || wire.behaviour == BH_SAFE || wire.behaviour == BH_STEALTH)
		{
			speed = EMovementType.WALK;
			return true;
		}
		if (wire.behaviour == BH_AWARE)
		{
			speed = EMovementType.RUN;
			return true;
		}
		if (wire.behaviour == BH_COMBAT)
		{
			speed = EMovementType.SPRINT;
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
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

		BaseWorld world = GetGame().GetWorld();
		if (!world)
			return null;

		s_VehicleHit = null;
		s_VehicleUidWanted = uid;

		vector mins = Vector(found.x - VEHICLE_XZ_M, -VEHICLE_Y_M, found.z - VEHICLE_XZ_M);
		vector maxs = Vector(found.x + VEHICLE_XZ_M, VEHICLE_Y_M, found.z + VEHICLE_XZ_M);
		world.QueryEntitiesByAABB(mins, maxs, OnVehicleQuery);

		IEntity hit = s_VehicleHit;
		s_VehicleHit = null;
		s_VehicleUidWanted = string.Empty;
		return hit;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool OnVehicleQuery(IEntity entity)
	{
		if (!entity)
			return true;
		if (ChimeraCharacter.Cast(entity))
			return true;
		if (!Vehicle.Cast(entity))
			return true;

		s_VehicleHit = entity;
		return false;
	}
}

//------------------------------------------------------------------------------------------------
//! Heartbeat. Same idiom as TBD_TriggerRuntime / TBD_WinConditionEvaluator: a modded game mode
//! self-wires because this slice cannot add a component to TBD_GameMode.et. Fenced by
//! IsFrameworkWorld so a vanilla scenario with the mod loaded schedules nothing.
modded class SCR_BaseGameMode
{
	protected bool m_bTBD_WaypointTickArmed;

	//------------------------------------------------------------------------------------------------
	protected override void OnGameStart()
	{
		super.OnGameStart();

		TBD_WaypointRuntime.Clear();

		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!TBD_FrameworkManager.IsFrameworkWorld())
			return;

		if (m_bTBD_WaypointTickArmed)
			return;

		m_bTBD_WaypointTickArmed = true;
		GetGame().GetCallqueue().CallLater(TBD_WaypointTick, TBD_WaypointRuntime.TICK_MS, false);
	}

	//------------------------------------------------------------------------------------------------
	void TBD_WaypointTick()
	{
		if (GetGame().GetGameMode() != this)
			return;

		TBD_WaypointRuntime.Tick();

		GetGame().GetCallqueue().CallLater(TBD_WaypointTick, TBD_WaypointRuntime.TICK_MS, false);
	}
}
