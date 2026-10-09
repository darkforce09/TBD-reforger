/**
 * @file TBD_BallisticsOracleForwardAngles.c
 * @brief Writes forward_angles.json: BallisticTable range, time and aim-height answers per shell.
 *
 * Role: asks the engine's BallisticTable, in the editor, for the range and time of flight of every
 * vanilla mortar shell at every charge and native table coefficient over the elevation lattice
 * 1600 to 800 mils in 1.5625-mil steps (513 elevations), and for the raw aim height and time of
 * seven altitude differences at five distances per charge.  Position: run by
 * TBD_BallisticsOraclePlugin; reads the prefabs through TBD_BallisticsOracleShellSource; its
 * samples feed the calibration bundle's `forward_angle` and `altitude_difference` oracle samples.
 * State: the open output and the sample and error counts of one Write call.  Invariants: every
 * engine answer is written raw, beside its inputs; a shell that cannot be read is written with its
 * errors and no samples; the sidecar is written only when every write succeeded; every lattice
 * elevation is a whole number of sixteenths of a mil, so the 1.5625-mil step divides the 25 and
 * 12.5-mil steps of the native tables and every elevation is written as an exact decimal.
 */

//! Samples BallisticTable for the vanilla mortar shells and writes `forward_angles.json`.
class TBD_BallisticsOracleForwardAngles
{
	static const int MILS_PER_CIRCLE = 6400; //!< mil convention of the elevation lattice
	static const int FIRST_MILS = 1600; //!< first lattice elevation, mils (90 degrees)
	static const int LAST_MILS = 800; //!< last lattice elevation, mils (45 degrees)
	static const int SIXTEENTHS_PER_MIL = 16; //!< lattice resolution: elevations are whole sixteenths of a mil
	static const int STEP_SIXTEENTHS = 25; //!< lattice step, sixteenths of a mil (1.5625 mils); JSON key `step_mils`
	static const string OUTPUT_NAME = "forward_angles.json"; //!< output file name in the run folder
	protected static const string DOCUMENT_TYPE = "ballistics_oracle_forward_angles"; //!< JSON key `document_type`

	protected ref TBD_BallisticsOracleOutputFile m_Output; //!< forward_angles.json
	protected ref array<float> m_aAltitudeDifferences = {-200.0, -100.0, -50.0, 0.0, 50.0, 100.0, 200.0}; //!< sampled altitude differences, m
	protected ref array<float> m_aDistanceFractions = {0.2, 0.35, 0.5, 0.65, 0.8}; //!< sampled distances, as fractions of the charge's longest lattice range
	protected int m_iForwardSamples; //!< forward-angle samples written; JSON key `forward_angle_sample_count`
	protected int m_iAltitudeSamples; //!< altitude samples written; JSON key `altitude_difference_sample_count`
	protected int m_iShellErrors; //!< shell errors written; JSON key `shell_error_count`

	//! Writes forward_angles.json and its sidecar into the run folder. Returns the final status,
	//! `complete` or `incomplete`, or `failed` when the file could not be written.
	string Write(TBD_BallisticsOracleRun run)
	{
		m_Output = new TBD_BallisticsOracleOutputFile();
		if (!m_Output.Open(run.Directory() + OUTPUT_NAME))
			return "failed";

		m_Output.Append("{\"document_type\":\"" + DOCUMENT_TYPE + "\",\"schema_version\":1," + run.HeaderMembers());
		m_Output.Append(",\"elevation_lattice\":{\"mils_per_circle\":" + MILS_PER_CIRCLE.ToString() + ",\"first_mils\":" + FIRST_MILS.ToString());
		m_Output.Append(",\"last_mils\":" + LAST_MILS.ToString() + ",\"step_mils\":" + SixteenthsAsMils(STEP_SIXTEENTHS) + ",\"direct_fire\":false}");
		m_Output.Append(",\"altitude_lattice\":{\"altitude_differences_m\":" + NumbersJson(m_aAltitudeDifferences));
		m_Output.Append(",\"distance_fractions_of_longest_range\":" + NumbersJson(m_aDistanceFractions) + "},\"shells\":[");

		array<ResourceName> prefabs = {};
		TBD_BallisticsOracleShellSource.VanillaMortarShells(prefabs);
		for (int index = 0; index < prefabs.Count(); index++)
		{
			if (index > 0)
				m_Output.Append(",");

			WriteShell(prefabs[index]);
		}

		string status = "complete";
		if (m_iShellErrors > 0)
			status = "incomplete";

		m_Output.Append("],\"forward_angle_sample_count\":" + m_iForwardSamples.ToString());
		m_Output.Append(",\"altitude_difference_sample_count\":" + m_iAltitudeSamples.ToString());
		m_Output.Append(",\"shell_error_count\":" + m_iShellErrors.ToString() + ",\"status\":\"" + status + "\"");
		m_Output.Append(",\"finished_at\":" + TBD_BallisticsOracleJson.Quote(TBD_BallisticsOracleJson.IsoNowUtc()) + "}\n");
		if (!m_Output.Close() || !m_Output.WriteSidecar(run.Directory(), OUTPUT_NAME, DOCUMENT_TYPE, run, status))
			return "failed";

		return status;
	}

