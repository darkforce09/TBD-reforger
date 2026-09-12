//! T-689 -- play-area vehicle-class axis: which occupants the penalty applies to.
//!
//! T-706 put `vehicleClasses` on `$defs/zoneRules`. Nothing bound it. Play-area enforcement
//! already has graceSeconds / warnEverySeconds / penalty; the missing axis is WHICH classes
//! those rules confine. FNF v4's play-zone AIR flag is false: aircraft may leave the play
//! area, and a player who then exits the aircraft outside it is infantry again and is
//! confined (teleport-back in FNF; warn/kill here via the existing penalty).
//!
//! Presence (JsonLoadContext allocates an absent `ref array`, so Count() is the test):
//!   * Count() == 0 (key absent, or authored `[]` which this typed reader cannot tell
//!     apart from absent) -- the rule applies to EVERY class. Today's behaviour.
//!   * Count() > 0 -- the rule applies to exactly the named classes. Omitting `aircraft`
//!     is how the aircraft exemption is authored.
//!
//! Occupant class: on-foot -> infantry; heli/plane -> aircraft; buoyancy-only -> sea;
//! any other Vehicle -> ground. Unknown vehicle class -> ground.
//! @contract mission.schema.json#/$defs/zoneRules

//------------------------------------------------------------------------------------------------
//! One zone's resolved vehicle-class filter, copied off the loader struct.
class TBD_PlayAreaVehicleAxisBound
{
	string zoneId;
	bool restrict;
	bool infantry;
	bool ground;
	bool aircraft;
	bool sea;
}

//------------------------------------------------------------------------------------------------
//! Server-side bind + classify + effective-penalty. TBD_ZoneRegistry applies the result on
//! the boundary / base_protection query path so PlayAreaComponent's existing caller does not
//! have to change.
class TBD_PlayAreaVehicleAxis
{
	static const string CH = "PlayAreaAxis";

	static const string CLASS_INFANTRY = "infantry";
	static const string CLASS_GROUND = "ground";
	static const string CLASS_AIRCRAFT = "aircraft";
	static const string CLASS_SEA = "sea";

	protected static ref array<ref TBD_PlayAreaVehicleAxisBound> s_aBounds;

	//------------------------------------------------------------------------------------------------
	static void Clear()
	{
		s_aBounds = null;
	}

	//------------------------------------------------------------------------------------------------
	//! Copy vehicleClasses off this zone. Called from TBD_ZoneRegistry.ResolveRules once per
	//! zone; statics outlive a world so Registry.Clear calls Clear() first.
	static void Bind(string zoneId, TBD_MissionZoneRulesStruct rules)
	{
		if (!s_aBounds)
			s_aBounds = new array<ref TBD_PlayAreaVehicleAxisBound>();

		TBD_PlayAreaVehicleAxisBound bound = new TBD_PlayAreaVehicleAxisBound();
		bound.zoneId = zoneId;
		bound.restrict = false;

		if (!rules)
		{
			s_aBounds.Insert(bound);
			return;
		}

		if (!rules.vehicleClasses)
		{
			s_aBounds.Insert(bound);
			return;
		}

		if (rules.vehicleClasses.Count() == 0)
		{
			s_aBounds.Insert(bound);
			return;
		}

		bound.restrict = true;
		foreach (string token : rules.vehicleClasses)
		{
			if (token == CLASS_INFANTRY)
			{
				bound.infantry = true;
			}
			else if (token == CLASS_GROUND)
			{
				bound.ground = true;
			}
			else if (token == CLASS_AIRCRAFT)
			{
				bound.aircraft = true;
			}
			else if (token == CLASS_SEA)
			{
				bound.sea = true;
			}
			else
			{
				TBD_Log.Warn(CH, string.Format("zone '%1' rules.vehicleClasses token '%2' is not infantry|ground|aircraft|sea -- ignored",
					zoneId, token));
			}
		}

		s_aBounds.Insert(bound);
		TBD_Log.Kv(CH, "vehicleClasses", string.Format("id=%1 infantry=%2 ground=%3 aircraft=%4 sea=%5",
			zoneId, bound.infantry, bound.ground, bound.aircraft, bound.sea));
	}

