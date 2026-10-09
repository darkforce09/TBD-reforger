/**
 * @file TBD_BallisticsOraclePlugin.c
 * @brief Workbench menu entry Plugins > TBD > Ballistics Oracle: writes forward_angles.json.
 *
 * Role: asks for the export generation id the oracle calibrates, writes the edit-mode oracle
 * output (forward_angles.json and its sidecar) under
 * `$profile:TBD_BallisticsOracle/<generation id>/`, and records the generation id for the
 * play-mode simulation run.  Position: started from the Workbench menu; runs
 * TBD_BallisticsOracleForwardAngles; TBD_BallisticsOracleSimulationComponent reads the recorded id
 * when the export world plays.
 * State: the generation id typed into the dialog, per plugin instance.  Invariants: nothing is
 * written for an invalid id or a failed SHA-256 self-test; the id is recorded for play mode only
 * after forward_angles.json and its sidecar are written.
 */

//! The Ballistics Oracle menu entry.
[WorkbenchPluginAttribute(name: "Ballistics Oracle", description: "Samples the engine ballistics of the vanilla mortar shells into $profile:TBD_BallisticsOracle/<export generation id>/", category: "TBD", wbModules: {"ResourceManager", "WorldEditor"})]
class TBD_BallisticsOraclePlugin : WorkbenchPlugin
{
	[Attribute("", UIWidgets.EditBox, desc: "Export generation id the oracle calibrates: the equipment export generation the ballistics catalog is trimmed from")]
	protected string m_sExportGenerationId; //!< generation id typed into the dialog; names the output folder

	//! Shows the dialog and, on Run, writes forward_angles.json and records the generation id.
	override void Run()
	{
		if (!Workbench.ScriptDialog("Ballistics Oracle", "Samples BallisticTable for the vanilla mortar shells into $profile:TBD_BallisticsOracle/<export generation id>/forward_angles.json.\nThen play the export world to write simulation.json.", this))
			return;

		string generationId = m_sExportGenerationId;
		generationId.TrimInPlace();
		TBD_BallisticsOracleRun run = new TBD_BallisticsOracleRun();
		if (!run.Begin(generationId))
			return;

		if (!TBD_BallisticsOracleSha256.SelfTest())
		{
			Print("[TBD Ballistics Oracle] SHA-256 self-test failed; no output written", LogLevel.ERROR);
			return;
		}

		int startMs = System.GetTickCount();
		TBD_BallisticsOracleForwardAngles forwardAngles = new TBD_BallisticsOracleForwardAngles();
		string status = forwardAngles.Write(run);
		if (status == "failed")
		{
			Print("[TBD Ballistics Oracle] forward_angles.json was not written in full; the generation is not recorded for play mode", LogLevel.ERROR);
			return;
		}

		if (!TBD_BallisticsOracleRun.WriteActiveGeneration(generationId))
			Print("[TBD Ballistics Oracle] Cannot write " + TBD_BallisticsOracleRun.ACTIVE_GENERATION_FILE + "; play mode will not run", LogLevel.ERROR);

		int elapsedMs = System.GetTickCount() - startMs;
		string summary = string.Format("[TBD Ballistics Oracle] forward_angles.json %1 in %2 ms, sha256 %3, folder %4", status, elapsedMs, forwardAngles.Sha256(), run.Directory());
		Print(summary, LogLevel.NORMAL);
	}

	//! Dialog button that starts the run.
	[ButtonAttribute("Run", true)]
	protected int ButtonRun()
	{
		return 1;
	}

	//! Dialog button that closes the dialog without running.
	[ButtonAttribute("Cancel")]
	protected int ButtonCancel()
	{
		return 0;
	}
}