	//! Returns the SHA-256 of the written output.
	string Sha256()
	{
		return m_Output.Sha256();
	}

	//! Writes one shell object: its metadata, its forward-angle samples and its altitude samples.
	protected void WriteShell(ResourceName prefab)
	{
		TBD_BallisticsOracleShellSource shell = new TBD_BallisticsOracleShellSource();
		bool loaded = shell.Load(prefab);
		m_Output.Append("{\"forward_angle_samples\":[");
		if (loaded)
			WriteForwardAngles(shell);

		m_Output.Append("],\"altitude_difference_samples\":[");
		if (loaded)
			WriteAltitudeDifferences(shell);

		if (!loaded && shell.m_aErrors.IsEmpty())
			shell.m_aErrors.Insert("The prefab has no positive InitSpeed or no charge ring");

		m_iShellErrors += shell.m_aErrors.Count();
		m_Output.Append("]," + shell.MetadataMembers() + "}");
	}

	//! Writes the lattice for every charge coefficient, then for every native table coefficient
	//! that no charge uses.
	protected void WriteForwardAngles(TBD_BallisticsOracleShellSource shell)
	{
		bool first = true;
		for (int charge = 0; charge < shell.m_aChargeRings.Count(); charge++)
		{
			float coefficient = shell.ChargeCoefficient(charge);
			if (shell.FindCharge(coefficient) != charge)
				continue;

			string source = "charge_ring";
			if (shell.FindNative(coefficient) >= 0)
				source = "charge_ring+native_table";

			WriteLattice(shell, coefficient, source, shell.ChargeRings(charge).ToString(), first);
			first = false;
		}

		foreach (float nativeCoefficient : shell.m_aNativeCoefficients)
		{
			if (shell.FindCharge(nativeCoefficient) >= 0)
				continue;

			WriteLattice(shell, nativeCoefficient, "native_table", "null", first);
			first = false;
		}
	}

	//! Writes one sample per lattice elevation for `coefficient`: the inputs and the raw range and
	//! time of flight of BallisticTable.GetDistanceOfProjectileSource on the indirect-fire tables.
	protected void WriteLattice(TBD_BallisticsOracleShellSource shell, float coefficient, string source, string ringsJson, bool first)
	{
		string head = "{\"init_speed_coef\":" + TBD_BallisticsOracleJson.Number(coefficient);
		head += ",\"coefficient_source\":\"" + source + "\",\"rings\":" + ringsJson;
		int count = LatticeCount();
		for (int index = 0; index < count; index++)
		{
			if (!first || index > 0)
				m_Output.Append(",");

			int sixteenths = LatticeSixteenths(index);
			float radians = SixteenthsAsRadians(sixteenths);
			float flightTime;
			float range = BallisticTable.GetDistanceOfProjectileSource(radians, flightTime, shell.Source(), coefficient, false);
			string json = head + ",\"inputs\":{\"elevation_mils_6400\":" + SixteenthsAsMils(sixteenths);
			json += ",\"elevation_rad\":" + TBD_BallisticsOracleJson.Number(radians) + ",\"direct_fire\":false}";
			json += ",\"outputs\":{\"range_m\":" + TBD_BallisticsOracleJson.Number(range);
			json += ",\"time_of_flight_s\":" + TBD_BallisticsOracleJson.Number(flightTime) + "}}";
			m_Output.Append(json);
			m_iForwardSamples++;
		}
	}

