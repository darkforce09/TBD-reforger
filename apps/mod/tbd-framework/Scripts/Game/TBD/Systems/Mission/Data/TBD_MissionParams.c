/**
 * @file TBD_MissionParams.c
 * @brief Binds `missionParams[]` and resolves the launch value of each parameter symbol.
 *
 * Role: resolves every authored launch parameter to one integer at mission parse and serves it
 * by symbol through `Get` and `Has`.  Position: `Resolve` runs from `TBD_MissionLoader` after a
 * valid parse; reads the loader's document, a second `JsonLoadContext` pass for each row's
 * `default`, and the server config `$profile:TBD_MissionParams.json`, shaped
 * `{ "selections": [ { "name": "time_of_day", "value": 2 } ] }`.
 * State: the static chosen and known maps, server only.  Invariants: a launch selection counts
 * only when it is in the row's `values[]`, else the authored default does when it is; an unknown
 * symbol, an empty `values[]` or an unusable default returns `EMPTY` (0) and logs, never
 * `values[0]` or a neighbour; `Has` tells a resolved 0 from `EMPTY`. The wire key `default` is an
 * Enforce keyword, so it is read by string on the second pass into `authoredDefault`, joined to
 * the primary parse by index. A `displays[]` whose length differs from `values[]` logs a WARNING.
 */

//! One authored launch parameter. Field names MUST equal the JSON keys (`JsonLoadContext`
//! binds by member NAME), except the wire key `default` -- see the file header.
//! Bound onto `TBD_MissionDocumentStruct.missionParams`.
//! @contract mission.schema.json#/$defs/missionParam
class TBD_MissionParamStruct
{
	//! A presence flag, not a magic value -- `0` is a legal launch value (see golden
	//! `time_of_day` dawn).
	static const int ABSENT = -1000000; //!< "`default` absent from JSON"

	string name;                    //!< Consuming symbol. Schema pattern ^[a-z][a-z0-9_]*$.
	string titleKey;                //!< i18n key for the launcher-facing title. Empty when omitted.
	ref array<int> values;          //!< Allowed integers (Arma `values[]`). Presence = Count().
	ref array<string> displays;     //!< Parallel labels (Arma `texts[]`). Presence = Count().
	int authoredDefault = ABSENT;   //!< JSON key `default`, read by TBD_MissionParams in a second pass.
}

//! One row of `$profile:TBD_MissionParams.json` `selections[]`; the server config, not the
//! mission schema.
class TBD_MissionParamSelectionWire
{
	string name;  //!< Consuming symbol, same spelling as `TBD_MissionParamStruct.name`.
	int value;    //!< Chosen integer. Rejected unless it is in the matching row's `values[]`.
}

//! Server-config file. Absent / unreadable / empty `selections` -> authored defaults.
class TBD_MissionParamLaunchFile
{
	ref array<ref TBD_MissionParamSelectionWire> selections; //!< `selections[]`; null when absent
}

//! Resolves launch selection at mission parse and exposes Get(symbol) to consumers.
class TBD_MissionParams
{
	//! Documented fail-closed return for Get(symbol) when the symbol is unknown or the
	//! row cannot be honoured. Collides with a legitimate 0 -- use Has(symbol).
	static const int EMPTY = 0; //!< fail-closed value of `Get`

	static const string CONFIG_PATH = "$profile:TBD_MissionParams.json"; //!< server config holding the launch selections

	protected static ref map<string, int> s_Chosen; //!< resolved value per symbol
	protected static ref map<string, bool> s_Known; //!< symbols `Resolve` honoured; null before the first `Resolve`

	//! Resolve every row to its launch value. Called by `TBD_MissionLoader` after a valid parse
	//! and by `Get`/`Has` before the first resolve; idempotent. Does nothing when `missionParams`
	//! is empty or no mission is loaded.
	//! @authority server
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

	//! Integer chosen for `symbol` at launch (server config, else authored default).
	//! @return the value, or `EMPTY` with a WARNING for an empty or unknown symbol
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

	//! True when `Resolve` honoured this consuming symbol (chosen value is in `values[]`).
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

	//! Second JsonLoadContext pass: read the wire key `default` (a reserved word, so it
	//! cannot be a class member) by STRING name, joined to the primary parse by index.
	protected static void FillAuthoredDefaults(array<ref TBD_MissionParamStruct> rows)
	{
		if (!rows)
			return;

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (!ctx)
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

	//! Resolve one row into `s_Chosen`/`s_Known`: the launch value when it is in `values[]`, else
	//! the authored default when that is. An empty name, empty `values[]`, a duplicate name or an
	//! unusable default logs a WARNING and leaves the symbol unresolved.
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

	//! Log a WARNING when a present `displays[]` does not pair 1:1 with `values[]`.
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

	//! Whether `v` is one of the row's `values[]`.
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

	//! Read the launch selections of `CONFIG_PATH` into `into`, first name wins. An absent file is
	//! the common case (authored defaults); a present but unreadable file logs an ERROR and adds
	//! nothing.
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
			TBD_MissionParamSelectionWire sel = file.selections.Get(i);
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
