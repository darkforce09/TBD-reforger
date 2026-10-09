/**
 * @file TBD_BallisticsOracleSimulationComponent.c
 * @brief Export game mode component that writes the ballistics oracle's simulation.json in play mode.
 *
 * Role: when the export world plays and the Workbench plugin has recorded an export generation,
 * spawns each vanilla mortar shell and samples ProjectileMoveComponent.GetProjectileSimulationResult
 * over every charge, elevation, wind and target height, together with the world gravity.
 * Position: carried by the export game mode prefab; reads the generation id the plugin writes to
 * `$profile:TBD_BallisticsOracle/active_generation.txt`; writes `simulation.json` and its sidecar
 * beside the plugin's `forward_angles.json`.
 * State: the run, the open output, the shell list, the case cursor and the spawned shell, on the
 * server only.  Invariants: one charge and elevation per call-queue tick, so the frame never holds
 * the whole run; a run with no recorded generation writes nothing; a shell that cannot be read or
 * spawned is listed as an error, never skipped silently; the sidecar is written only when every
 * write succeeded.
 */

//! Editor class of the ballistics oracle simulation component.
[ComponentEditorProps(category: "TBD/Export", description: "Writes the ballistics oracle simulation samples of the vanilla mortar shells when the export world plays")]
class TBD_BallisticsOracleSimulationComponentClass : SCR_BaseGameModeComponentClass
{
}

//! Samples the engine's projectile simulation for every vanilla mortar shell charge and writes
//! `simulation.json`.
class TBD_BallisticsOracleSimulationComponent : SCR_BaseGameModeComponent
{
	protected static const int START_DELAY_MS = 1000; //!< delay after OnPostInit before the run starts, ms
	protected static const string OUTPUT_NAME = "simulation.json"; //!< output file name in the run folder
	protected static const string DOCUMENT_TYPE = "ballistics_oracle_simulation"; //!< JSON key `document_type`

	protected ref TBD_BallisticsOracleRun m_Run; //!< the run being written
	protected ref TBD_BallisticsOracleOutputFile m_Output; //!< simulation.json
	protected ref array<ref TBD_BallisticsOracleShellSource> m_aShells = {}; //!< the shells, in VanillaMortarShells order
	protected ref array<float> m_aElevations = {}; //!< sampled elevations, degrees
	protected ref array<float> m_aWindSpeeds = {}; //!< sampled wind speeds, m/s, parallel to m_aWindFrom
	protected ref array<float> m_aWindFrom = {}; //!< sampled wind "from" directions, degrees
	protected ref array<float> m_aTargetHeights = {}; //!< sampled target heights, m
	protected ref array<string> m_aErrors = {}; //!< run-level errors; JSON key `errors`
	protected IEntity m_Projectile; //!< spawned instance of the shell being sampled; null between shells
	protected ProjectileMoveComponent m_Move; //!< m_Projectile's move component
	protected int m_iShell; //!< index of the shell being sampled
	protected int m_iCharge; //!< index of the charge being sampled
	protected int m_iElevation; //!< index of the elevation being sampled
	protected int m_iSamples; //!< samples written; JSON key `sample_count`
	protected bool m_bShellOpen; //!< true while the current shell's JSON object is open
	protected bool m_bShellHasSample; //!< true once the open shell holds a sample
	protected bool m_bUnavailable; //!< true when the engine simulation is off

