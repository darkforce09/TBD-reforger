//! T-684 -- bind payload `missionParams[]` and expose Get(symbol) to consumers.
//! The Enfusion half of `mission.schema.json#/$defs/missionParam`.
//!
//! == What was missing ========================================================================
//! T-706 put `missionParams[]` on the wire (name, titleKey, values, displays, default).
//! Nothing in `apps/mod` bound the array or resolved a launch choice, so an authored
//! parameter never reached gameplay without re-baking the mission. This file is the reader.
//!
//! == Launch-selection surface (honest) =======================================================
//! There is no lobby on this build. Chosen values come from server config:
//! `$profile:TBD_MissionParams.json`, shape:
//!   { "selections": [ { "name": "time_of_day", "value": 2 } ] }
//! `name` is the consuming symbol (`missionParams[].name`). Absent file, unreadable
//! file, or a selection whose value is not in that row's `values[]` -> the authored
//! default (when that default is itself in `values[]`). No guessing.
//!
//! == Why authoredDefault, not a member named default ========================================
//! The wire key is `default`. Enforce Script reserves `default` (switch), so a field of
//! that name does not compile (measured: TBD_MissionParams.c "Syntax error" /
//! "Unexpected scope"). JsonLoadContext still binds BY MEMBER NAME for the other keys
//! on the primary parse. The integer itself is read on a second JsonLoadContext pass
//! via ReadValue("default", authored) -- the key is a STRING, not an identifier --
//! then stored on `authoredDefault`. Same array, same order (TBD_ObjectiveRules join).
//!
//! == Get(symbol) fail-closed =================================================================
//! Unknown symbol, empty symbol, a row with no `values[]`, or a default that is not
//! a member of `values[]` returns EMPTY (0) and logs. It does not pick values[0], a
//! neighbour name, or a stale override. EMPTY collides with a legitimate authored 0;
//! callers that must distinguish use Has(symbol).
//!
//! == Presence: the nested-ref / array landmine ===============================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` even when the JSON key
//! is ABSENT. `ref array<>` is the other shape: measured NULL when the key is absent
//! (TBD_MissionValidator second-pass header). Either way, `if (doc.missionParams)` is
//! not a presence test. Presence is Count() after allocate-on-absent. Missions that
//! author no parameters have Count() == 0 and boot unchanged.
//! `default: 0` is a real value, so authoredDefault initialises to ABSENT (-1e6).
//!
//! == displays[] pairing ======================================================================
//! When `displays` is present it must line up 1:1 with `values` (the DTAS description.ext
//! bug: a missing comma left values[] one short of texts[] and every later label mapped
//! to the wrong integer). A mismatch is a WARNING; Get still returns the integer. There
//! is no launcher UI here to index `displays`.
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. It proves the symbols exist. It cannot run a
//! round. Changing a launch selection without re-baking is a human checklist item.
//! @contract mission.schema.json#/properties/missionParams
//! @contract mission.schema.json#/$defs/missionParam

//------------------------------------------------------------------------------------------------
//! One authored launch parameter. Field names MUST equal the JSON keys (`JsonLoadContext`
//! binds by member NAME), except the wire key `default` -- see the file header.
//! Bound onto `TBD_MissionDocumentStruct.missionParams`.
class TBD_MissionParamStruct
{
	//! "key absent from JSON". A presence flag, not a magic value -- `0` is a legal
	//! launch value (see golden `time_of_day` dawn).
	static const int ABSENT = -1000000;

	string name;                    //!< Consuming symbol. Schema pattern ^[a-z][a-z0-9_]*$.
	string titleKey;                //!< i18n key for the launcher-facing title. Empty when omitted.
	ref array<int> values;          //!< Allowed integers (Arma `values[]`). Presence = Count().
	ref array<string> displays;     //!< Parallel labels (Arma `texts[]`). Presence = Count().
	int authoredDefault = ABSENT;   //!< Filled from wire key `default` by TBD_MissionParams.
}

//------------------------------------------------------------------------------------------------
//! One row of `$profile:TBD_MissionParams.json` `selections[]`.
class TBD_MissionParamSelectionStruct
{
	string name;  //!< Consuming symbol, same spelling as `TBD_MissionParamStruct.name`.
	int value;    //!< Chosen integer. Rejected unless it is in the matching row's `values[]`.
}