	//------------------------------------------------------------------------------------------------
	//! Schema class of this occupant. On-foot is infantry. Unknown vehicle -> ground.
	static string ClassifyOccupant(IEntity body)
	{
		if (!body)
			return CLASS_GROUND;

		IEntity vehicle = SCR_CompartmentAccessComponent.GetVehicleIn(body);
		if (!vehicle)
			return CLASS_INFANTRY;

		if (IsAircraft(vehicle))
			return CLASS_AIRCRAFT;

		if (vehicle.FindComponent(VehicleBuoyancyComponent))
			return CLASS_SEA;

		return CLASS_GROUND;
	}

	//------------------------------------------------------------------------------------------------
	//! Does this zone's axis confine this occupant? Absent filter -> yes (today's everyone).
	static bool AppliesToOccupant(string zoneId, IEntity body)
	{
		TBD_PlayAreaVehicleAxisBound bound = Find(zoneId);
		if (!bound)
			return true;
		if (!bound.restrict)
			return true;

		string cls = ClassifyOccupant(body);
		if (cls == CLASS_INFANTRY)
			return bound.infantry;
		if (cls == CLASS_GROUND)
			return bound.ground;
		if (cls == CLASS_AIRCRAFT)
			return bound.aircraft;
		if (cls == CLASS_SEA)
			return bound.sea;
		return bound.ground;
	}

	//------------------------------------------------------------------------------------------------
	//! Authored penalty, or NONE when this occupant is off the axis (the aircraft exemption).
	static TBD_EZonePenalty EffectivePenalty(TBD_EZonePenalty authored, string zoneId, IEntity body)
	{
		if (!AppliesToOccupant(zoneId, body))
			return TBD_EZonePenalty.NONE;
		return authored;
	}

	//------------------------------------------------------------------------------------------------
	//! Reverse-lookup the body PlayAreaComponent just sampled at this XZ, then ask the axis.
	//! No body -> confined (fail toward today's apply-all). Exemption is EffectivePenalty
	//! dropping the authored value -- that is the apply, not a second unused helper.
	static bool OccupantConfinedByZone(TBD_Zone zone, float px, float pz)
	{
		if (!zone)
			return true;
		IEntity body = FindOccupantAt(px, pz);
		if (!body)
			return true;
		TBD_EZonePenalty effective = EffectivePenalty(zone.m_ePenalty, zone.m_sId, body);
		if (!AppliesToOccupant(zone.m_sId, body))
			return false;
		return effective == zone.m_ePenalty;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool IsAircraft(IEntity vehicle)
	{
		if (vehicle.FindComponent(HelicopterControllerComponent))
			return true;
		if (vehicle.FindComponent(VehicleHelicopterSimulation))
			return true;
		if (vehicle.FindComponent(AirplaneControllerComponent))
			return true;
		if (vehicle.FindComponent(VehicleFixedWingSimulation))
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_PlayAreaVehicleAxisBound Find(string zoneId)
	{
		if (!s_aBounds)
			return null;
		foreach (TBD_PlayAreaVehicleAxisBound bound : s_aBounds)
		{
			if (bound && bound.zoneId == zoneId)
				return bound;
		}
		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected static IEntity FindOccupantAt(float px, float pz)
	{
		PlayerManager players = GetGame().GetPlayerManager();
		if (!players)
			return null;

		array<int> ids = new array<int>();
		players.GetPlayers(ids);
		foreach (int playerId : ids)
		{
			IEntity body = players.GetPlayerControlledEntity(playerId);
			if (!body)
				continue;
			vector origin = body.GetOrigin();
			if (origin[0] != px)
				continue;
			if (origin[2] != pz)
				continue;
			return body;
		}
		return null;
	}
}