	//! Schedules the run on the server when the plugin has recorded an export generation.
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);
		if (RplSession.Mode() == RplMode.Client)
			return;

		string generationId = TBD_BallisticsOracleRun.ReadActiveGeneration();
		if (generationId.IsEmpty())
		{
			Print("[TBD Ballistics Oracle] No active generation recorded; the simulation run stays idle", LogLevel.NORMAL);
			return;
		}

		GetGame().GetCallqueue().CallLater(Begin, START_DELAY_MS, false, generationId);
	}

	//! Starts the run: checks the hash, reads the shells, opens simulation.json and writes the
	//! header, the gravity, the case lattice and the decoding; then queues the first step.
	//! @authority server
	protected void Begin(string generationId)
	{
		m_Run = new TBD_BallisticsOracleRun();
		if (!m_Run.Begin(generationId))
			return;

		if (!TBD_BallisticsOracleSha256.SelfTest())
		{
			Print("[TBD Ballistics Oracle] SHA-256 self-test failed; no output written", LogLevel.ERROR);
			return;
		}

		TBD_BallisticsOracleSimulationSampler.Elevations(m_aElevations);
		TBD_BallisticsOracleSimulationSampler.Winds(m_aWindSpeeds, m_aWindFrom);
		TBD_BallisticsOracleSimulationSampler.TargetHeights(m_aTargetHeights);
		array<ResourceName> prefabs = {};
		TBD_BallisticsOracleShellSource.VanillaMortarShells(prefabs);
		foreach (ResourceName prefab : prefabs)
		{
			TBD_BallisticsOracleShellSource shell = new TBD_BallisticsOracleShellSource();
			shell.Load(prefab);
			m_aShells.Insert(shell);
		}

		m_Output = new TBD_BallisticsOracleOutputFile();
		if (!m_Output.Open(m_Run.Directory() + OUTPUT_NAME))
			return;

		m_Output.Append("{\"document_type\":\"" + DOCUMENT_TYPE + "\",\"schema_version\":1," + m_Run.HeaderMembers());
		m_Output.Append(",\"gravity\":" + GravityJson() + ",\"lattice\":" + LatticeJson());
		m_Output.Append(",\"decoding\":" + DecodingJson() + ",\"shells\":[");
		Print("[TBD Ballistics Oracle] Simulation run started: " + m_Run.Directory() + OUTPUT_NAME, LogLevel.NORMAL);
		GetGame().GetCallqueue().CallLater(Step, 0, false);
	}

	//! Samples one charge and elevation of the current shell, advances the cursor and queues the
	//! next step; finishes after the last shell or when the engine simulation is off.
	//! @authority server
	protected void Step()
	{
		if (m_iShell >= m_aShells.Count() || m_bUnavailable)
		{
			Finish();
			return;
		}

		TBD_BallisticsOracleShellSource shell = m_aShells[m_iShell];
		if (!m_bShellOpen && !OpenShell(shell))
		{
			CloseShell(shell);
			GetGame().GetCallqueue().CallLater(Step, 0, false);
			return;
		}

		float elevation = m_aElevations[m_iElevation];
		for (int wind = 0; wind < m_aWindSpeeds.Count(); wind++)
		{
			foreach (float height : m_aTargetHeights)
			{
				if (m_bShellHasSample)
					m_Output.Append(",");

				m_Output.Append(TBD_BallisticsOracleSimulationSampler.Sample(m_Move, shell, m_iCharge, elevation, m_aWindSpeeds[wind], m_aWindFrom[wind], height));
				m_bShellHasSample = true;
				m_iSamples++;
			}
		}

		m_iElevation++;
		if (m_iElevation >= m_aElevations.Count())
		{
			m_iElevation = 0;
			m_iCharge++;
		}

		if (m_iCharge >= shell.m_aChargeRings.Count())
			CloseShell(shell);

		GetGame().GetCallqueue().CallLater(Step, 0, false);
	}

	//! Opens the shell's JSON object, spawns the shell and checks that the engine simulation runs.
	//! Returns false, with the reason in the shell's errors, when the shell cannot be sampled.
	//! @authority server
	protected bool OpenShell(TBD_BallisticsOracleShellSource shell)
	{
		if (m_iShell > 0)
			m_Output.Append(",");

		m_Output.Append("{" + shell.MetadataMembers() + ",\"samples\":[");
		m_bShellOpen = true;
		m_bShellHasSample = false;
		m_iCharge = 0;
		m_iElevation = 0;
		if (!shell.Source() || shell.m_aChargeRings.IsEmpty() || shell.m_fInitSpeed <= 0)
		{
			shell.m_aErrors.Insert("The shell prefab did not read in full, so it is not simulated");
			return false;
		}

		m_Projectile = SpawnShell(shell.m_sPrefab);
		if (m_Projectile)
			m_Move = ProjectileMoveComponent.Cast(m_Projectile.FindComponent(ProjectileMoveComponent));

		if (!m_Move)
		{
			shell.m_aErrors.Insert("The spawned shell has no ProjectileMoveComponent");
			return false;
		}

		if (TBD_BallisticsOracleSimulationSampler.IsAvailable(m_Move, shell.m_fInitSpeed * shell.ChargeCoefficient(0)))
			return true;

		m_bUnavailable = true;
		m_aErrors.Insert("GetProjectileSimulationResult returned the launch point: enable projectile debugging and play again");
		return false;
	}

	//! Closes the shell's JSON object with its errors, deletes the spawned shell and moves to the
	//! next shell.
	//! @authority server
	protected void CloseShell(TBD_BallisticsOracleShellSource shell)
	{
		m_Output.Append("],\"shell_errors\":" + shell.ErrorsJson() + "}");
		if (m_Projectile)
			SCR_EntityHelper.DeleteEntityAndChildren(m_Projectile);

		m_Projectile = null;
		m_Move = null;
		m_bShellOpen = false;
		m_iShell++;
	}

	//! Writes the trailer (counts, errors, status, finish time), closes simulation.json and writes
	//! its sidecar.
	//! @authority server
	protected void Finish()
	{
		int shellErrors = 0;
		foreach (TBD_BallisticsOracleShellSource shell : m_aShells)
			shellErrors += shell.m_aErrors.Count();

		string status = "complete";
		if (m_bUnavailable)
			status = "simulation_unavailable";
		else if (shellErrors > 0 || !m_aErrors.IsEmpty())
			status = "incomplete";

		string errors = "[";
		for (int index = 0; index < m_aErrors.Count(); index++)
		{
			if (index > 0)
				errors += ",";

			errors += TBD_BallisticsOracleJson.Quote(m_aErrors[index]);
		}

		m_Output.Append("],\"sample_count\":" + m_iSamples.ToString() + ",\"shell_error_count\":" + shellErrors.ToString());
		m_Output.Append(",\"errors\":" + errors + "],\"status\":" + TBD_BallisticsOracleJson.Quote(status));
		m_Output.Append(",\"finished_at\":" + TBD_BallisticsOracleJson.Quote(TBD_BallisticsOracleJson.IsoNowUtc()) + "}\n");
		bool closed = m_Output.Close();
		bool sidecar = closed && m_Output.WriteSidecar(m_Run.Directory(), OUTPUT_NAME, DOCUMENT_TYPE, m_Run, status);
		string summary = string.Format("[TBD Ballistics Oracle] Simulation run %1: %2 samples, sha256 %3, sidecar %4", status, m_iSamples, m_Output.Sha256(), sidecar);
		Print(summary, LogLevel.NORMAL);
	}

	//! Spawns `prefab` 1000 m above the world origin, where nothing collides with it while it is
	//! sampled; returns null when it does not spawn.
	//! @authority server
	protected IEntity SpawnShell(ResourceName prefab)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource || !resource.IsValid())
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = Vector(0, 1000, 0);
		return GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
	}

	//! Returns the gravity object: `raw_vector` from PhysicsWorld.GetGravity (world axes) and its
	//! `magnitude_m_s2`, or nulls when the world entity is missing.
	protected string GravityJson()
	{
		IEntity world = GetGame().GetWorldEntity();
		if (!world)
		{
			m_aErrors.Insert("GetGame().GetWorldEntity() returned null, so the gravity is unknown");
			return "{\"source\":\"PhysicsWorld.GetGravity\",\"raw_vector\":null,\"magnitude_m_s2\":null}";
		}

		vector gravity = PhysicsWorld.GetGravity(world);
		string json = "{\"source\":\"PhysicsWorld.GetGravity\",\"raw_vector\":" + TBD_BallisticsOracleJson.Vector3(gravity);
		return json + ",\"magnitude_m_s2\":" + TBD_BallisticsOracleJson.Number(gravity.Length()) + "}";
	}

	//! Returns the case lattice: elevations, winds, target heights, azimuth, time limit and the
	//! time-of-flight bisection.
	protected string LatticeJson()
	{
		string json = "{\"elevations_deg\":" + NumbersJson(m_aElevations);
		json += ",\"wind_speeds_m_s\":" + NumbersJson(m_aWindSpeeds) + ",\"wind_from_deg\":" + NumbersJson(m_aWindFrom);
		json += ",\"target_heights_m\":" + NumbersJson(m_aTargetHeights);
		json += ",\"azimuth_deg\":" + TBD_BallisticsOracleJson.Number(TBD_BallisticsOracleSimulationSampler.AZIMUTH_DEG);
		json += ",\"launch_position_world\":[0,0,0]";
		json += ",\"max_simulation_time_s\":" + TBD_BallisticsOracleJson.Number(TBD_BallisticsOracleSimulationSampler.MAX_SIMULATION_TIME_S);
		json += ",\"time_of_flight_bisection_steps\":" + TBD_BallisticsOracleSimulationSampler.TIME_BISECTION_STEPS.ToString();
		return json + ",\"init_speed\":\"ShellMoveComponent InitSpeed x charge init_speed_coef\"}";
	}

	//! Returns how the raw result vector is read, as the samples decode it.
	protected string DecodingJson()
	{
		string json = "{\"raw_result\":\"world position (x east, y up, z north) where the simulation ends\"";
		json += ",\"downrange_m\":\"raw x sin(azimuth) + raw z cos(azimuth)\"";
		json += ",\"crossrange_m\":\"raw x cos(azimuth) - raw z sin(azimuth), right of the line of fire positive\"";
		json += ",\"height_m\":\"raw y, relative to the launch point\"";
		json += ",\"time_of_flight_s\":\"shortest max_simulation_time whose result is within 0.01 m of the full result\"";
		return json + ",\"check\":\"forward_angle_reference.range_m equals downrange_m in each calm level sample\"}";
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