//------------------------------------------------------------------------------------------------
//! Server-config file. Absent / unreadable / empty `selections` -> authored defaults.
class TBD_MissionParamLaunchFile
{
	ref array<ref TBD_MissionParamSelectionStruct> selections;
}

//------------------------------------------------------------------------------------------------
//! Resolves launch selection at mission parse and exposes Get(symbol) to consumers.
class TBD_MissionParams
{
	//! Documented fail-closed return for Get(symbol) when the symbol is unknown or the
	//! row cannot be honoured. Collides with a legitimate 0 -- use Has(symbol).
	static const int EMPTY = 0;

	static const string CONFIG_PATH = "$profile:TBD_MissionParams.json";

	protected static ref map<string, int> s_Chosen;
	protected static ref map<string, bool> s_Known;

	//------------------------------------------------------------------------------------------------
	//! Called from `TBD_MissionLoader.ParseMissionJson` after a valid parse, on the
	//! server-only load path. Also safe to call from Get/Has (idempotent). No-ops when
	//! `missionParams` is empty, so missions without parameters boot unchanged.
	static void Resolve()
	{
		s_Chosen = new map<string, int>();
		s_Known = new map<string, bool>();

		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!mission)
			return;

		if (!mission.missionParams)
			mission.missionParams = new array<ref TBD_MissionParamStruct>();

		int n = mission.missionParams.Count();
		if (n == 0)
			return;

		FillAuthoredDefaults(mission.missionParams);

		map<string, int> launch = new map<string, int>();
		LoadLaunchSelections(launch);

