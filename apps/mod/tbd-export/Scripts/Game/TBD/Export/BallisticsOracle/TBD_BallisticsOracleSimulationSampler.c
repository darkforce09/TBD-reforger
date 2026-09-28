/**
 * @file TBD_BallisticsOracleSimulationSampler.c
 * @brief One GetProjectileSimulationResult sample: the call, its raw vector and its decoded meaning.
 *
 * Role: defines the simulation case lattice (elevations, winds, target heights) and turns one case
 * into a JSON sample: the engine inputs, the raw result vector, the decoded downrange, crossrange
 * and height, and the time of flight found by bisection on the simulation time limit.
 * Position: called by TBD_BallisticsOracleSimulationComponent with a spawned shell's
 * ProjectileMoveComponent; its samples feed the calibration bundle's `simulation` oracle samples.
 * State: none; pure functions over the component they are given.  Invariants: every case launches
 * from the world origin at azimuth 0 (north, +z), so the raw result is decoded without a frame
 * change; the zero-wind, zero-height case of every elevation carries the BallisticTable
 * forward-angle reference that confirms the decoding.
 */

//! Runs and records GetProjectileSimulationResult cases for one shell charge.
class TBD_BallisticsOracleSimulationSampler
{
	static const float AZIMUTH_DEG = 0; //!< launch azimuth, degrees clockwise from north; JSON key `azimuth_deg`
	static const float MAX_SIMULATION_TIME_S = 60; //!< simulation time limit of a full case, s; JSON key `max_simulation_time_s`
	static const int TIME_BISECTION_STEPS = 16; //!< bisection steps on the time limit; resolution 60 s / 2^16
	protected static const float SAME_POSITION_M = 0.01; //!< a limited run that ends this close to the full run has reached its end
	protected static const float TARGET_REACHED_M = 1.0; //!< a full run whose end height is this close to the target height reached it
	protected static const float AVAILABLE_MIN_RANGE_M = 1.0; //!< a probe shorter than this means the engine simulation is off

	//! Fills `elevations` with the sampled elevations, degrees: 45, 55, 65, 75, 85.
	static void Elevations(notnull array<float> elevations)
	{
		elevations.Insert(45);
		elevations.Insert(55);
		elevations.Insert(65);
		elevations.Insert(75);
		elevations.Insert(85);
	}

	//! Fills the parallel `speeds` (m/s) and `fromDegrees` (meteorological "from" direction) with
	//! calm plus 5 and 10 m/s from 0, 90, 180 and 270 degrees.
	static void Winds(notnull array<float> speeds, notnull array<float> fromDegrees)
	{
		speeds.Insert(0);
		fromDegrees.Insert(0);
		for (int speedStep = 1; speedStep <= 2; speedStep++)
		{
			for (int quadrant = 0; quadrant < 4; quadrant++)
			{
				speeds.Insert(speedStep * 5);
				fromDegrees.Insert(quadrant * 90);
			}
		}
	}

	//! Fills `heights` with the sampled target heights relative to the launch point, m: -100, 0, 100.
	static void TargetHeights(notnull array<float> heights)
	{
		heights.Insert(-100);
		heights.Insert(0);
		heights.Insert(100);
	}

	//! Returns the air velocity, world axes (x east, y up, z north), of a wind of `speed` m/s
	//! blowing from `fromDegrees`: the air moves toward the opposite bearing.
	static vector WindVector(float speed, float fromDegrees)
	{
		float bearing = fromDegrees * Math.DEG2RAD;
		return Vector(-speed * Math.Sin(bearing), 0, -speed * Math.Cos(bearing));
	}

	//! Returns true when a 45 degree, calm, level probe at `speed` m/s travels at least 1 m. The
	//! engine documents that the simulation works only with projectile debugging enabled; a probe
	//! that stays at the launch point means it is off.
	static bool IsAvailable(notnull ProjectileMoveComponent move, float speed)
	{
		vector result = move.GetProjectileSimulationResult(vector.Zero, speed, 45, AZIMUTH_DEG, vector.Zero, 0, true, MAX_SIMULATION_TIME_S, -1);
		return Math.Sqrt(result[0] * result[0] + result[2] * result[2]) >= AVAILABLE_MIN_RANGE_M;
	}