	//! Writes, for every charge, the raw GetAimHeightOfProjectileAltitudeFromSource answer at each
	//! altitude difference and each distance fraction of the charge's longest lattice range.
	protected void WriteAltitudeDifferences(TBD_BallisticsOracleShellSource shell)
	{
		bool first = true;
		for (int charge = 0; charge < shell.m_aChargeRings.Count(); charge++)
		{
			float coefficient = shell.ChargeCoefficient(charge);
			float longest = LongestRange(shell, coefficient);
			if (longest <= 0)
			{
				shell.m_aErrors.Insert("Charge " + charge.ToString() + " has no positive lattice range, so it has no altitude samples");
				continue;
			}

			string head = "{\"rings\":" + shell.ChargeRings(charge).ToString() + ",\"charge_index\":" + charge.ToString();
			head += ",\"init_speed_coef\":" + TBD_BallisticsOracleJson.Number(coefficient);
			foreach (float fraction : m_aDistanceFractions)
			{
				foreach (float difference : m_aAltitudeDifferences)
				{
					if (!first)
						m_Output.Append(",");

					m_Output.Append(head + AltitudeSampleMembers(shell, coefficient, longest * fraction, difference));
					first = false;
					m_iAltitudeSamples++;
				}
			}
		}
	}

	//! Returns the inputs and outputs members of one altitude sample, closing its object: the call's
	//! boolean result, the aim height and the time of flight, all raw.
	protected string AltitudeSampleMembers(TBD_BallisticsOracleShellSource shell, float coefficient, float distance, float difference)
	{
		float aimHeight;
		float flightTime;
		bool answered = BallisticTable.GetAimHeightOfProjectileAltitudeFromSource(distance, aimHeight, flightTime, shell.Source(), difference, coefficient);
		string json = ",\"inputs\":{\"distance_m\":" + TBD_BallisticsOracleJson.Number(distance);
		json += ",\"altitude_difference_m\":" + TBD_BallisticsOracleJson.Number(difference) + "}";
		json += ",\"outputs\":{\"returned\":" + TBD_BallisticsOracleJson.Boolean(answered);
		json += ",\"aim_height_m\":" + TBD_BallisticsOracleJson.Number(aimHeight);
		return json + ",\"time_of_flight_s\":" + TBD_BallisticsOracleJson.Number(flightTime) + "}}";
	}

	//! Returns the longest range BallisticTable gives for `coefficient` over the lattice, m.
	protected float LongestRange(TBD_BallisticsOracleShellSource shell, float coefficient)
	{
		float longest = 0;
		int count = LatticeCount();
		for (int index = 0; index < count; index++)
		{
			float flightTime;
			float radians = SixteenthsAsRadians(LatticeSixteenths(index));
			float range = BallisticTable.GetDistanceOfProjectileSource(radians, flightTime, shell.Source(), coefficient, false);
			if (range > longest)
				longest = range;
		}

		return longest;
	}

	//! Returns the number of lattice elevations, FIRST_MILS and LAST_MILS included (513).
	protected static int LatticeCount()
	{
		return (FIRST_MILS - LAST_MILS) * SIXTEENTHS_PER_MIL / STEP_SIXTEENTHS + 1;
	}

	//! Returns the elevation of lattice position `index`, counted down from FIRST_MILS, in
	//! sixteenths of a mil.
	protected static int LatticeSixteenths(int index)
	{
		return FIRST_MILS * SIXTEENTHS_PER_MIL - index * STEP_SIXTEENTHS;
	}

	//! Returns `sixteenths` of a mil as the angle in radians passed to BallisticTable.
	protected static float SixteenthsAsRadians(int sixteenths)
	{
		return sixteenths * Math.PI2 / (MILS_PER_CIRCLE * SIXTEENTHS_PER_MIL);
	}

	//! Returns a non-negative `sixteenths` of a mil as an exact JSON decimal in mils, without
	//! trailing zeros: 25600 gives `1600`, 25 gives `1.5625`, 25592 gives `1599.5`.
	protected static string SixteenthsAsMils(int sixteenths)
	{
		int whole = sixteenths / SIXTEENTHS_PER_MIL;
		int remainder = sixteenths % SIXTEENTHS_PER_MIL;
		if (remainder == 0)
			return whole.ToString();

		// One sixteenth is 0.0625, so remainder * 625 is the fraction's four decimal digits.
		string digits = (remainder * 625).ToString();
		while (digits.Length() < 4)
			digits = "0" + digits;

		while (digits.Substring(digits.Length() - 1, 1) == "0")
			digits = digits.Substring(0, digits.Length() - 1);

		return whole.ToString() + "." + digits;
	}

	//! Returns `values` as a JSON array of numbers.
	protected static string NumbersJson(array<float> values)
	{
		string json = "[";
		for (int index = 0; index < values.Count(); index++)
		{
			if (index > 0)
				json += ",";

			json += TBD_BallisticsOracleJson.Number(values[index]);
		}

		return json + "]";
	}
}
