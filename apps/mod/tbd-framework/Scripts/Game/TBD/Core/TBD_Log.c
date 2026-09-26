/**
 * @file TBD_Log.c
 * @brief Thin structured event log for the TBD framework: `[TBD][<channel>] <event> key=value ...`.
 *
 * Role: writes every framework log line in one greppable shape with an explicit level, so
 * `grep '\[TBD\]\[Validate\]' console.log` returns a whole validation pass and nothing else.
 * Position: called by every framework module on client and server.
 * State: none.
 * Invariants: every call carries an explicit LogLevel; nothing here sits on a per-frame or
 * per-replication-tick path (the validator runs once per mission parse, the stage helper once per
 * transition).
 */

//! Structured log helpers. One tag vocabulary, one call per event, no state: TBD runs one event
//! on one server and needs a fixed prefix, an explicit level and a greppable shape, not a logging
//! manager with per-subsystem toggles, ring buffers and RPC fan-out. Filtering, when needed, is a
//! channel allowlist here.
class TBD_Log
{
	//! Fixed channel vocabulary. Prefer a constant over a literal at the call site so the set
	//! of greppable tags stays enumerable from one place.
	static const string CH_MISSION  = "Mission";  //!< Mission document fetch / parse / cache.
	static const string CH_VALIDATE = "Validate"; //!< TBD_MissionValidator findings and verdict.
	static const string CH_STAGE    = "Stage";    //!< Gamemode stage machine transitions.
	static const string CH_SAFESTART = "Safestart"; //!< safe-start warmup: damage off, countdown, lift

	//! Rule used by Banner(). Wide enough that it cannot be mistaken for a normal line.
	protected static const string RULE = "========================================================"; //!< the banner rule line

	//! `[TBD][<channel>] <message>` -- the one line shape everything else composes.
	protected static string Compose(string channel, string message)
	{
		return "[TBD][" + channel + "] " + message;
	}

	//! Normal-level framework event.
	static void Event(string channel, string message)
	{
		Print(Compose(channel, message), LogLevel.NORMAL);
	}

	//! Something is wrong but the round can still run.
	static void Warn(string channel, string message)
	{
		Print(Compose(channel, message), LogLevel.WARNING);
	}

	//! Something is wrong and the caller is about to refuse to proceed.
	static void Error(string channel, string message)
	{
		Print(Compose(channel, message), LogLevel.ERROR);
	}

	//! Structured event line: `[TBD][Mission] loaded id=msn_8f3a2c slots=18`.
	//! `keyValues` is a pre-built `k=v k=v` string -- Enforce Script has no varargs, and a
	//! key/value builder object would cost more than it saves at this scale.
	static void Kv(string channel, string eventName, string keyValues)
	{
		if (keyValues.IsEmpty())
		{
			Event(channel, eventName);
			return;
		}

		Event(channel, eventName + " " + keyValues);
	}

	//! `[TBD][Mission] loaded id=... name='...' slots=... source=platform|cache|last-verified-cache`:
	//! the deployment's artifact fetched and verified, the deployment's artifact from the profile
	//! cache, or the last verified artifact because the deployment could not be read
	//! (TBD_DeployedMission).
	static void MissionLoaded(string missionId, string name, int slotCount, string source)
	{
		Kv(CH_MISSION, "loaded", string.Format("id=%1 name='%2' slots=%3 source=%4",
			missionId, name, slotCount, source));
	}

	//! `[TBD][Validate] mission result=PASS errors=0 warnings=2` -- the single line an operator
	//! (or a log scraper) reads to know whether the mission is loadable. A failure is logged at
	//! ERROR so it survives a level filter.
	static void ValidationResult(bool passed, int errorCount, int warningCount)
	{
		string verdict = "FAIL";
		if (passed)
			verdict = "PASS";

		string line = string.Format("mission result=%1 errors=%2 warnings=%3", verdict, errorCount, warningCount);
		if (passed)
		{
			Event(CH_VALIDATE, line);
			return;
		}

		Error(CH_VALIDATE, line);
	}

	//! `[TBD][Stage] LOADING -> LOBBY`.
	//! Wired at TBD_FrameworkManager.SetStage, logged before the subsystem fan-out so
	//! the transition line precedes whatever the subsystems say about it. SetStage also keeps the
	//! `[TBD] Stage <arrow> <stage>` Print verbatim (README.md and
	//! documentation_v2/runbooks/game_server_staging/README.md quote it), so both
	//! formats appear on every transition -- detectors should accept either and never depend on the
	//! non-ASCII arrow (`cargo xtask mod remote-logs` pins the prefix only).
	static void Stage(TBD_EGameStage from, TBD_EGameStage to)
	{
		Event(CH_STAGE, string.Format("%1 -> %2",
			typename.EnumToString(TBD_EGameStage, from),
			typename.EnumToString(TBD_EGameStage, to)));
	}

	//! A rule an operator cannot scroll past. Reserved for load-blocking failures -- using it
	//! for anything routine destroys the signal it exists to carry.
	static void Banner(string channel, string title, bool isError)
	{
		if (isError)
		{
			Error(channel, RULE);
			Error(channel, title);
			Error(channel, RULE);
			return;
		}

		Event(channel, RULE);
		Event(channel, title);
		Event(channel, RULE);
	}
}