	//! Runs one case and returns its JSON sample: `rings`, `charge_index`, `init_speed_coef`,
	//! `inputs`, `outputs` and, for the calm level case, `forward_angle_reference`.
	static string Sample(notnull ProjectileMoveComponent move, notnull TBD_BallisticsOracleShellSource shell, int charge, float elevationDeg, float windSpeed, float windFromDeg, float targetHeight)
	{
		float coefficient = shell.ChargeCoefficient(charge);
		float speed = shell.m_fInitSpeed * coefficient;
		vector wind = WindVector(windSpeed, windFromDeg);
		vector result = move.GetProjectileSimulationResult(vector.Zero, speed, elevationDeg, AZIMUTH_DEG, wind, targetHeight, true, MAX_SIMULATION_TIME_S, -1);
		bool reached = Math.AbsFloat(result[1] - targetHeight) <= TARGET_REACHED_M;

		string json = "{\"rings\":" + shell.ChargeRings(charge).ToString() + ",\"charge_index\":" + charge.ToString();
		json += ",\"init_speed_coef\":" + TBD_BallisticsOracleJson.Number(coefficient);
		json += ",\"inputs\":{\"launch_position_world\":[0,0,0],\"init_speed_m_s\":" + TBD_BallisticsOracleJson.Number(speed);
		json += ",\"elevation_deg\":" + TBD_BallisticsOracleJson.Number(elevationDeg);
		json += ",\"azimuth_deg\":" + TBD_BallisticsOracleJson.Number(AZIMUTH_DEG);
		json += ",\"wind_speed_m_s\":" + TBD_BallisticsOracleJson.Number(windSpeed);
		json += ",\"wind_from_deg\":" + TBD_BallisticsOracleJson.Number(windFromDeg);
		json += ",\"wind_vector_world\":" + TBD_BallisticsOracleJson.Vector3(wind);
		json += ",\"target_height_m\":" + TBD_BallisticsOracleJson.Number(targetHeight);
		json += ",\"must_fall_down\":true,\"max_simulation_time_s\":" + TBD_BallisticsOracleJson.Number(MAX_SIMULATION_TIME_S) + "}";
		json += ",\"outputs\":" + OutputsJson(move, speed, elevationDeg, wind, targetHeight, result, reached);
		if (windSpeed == 0 && targetHeight == 0)
			json += ",\"forward_angle_reference\":" + ForwardReferenceJson(shell, coefficient, elevationDeg);

		return json + "}";
	}

	//! Returns the outputs object: `raw_result`, the decoded `downrange_m`, `crossrange_m` (right of
	//! the line of fire positive) and `height_m`, `reached_target_height` and `time_of_flight_s`
	//! (null when the target height was not reached).
	protected static string OutputsJson(ProjectileMoveComponent move, float speed, float elevationDeg, vector wind, float targetHeight, vector result, bool reached)
	{
		float azimuth = AZIMUTH_DEG * Math.DEG2RAD;
		float downrange = result[0] * Math.Sin(azimuth) + result[2] * Math.Cos(azimuth);
		float crossrange = result[0] * Math.Cos(azimuth) - result[2] * Math.Sin(azimuth);
		string json = "{\"raw_result\":" + TBD_BallisticsOracleJson.Vector3(result);
		json += ",\"downrange_m\":" + TBD_BallisticsOracleJson.Number(downrange);
		json += ",\"crossrange_m\":" + TBD_BallisticsOracleJson.Number(crossrange);
		json += ",\"height_m\":" + TBD_BallisticsOracleJson.Number(result[1]);
		json += ",\"reached_target_height\":" + TBD_BallisticsOracleJson.Boolean(reached);
		if (!reached)
			return json + ",\"time_of_flight_s\":null}";

		float flightTime = TimeOfFlight(move, speed, elevationDeg, wind, targetHeight, result);
		return json + ",\"time_of_flight_s\":" + TBD_BallisticsOracleJson.Number(flightTime) + "}";
	}

	//! Returns the shortest time limit, to 60 s / 2^16, at which the limited run ends within 1 cm of
	//! the full run's `finalPosition`: the time the projectile reaches the target height.
	protected static float TimeOfFlight(ProjectileMoveComponent move, float speed, float elevationDeg, vector wind, float targetHeight, vector finalPosition)
	{
		float low = 0;
		float high = MAX_SIMULATION_TIME_S;
		for (int step = 0; step < TIME_BISECTION_STEPS; step++)
		{
			float middle = (low + high) * 0.5;
			vector partial = move.GetProjectileSimulationResult(vector.Zero, speed, elevationDeg, AZIMUTH_DEG, wind, targetHeight, true, middle, -1);
			if (vector.Distance(partial, finalPosition) <= SAME_POSITION_M)
				high = middle;
			else
				low = middle;
		}

		return high;
	}

	//! Returns the BallisticTable forward-angle result at the same elevation and coefficient:
	//! `elevation_rad`, `range_m` and `time_of_flight_s`, indirect-fire tables.
	protected static string ForwardReferenceJson(TBD_BallisticsOracleShellSource shell, float coefficient, float elevationDeg)
	{
		float elevationRad = elevationDeg * Math.DEG2RAD;
		float flightTime;
		float range = BallisticTable.GetDistanceOfProjectileSource(elevationRad, flightTime, shell.Source(), coefficient, false);
		string json = "{\"elevation_rad\":" + TBD_BallisticsOracleJson.Number(elevationRad);
		json += ",\"range_m\":" + TBD_BallisticsOracleJson.Number(range);
		return json + ",\"time_of_flight_s\":" + TBD_BallisticsOracleJson.Number(flightTime) + "}";
	}
}