		int i;
		for (i = 0; i < n; i++)
		{
			TBD_MissionParamStruct row = mission.missionParams.Get(i);
			ResolveRow(row, launch);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Integer chosen for `symbol` at launch (server config, else authored default).
	//! Unknown / unusable symbol -> EMPTY, never a guess.
	static int Get(string symbol)
	{
		if (!s_Known)
			Resolve();

		if (symbol.IsEmpty())
		{
			Print("[TBD][MissionParams] Get() empty symbol -- fail closed", LogLevel.WARNING);
			return EMPTY;
		}

		if (!s_Known)
			return EMPTY;

		bool known;
		if (!s_Known.Find(symbol, known))
		{
			Print(string.Format("[TBD][MissionParams] Get('%1') unknown symbol -- fail closed", symbol), LogLevel.WARNING);
			return EMPTY;
		}

		if (!known)
		{
			Print(string.Format("[TBD][MissionParams] Get('%1') unknown symbol -- fail closed", symbol), LogLevel.WARNING);
			return EMPTY;
		}

		int value;
		if (!s_Chosen.Find(symbol, value))
			return EMPTY;

		return value;
	}

	//------------------------------------------------------------------------------------------------
	//! True when Resolve honoured this consuming symbol (chosen value is in `values[]`).
	static bool Has(string symbol)
	{
		if (!s_Known)
			Resolve();

		if (symbol.IsEmpty() || !s_Known)
			return false;

		bool known;
		if (!s_Known.Find(symbol, known))
			return false;

		return known;
	}

	//------------------------------------------------------------------------------------------------
	//! Authored row count. Presence of the array: this number, never a null check.
	static int Count()
	{
		TBD_MissionDocumentStruct mission = TBD_MissionLoader.GetMission();
		if (!mission)
			return 0;

		if (!mission.missionParams)
			return 0;

		return mission.missionParams.Count();
	}

	//------------------------------------------------------------------------------------------------
	//! Second JsonLoadContext pass: read the wire key `default` (a reserved word, so it
	//! cannot be a class member) by STRING name, joined to the primary parse by index.
	protected static void FillAuthoredDefaults(array<ref TBD_MissionParamStruct> rows)
	{
		if (!rows)
			return;

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
			return;

		int count;
		if (!ctx.StartArray("missionParams", count))
			return;

		int n = rows.Count();
		int i;
		for (i = 0; i < count; i++)
		{
			if (!ctx.StartObject(""))
				break;

			int authored = TBD_MissionParamStruct.ABSENT;
			string rowName;
			ctx.ReadValue("name", rowName);
			if (!ctx.ReadValue("default", authored))
				authored = TBD_MissionParamStruct.ABSENT;

			if (i < n)
			{
				TBD_MissionParamStruct row = rows.Get(i);
				if (row)
				{
					if (rowName.IsEmpty())
						row.authoredDefault = authored;
					else if (rowName == row.name)
						row.authoredDefault = authored;
				}
			}

			ctx.EndObject();
		}

		ctx.EndArray();
	}

	//------------------------------------------------------------------------------------------------
	protected static void ResolveRow(TBD_MissionParamStruct row, map<string, int> launch)
	{
		if (!row)
			return;

		if (row.name.IsEmpty())
		{
			Print("[TBD][MissionParams] row with empty name -- skipped", LogLevel.WARNING);
			return;
		}

		if (!row.values)
			row.values = new array<int>();

		if (row.values.Count() == 0)
		{
			Print(string.Format("[TBD][MissionParams] name='%1' has empty values[] -- fail closed", row.name), LogLevel.WARNING);
			return;
		}

		WarnDisplaysMismatch(row);

		bool already;
		if (s_Known.Find(row.name, already))
		{
			Print(string.Format("[TBD][MissionParams] duplicate name='%1' -- first wins", row.name), LogLevel.WARNING);
			return;
		}

		int launchVal;
		bool haveLaunch = false;
		if (launch && launch.Find(row.name, launchVal))
			haveLaunch = true;

		int chosen;
		string source;
		if (haveLaunch && InValues(row, launchVal))
		{
			chosen = launchVal;
			source = "launch";
		}
		else
		{
			if (haveLaunch)
			{
				Print(string.Format("[TBD][MissionParams] name='%1' launch value=%2 not in values[] -- authored default", row.name, launchVal), LogLevel.WARNING);
			}

			if (row.authoredDefault == TBD_MissionParamStruct.ABSENT)
			{
				Print(string.Format("[TBD][MissionParams] name='%1' default absent -- fail closed", row.name), LogLevel.WARNING);
				return;
			}

			if (!InValues(row, row.authoredDefault))
			{
				Print(string.Format("[TBD][MissionParams] name='%1' default=%2 not in values[] -- fail closed", row.name, row.authoredDefault), LogLevel.WARNING);
				return;
			}

			chosen = row.authoredDefault;
			source = "default";
		}

		s_Chosen.Set(row.name, chosen);
		s_Known.Set(row.name, true);
		Print(string.Format("[TBD][MissionParams] symbol='%1' value=%2 source=%3", row.name, chosen, source), LogLevel.NORMAL);
	}

	//------------------------------------------------------------------------------------------------
	protected static void WarnDisplaysMismatch(TBD_MissionParamStruct row)
	{
		if (!row.displays)
			return;

		int nDisp = row.displays.Count();
		if (nDisp == 0)
			return;

		int nVal = row.values.Count();
		if (nDisp == nVal)
			return;

		Print(string.Format("[TBD][MissionParams] name='%1' displays=%2 values=%3 -- labels will not pair", row.name, nDisp, nVal), LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	protected static bool InValues(TBD_MissionParamStruct row, int v)
	{
		if (!row.values)
			return false;

		int n = row.values.Count();
		int i;
		for (i = 0; i < n; i++)
		{
			if (row.values.Get(i) == v)
				return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Absent file is the common case (authored defaults). A present but unreadable file
	//! must not invent selections -- fail closed onto authored defaults.
	protected static void LoadLaunchSelections(map<string, int> into)
	{
		if (!into)
			return;

		if (!FileIO.FileExists(CONFIG_PATH))
			return;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromFile(CONFIG_PATH))
		{
			Print("[TBD][MissionParams] failed to read $profile:TBD_MissionParams.json -- authored defaults", LogLevel.ERROR);
			return;
		}

		TBD_MissionParamLaunchFile file = new TBD_MissionParamLaunchFile();
		if (!ctx.ReadValue("", file))
		{
			Print("[TBD][MissionParams] failed to parse $profile:TBD_MissionParams.json -- authored defaults", LogLevel.ERROR);
			return;
		}

		if (!file.selections)
			return;

		int n = file.selections.Count();
		int i;
		for (i = 0; i < n; i++)
		{
			TBD_MissionParamSelectionStruct sel = file.selections.Get(i);
			if (!sel)
				continue;

			if (sel.name.IsEmpty())
				continue;

			int existing;
			if (into.Find(sel.name, existing))
			{
				Print(string.Format("[TBD][MissionParams] launch config duplicate name='%1' -- first wins", sel.name), LogLevel.WARNING);
				continue;
			}

			into.Set(sel.name, sel.value);
		}
	}
}
