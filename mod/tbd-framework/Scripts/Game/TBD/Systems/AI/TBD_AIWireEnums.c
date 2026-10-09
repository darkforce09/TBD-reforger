/**
 * @file TBD_AIWireEnums.c
 * @brief The AI wire tokens for speed and behaviour, and the movement speed they select.
 *
 * Role: one vocabulary for the `speedMode` and `behaviour` strings the mission document authors
 * on groups and waypoints, and their mapping to `EMovementType`.  Position: called by
 * `TBD_GroupState` and `TBD_WaypointRuntime`.
 * State: none.  Invariants: an explicit `speedMode` always wins over `behaviour`; an unknown or
 * empty token selects nothing and leaves the engine default in place.
 */

//! AI wire tokens and the speed mapping.
class TBD_AIWireEnums
{
	static const string SPEED_LIMITED = "limited"; //!< speedMode: walk
	static const string SPEED_NORMAL = "normal"; //!< speedMode: run
	static const string SPEED_FULL = "full"; //!< speedMode: sprint

	static const string BEHAVIOUR_CARELESS = "careless"; //!< behaviour: walk ceiling
	static const string BEHAVIOUR_SAFE = "safe"; //!< behaviour: walk ceiling
	static const string BEHAVIOUR_AWARE = "aware"; //!< behaviour: run ceiling
	static const string BEHAVIOUR_COMBAT = "combat"; //!< behaviour: sprint ceiling
	static const string BEHAVIOUR_STEALTH = "stealth"; //!< behaviour: walk ceiling

	//! The explicit speed an authored `speedMode` token names.
	//! @param speed set to walk, run or sprint when the token is known
	//! @return false for an empty or unknown token, leaving `speed` untouched
	static bool SpeedFromSpeedMode(string speedMode, out EMovementType speed)
	{
		if (speedMode == SPEED_LIMITED)
		{
			speed = EMovementType.WALK;
			return true;
		}
		if (speedMode == SPEED_NORMAL)
		{
			speed = EMovementType.RUN;
			return true;
		}
		if (speedMode == SPEED_FULL)
		{
			speed = EMovementType.SPRINT;
			return true;
		}

		return false;
	}

	//! The speed ceiling an authored `behaviour` token implies: careless, safe and stealth walk,
	//! aware runs, combat sprints.
	//! @param speed set to the ceiling when the token is known
	//! @return false for an empty or unknown token, leaving `speed` untouched
	static bool SpeedCeilingFromBehaviour(string behaviour, out EMovementType speed)
	{
		if (behaviour == BEHAVIOUR_CARELESS || behaviour == BEHAVIOUR_SAFE || behaviour == BEHAVIOUR_STEALTH)
		{
			speed = EMovementType.WALK;
			return true;
		}
		if (behaviour == BEHAVIOUR_AWARE)
		{
			speed = EMovementType.RUN;
			return true;
		}
		if (behaviour == BEHAVIOUR_COMBAT)
		{
			speed = EMovementType.SPRINT;
			return true;
		}

		return false;
	}

	//! The speed for one authored group or waypoint: `speedMode` when it names one, else the
	//! `behaviour` ceiling.
	//! @return false when neither token selects a speed
	static bool SpeedFromWire(string speedMode, string behaviour, out EMovementType speed)
	{
		if (SpeedFromSpeedMode(speedMode, speed))
			return true;

		return SpeedCeilingFromBehaviour(behaviour, speed);
	}
}
