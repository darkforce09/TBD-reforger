//! Thin structured event log for the TBD framework (T-181.14).
//!
//! One tag vocabulary, one call per event, no state. Everything an operator greps for during
//! an event goes out as `[TBD][<channel>] <event> key=value …`, so
//! `grep '\[TBD\]\[Validate\]' console.log` returns the whole validation pass and nothing else.
//!
//! Deliberately small. CRF solves the same problem with `CRF_LoggingManager` — 888 lines of
//! per-subsystem toggles, ring buffers and RPC fan-out for a framework that ships ten game
//! modes. TBD runs one event, on one server: it needs a fixed prefix, an explicit level and a
//! greppable shape. If this ever needs filtering, add a channel allowlist here — do not grow
//! it into a manager component.
//!
//! ENF-1: every call carries an explicit LogLevel and nothing here sits on a per-frame or
//! per-replication-tick path (the validator runs once per mission parse; the stage helper
//! once per transition).
class TBD_Log
{
	//! Fixed channel vocabulary. Prefer a constant over a literal at the call site so the set
	//! of greppable tags stays enumerable from one place.
	static const string CH_MISSION  = "Mission";  //!< Mission document fetch / parse / cache.
	static const string CH_VALIDATE = "Validate"; //!< TBD_MissionValidator findings and verdict.
	static const string CH_STAGE    = "Stage";    //!< Gamemode stage machine transitions.
	static const string CH_SAFESTART = "Safestart"; //!< T-181.17 warmup: damage-off, countdown, lift.

	//! Rule used by Banner(). Wide enough that it cannot be mistaken for a normal line.
	protected static const string RULE = "========================================================";

	//------------------------------------------------------------------------------------------------
	//! `[TBD][<channel>] <message>` — the one line shape everything else composes.
	protected static string Compose(string channel, string message)
	{
		return "[TBD][" + channel + "] " + message;
	}

	//------------------------------------------------------------------------------------------------
	//! Normal-level framework event.
	static void Event(string channel, string message)
	{
		Print(Compose(channel, message), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	//! Something is wrong but the round can still run.
	static void Warn(string channel, string message)
	{
		Print(Compose(channel, message), LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	//! Something is wrong and the caller is about to refuse to proceed.
	static void Error(string channel, string message)
	{
		Print(Compose(channel, message), LogLevel.ERROR);
	}

	//------------------------------------------------------------------------------------------------
	//! Structured event line: `[TBD][Mission] loaded id=msn_8f3a2c slots=18`.
	//! `keyValues` is a pre-built `k=v k=v` string — Enforce Script has no varargs, and a
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

	//------------------------------------------------------------------------------------------------
	//! `[TBD][Mission] loaded id=… name='…' slots=… source=backend|profile`
	static void MissionLoaded(string missionId, string name, int slotCount, string source)
	{
		Kv(CH_MISSION, "loaded", string.Format("id=%1 name='%2' slots=%3 source=%4",
			missionId, name, slotCount, source));
	}

	//------------------------------------------------------------------------------------------------
	//! `[TBD][Validate] mission result=PASS errors=0 warnings=2` — the single line an operator
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

	//------------------------------------------------------------------------------------------------
	//! `[TBD][Stage] LOADING -> LOBBY`.
	//! NOT yet wired: the only caller would be TBD_FrameworkManager.SetStage, which belongs to
	//! another slice. The exact one-line hook is recorded in the T-181.14 slice report.
	static void Stage(TBD_EGameStage from, TBD_EGameStage to)
	{
		Event(CH_STAGE, string.Format("%1 -> %2",
			typename.EnumToString(TBD_EGameStage, from),
			typename.EnumToString(TBD_EGameStage, to)));
	}

	//------------------------------------------------------------------------------------------------
	//! A rule an operator cannot scroll past. Reserved for load-blocking failures — using it
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
