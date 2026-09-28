/**
 * @file TBD_BallisticsOracleRun.c
 * @brief The identity of one ballistics oracle run: generation id, game build, revision, start time.
 *
 * Role: names the output folder and the header every oracle output and sidecar carries, and hands
 * the export generation id from the Workbench plugin to the play-mode component.
 * Position: created by TBD_BallisticsOraclePlugin (edit mode) and TBD_BallisticsOracleSimulationComponent
 * (play mode); read by TBD_BallisticsOracleOutputFile and both writers.
 * State: the run's generation id, game build and start time, fixed at Begin.  Invariants: a
 * generation id is 1 to 64 characters of A-Z, a-z, 0-9, `_` and `-`, so it is always one safe path
 * segment; every output lives under `$profile:TBD_BallisticsOracle/<generation id>/`.
 */

//! One oracle run: the export generation it calibrates, the game build, the revision and the time.
class TBD_BallisticsOracleRun
{
	static const string PLUGIN_REVISION = "tbd-ballistics-oracle/2"; //!< oracle script revision, raised when an output's meaning or layout changes; JSON key `plugin_revision`
	static const string ROOT_DIRECTORY = "$profile:TBD_BallisticsOracle/"; //!< root of every oracle output
	static const string ACTIVE_GENERATION_FILE = "$profile:TBD_BallisticsOracle/active_generation.txt"; //!< generation id the plugin hands to play mode
	protected static const string ID_CHARACTERS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-"; //!< characters a generation id may hold
	protected static const int ID_MAX_LENGTH = 64; //!< longest accepted generation id

	string m_sGenerationId; //!< export generation id; JSON key `export_generation_id`
	string m_sGameBuild; //!< Game.GetBuildVersion, empty when unavailable; JSON key `game_build`
	string m_sRunAt; //!< UTC start time; JSON key `run_at`

	//! Starts a run for `generationId`: records the game build and the start time and creates the
	//! output folder. Returns false, and logs why, when the id is not a safe path segment.
	bool Begin(string generationId)
	{
		if (!IsValidGenerationId(generationId))
		{
			Print("[TBD Ballistics Oracle] Export generation id '" + generationId + "' is not 1 to 64 characters of A-Z, a-z, 0-9, _ or -", LogLevel.ERROR);
			return false;
		}

		m_sGenerationId = generationId;
		m_sGameBuild = string.Empty;
		if (GetGame())
			m_sGameBuild = GetGame().GetBuildVersion();

		m_sRunAt = TBD_BallisticsOracleJson.IsoNowUtc();
		FileIO.MakeDirectory("$profile:TBD_BallisticsOracle");
		FileIO.MakeDirectory(ROOT_DIRECTORY + generationId);
		return true;
	}

	//! Returns the run's output folder, ending in `/`.
	string Directory()
	{
		return ROOT_DIRECTORY + m_sGenerationId + "/";
	}

	//! Returns the header members every output and sidecar opens with, without braces:
	//! `export_generation_id`, `game_build` (null when unavailable), `plugin_revision`, `run_at`.
	string HeaderMembers()
	{
		string json = "\"export_generation_id\":" + TBD_BallisticsOracleJson.Quote(m_sGenerationId);
		json += ",\"game_build\":" + TBD_BallisticsOracleJson.QuoteOrNull(m_sGameBuild);
		json += ",\"plugin_revision\":" + TBD_BallisticsOracleJson.Quote(PLUGIN_REVISION);
		json += ",\"run_at\":" + TBD_BallisticsOracleJson.Quote(m_sRunAt);
		return json;
	}

	//! Returns true for 1 to 64 characters of A-Z, a-z, 0-9, `_` and `-`.
	static bool IsValidGenerationId(string generationId)
	{
		int length = generationId.Length();
		if (length < 1 || length > ID_MAX_LENGTH)
			return false;

		for (int index = 0; index < length; index++)
		{
			if (!ID_CHARACTERS.Contains(generationId.Get(index)))
				return false;
		}

		return true;
	}

	//! Records `generationId` as the generation the next play-mode run samples. Returns false when
	//! the file cannot be written.
	static bool WriteActiveGeneration(string generationId)
	{
		FileHandle handle = FileIO.OpenFile(ACTIVE_GENERATION_FILE, FileMode.WRITE);
		if (!handle)
			return false;

		int written = handle.Write(generationId);
		handle.Close();
		return written > 0;
	}

	//! Returns the generation id the plugin last recorded, trimmed, or an empty string when there is
	//! none.
	static string ReadActiveGeneration()
	{
		if (!FileIO.FileExists(ACTIVE_GENERATION_FILE))
			return string.Empty;

		FileHandle handle = FileIO.OpenFile(ACTIVE_GENERATION_FILE, FileMode.READ);
		if (!handle)
			return string.Empty;

		string text;
		handle.Read(text, ID_MAX_LENGTH + 8);
		handle.Close();
		text.TrimInPlace();
		return text;
	}
}
