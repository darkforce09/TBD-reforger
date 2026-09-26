/**
 * @file TBD_PlayAreaVehicleAxis.c
 * @brief Which occupant classes a play-area zone confines, from `zoneRules.vehicleClasses`.
 *
 * Role: binds each zone's `vehicleClasses` (`infantry`, `ground`, `sea`, `aircraft`), classifies
 * a body's occupant class and says whether a zone confines it.  Position: bound by
 * `TBD_ZoneCompiler.ResolveRules`; asked by `TBD_ZoneRegistry.IsInsideBoundary` and
 * `FindViolatedProtection`.
 * State: static filter list for the life of the script VM, cleared with the zone registry.
 * Invariants: an empty or absent list confines every class (a typed reader cannot tell `[]` from
 * absent), so leaving out `aircraft` is how the aircraft exemption is authored; an unknown vehicle
 * is ground; with no body found at the sampled XZ the zone confines.
 */

//! One zone's resolved vehicle-class filter.
class TBD_PlayAreaVehicleAxisBound
{
	string zoneId; //!< `zones[].id`
	bool restrict; //!< false = the zone confines every class
	bool infantry; //!< `infantry` listed: on-foot players are confined
	bool ground; //!< `ground` listed: ground vehicle occupants are confined
	bool aircraft; //!< `aircraft` listed: helicopter and plane occupants are confined
	bool sea; //!< `sea` listed: boat occupants are confined
}

//! The vehicle-class axis of play-area zones.
//! @authority server
class TBD_PlayAreaVehicleAxis
{
	static const string CH = "PlayAreaAxis"; //!< log channel

	static const string CLASS_INFANTRY = "infantry"; //!< `vehicleClasses` token: on foot
	static const string CLASS_GROUND = "ground"; //!< `vehicleClasses` token: any other vehicle
	static const string CLASS_AIRCRAFT = "aircraft"; //!< `vehicleClasses` token: helicopter or plane
	static const string CLASS_SEA = "sea"; //!< `vehicleClasses` token: buoyant vehicle

	protected static ref array<ref TBD_PlayAreaVehicleAxisBound> s_aBounds; //!< one filter per bound zone; null until bound

	//! Drop every filter.
	static void Clear()
	{
		s_aBounds = null;
	}

	//! Bind one zone's `vehicleClasses`, logging unknown tokens and the resolved filter.
	//! @param zoneId the zone's id
	//! @param rules the zone's wire rules; may be null (confines every class)
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

	//! The schema class of this occupant: on foot is infantry, a helicopter or plane aircraft, a
	//! buoyant vehicle sea, anything else ground.
	//! @param body the player's controlled entity; null reads as ground
	//! @return one of the `CLASS_*` tokens
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

	//! Whether this zone's filter confines this occupant.
	//! @param zoneId the zone
	//! @param body the player's controlled entity
	//! @return true with no filter or an unrestricted one, else whether the class is listed
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

	//! The penalty that applies to this occupant.
	//! @param authored the zone's penalty
	//! @param zoneId the zone
	//! @param body the player's controlled entity
	//! @return `authored`, or `NONE` when the occupant is off the zone's axis
	static TBD_EZonePenalty EffectivePenalty(TBD_EZonePenalty authored, string zoneId, IEntity body)
	{
		if (!AppliesToOccupant(zoneId, body))
			return TBD_EZonePenalty.NONE;
		return authored;
	}

	//! Whether the zone confines whoever stands exactly at this XZ, found among the players'
	//! controlled entities (the position the play area just sampled).
	//! @param zone the zone; null confines
	//! @param px world X in metres
	//! @param pz world Z in metres
	//! @return true when confined, or when no zone or no body is found
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

	//! Whether a vehicle is a helicopter or a plane, by its controller or simulation component.
	//! @param vehicle the vehicle entity
	//! @return true for an aircraft
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

	//! The filter bound for this zone.
	//! @param zoneId the zone
	//! @return the filter, or null
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

	//! The player-controlled entity whose origin is exactly at this XZ.
	//! @param px world X in metres
	//! @param pz world Z in metres
	//! @return the body, or null
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
