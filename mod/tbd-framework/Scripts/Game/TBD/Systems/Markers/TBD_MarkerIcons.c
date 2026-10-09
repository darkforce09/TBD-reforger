/**
 * @file TBD_MarkerIcons.c
 * @brief Resolves a mission marker's authored `icon` string to a placed-marker icon entry index.
 *
 * Role: bridges authored icon names to `SCR_MapMarkerBase.SetIconEntry(int)`, which takes an index
 * into the placed-marker icon array of the vanilla `MapMarkerConfig.conf`.  Position: called by
 * `TBD_MarkerApplier` and `TBD_TaskHud`; reads the live config through
 * `TBD_MarkerClient.FindMarkerManager`.
 * State: static lookup caches and report latches on the client, dropped by `ResetForWorld`.
 * Invariants: `Resolve` never fails: the engine's own quad names win over the alias table, and an
 * empty or unknown name draws `FALLBACK_ICON`; the alias table maps friendly words and the
 * compile-verified `SCR_EScenarioFrameworkMarkerCustom` members, whose values are the config
 * indices (vanilla's `SCR_BaseTutorialStage.CreateMarkerCustom` uses them that way); an index
 * outside the loaded config is clamped to 0 with a warning. The schema's `#/$defs/marker.icon`
 * enum lists the alias keys; the runtime also accepts engine quad names outside it.
 */

//! Marker icon vocabulary.
//! @authority client
class TBD_MarkerIcons
{
	static const int FALLBACK_ICON = SCR_EScenarioFrameworkMarkerCustom.DOT; //!< icon drawn for an empty or unknown name
	static const int MARKER_COLOR = SCR_EScenarioFrameworkMarkerCustomColor.REFORGER_ORANGE; //!< palette entry for a marker without an authored colour

	protected static ref map<string, int> s_mAliases; //!< normalised alias -> icon entry index; null until first use
	protected static ref map<string, int> s_mConfigQuads; //!< normalised quad name -> icon entry index from the live config; null until first use
	protected static int s_iConfigIconCount = -1; //!< placed icons in the live config; -1 not read, 0 unreadable
	protected static ref map<string, bool> s_mReported; //!< normalised unknown names already reported
	protected static bool s_bVocabularyDumped; //!< true once the accepted vocabulary was logged

	//! Trim, lower-case and fold `-` and space to `_`. `ToLower` and `Replace` mutate in place.
	//! @return the normalised key
	static string Normalise(string raw)
	{
		string key = raw;
		key.TrimInPlace();
		key.ToLower();
		key.Replace("-", "_");
		key.Replace(" ", "_");

		return key;
	}

	//! Resolve an authored icon: the live config's quad names first, then the alias table.
	//! @param authoredIcon the marker's `icon`
	//! @param recognised set true when either source knows the name
	//! @return the icon entry index, or `FALLBACK_ICON` for an empty or unknown name
	static int Resolve(string authoredIcon, out bool recognised)
	{
		recognised = false;

		string key = Normalise(authoredIcon);
		if (key.IsEmpty())
			return FALLBACK_ICON;

		EnsureConfigQuads();

		int entry;
		if (s_mConfigQuads.Find(key, entry))
		{
			recognised = true;
			return entry;
		}

		EnsureAliases();

		if (!s_mAliases.Find(key, entry))
			return FALLBACK_ICON;

		recognised = true;
		return entry;
	}

	//! Report an icon name that could not be placed, once per normalised name: an empty name logs
	//! one NORMAL line; an unknown one warns and dumps the accepted vocabulary once. Never ERROR.
	static void ReportUnknown(string authoredIcon)
	{
		EnsureReported();

		string key = Normalise(authoredIcon);

		bool seen;
		if (s_mReported.Find(key, seen))
			return;

		s_mReported.Set(key, true);

		// An empty icon means the document bypassed schema validation; it gets no vocabulary dump.
		if (key.IsEmpty())
		{
			TBD_Log.Event(TBD_MarkerService.CH_MARKERS,
				"a marker was authored with no icon -- drew the default dot.");
			return;
		}

		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS, string.Format(
			"icon '%1' is not a known marker icon -- drew the fallback dot instead. The marker itself is still on the map.",
			authoredIcon));

		DumpVocabularyOnce();
	}

	//! Let every unknown name and the vocabulary report again (a new mission).
	static void ResetReported()
	{
		s_mReported = null;
		s_bVocabularyDumped = false;
	}

	//! Drop the report latches and the cached config read; the cache holds indices into an array
	//! owned by a component that dies with the world.
	static void ResetForWorld()
	{
		ResetReported();
		s_mConfigQuads = null;
		s_iConfigIconCount = -1;
	}

	//! Log everything `Resolve` accepts, once until `ResetReported`: one line of engine quad names
	//! (packed vanilla data, published from the machine that loaded it) and one of aliases.
	static void DumpVocabularyOnce()
	{
		if (s_bVocabularyDumped)
			return;

		s_bVocabularyDumped = true;

		EnsureConfigQuads();
		EnsureAliases();

		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
			string.Format("engine icon names (%1): %2", s_mConfigQuads.Count(), JoinKeys(s_mConfigQuads)));
		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
			string.Format("TBD icon aliases (%1): %2", s_mAliases.Count(), JoinKeys(s_mAliases)));
	}

	//! Comma-separated keys of a lookup table, for the vocabulary dump.
	protected static string JoinKeys(map<string, int> table)
	{
		string list;
		foreach (string key, int entry : table)
		{
			if (!list.IsEmpty())
			{
				// Appended in steps: a long `+` chain hits `Formula too complex`.
				list = list + ", ";
			}

			list = list + key;
		}

		return list;
	}

	//! Check a resolved index against the loaded icon array, so an out-of-range index warns
	//! instead of drawing a blank marker.
	//! @return `entry` when in range or when the config is unreadable; otherwise 0, with a warning
	static int ClampToLoadedConfig(int entry)
	{
		EnsureConfigQuads();

		// Config unreadable: nothing to check against.
		if (s_iConfigIconCount <= 0)
			return entry;

		if (entry >= 0 && entry < s_iConfigIconCount)
			return entry;

		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
			string.Format("icon entry %1 is outside the %2 icons this game build loaded -- using 0.",
				entry, s_iConfigIconCount));

		return 0;
	}

	//! Read the live placed-marker icon list once and index it by normalised quad name. Without a
	//! reachable marker system the table stays empty, the count 0, and lookups use aliases alone.
	protected static void EnsureConfigQuads()
	{
		if (s_mConfigQuads)
			return;

		s_mConfigQuads = new map<string, int>();
		s_iConfigIconCount = 0;

		SCR_MapMarkerManagerComponent mgr = TBD_MarkerClient.FindMarkerManager();
		if (!mgr)
			return;

		SCR_MapMarkerConfig cfg = mgr.GetMarkerConfig();
		if (!cfg)
			return;

		SCR_MapMarkerEntryPlaced placed = SCR_MapMarkerEntryPlaced.Cast(
			cfg.GetMarkerEntryConfigByType(SCR_EMapMarkerType.PLACED_CUSTOM));
		if (!placed)
			return;

		array<ref SCR_MarkerIconEntry> icons = placed.GetIconEntries();
		if (!icons)
			return;

		s_iConfigIconCount = icons.Count();

		for (int i = 0; i < s_iConfigIconCount; i++)
		{
			SCR_MarkerIconEntry icon = icons[i];
			if (!icon)
				continue;

			ResourceName imageset;
			ResourceName imagesetGlow;
			string quad;
			icon.GetIconResource(imageset, imagesetGlow, quad);

			string key = Normalise(quad);
			if (key.IsEmpty())
				continue;

			// First wins: the lower index is the one vanilla's selection menu shows first.
			if (s_mConfigQuads.Contains(key))
				continue;

			s_mConfigQuads.Set(key, i);
		}
	}

	//! Build the alias table once: every compile-verified `SCR_EScenarioFrameworkMarkerCustom`
	//! member under its own name, then the friendly words a mission maker is likely to type.
	protected static void EnsureAliases()
	{
		if (s_mAliases)
			return;

		s_mAliases = new map<string, int>();

		Register("dot", SCR_EScenarioFrameworkMarkerCustom.DOT);
		Register("dot2", SCR_EScenarioFrameworkMarkerCustom.DOT2);
		Register("objective_marker", SCR_EScenarioFrameworkMarkerCustom.OBJECTIVE_MARKER);
		Register("objective_marker2", SCR_EScenarioFrameworkMarkerCustom.OBJECTIVE_MARKER2);
		Register("point_of_interest", SCR_EScenarioFrameworkMarkerCustom.POINT_OF_INTEREST);
		Register("point_of_interest2", SCR_EScenarioFrameworkMarkerCustom.POINT_OF_INTEREST2);
		Register("observation_post", SCR_EScenarioFrameworkMarkerCustom.OBSERVATION_POST);
		Register("observation_post2", SCR_EScenarioFrameworkMarkerCustom.OBSERVATION_POST2);
		Register("destroy", SCR_EScenarioFrameworkMarkerCustom.DESTROY);
		Register("destroy2", SCR_EScenarioFrameworkMarkerCustom.DESTROY2);
		Register("attack", SCR_EScenarioFrameworkMarkerCustom.ATTACK);
		Register("defend", SCR_EScenarioFrameworkMarkerCustom.DEFEND);
		Register("defend2", SCR_EScenarioFrameworkMarkerCustom.DEFEND2);
		Register("waypoint", SCR_EScenarioFrameworkMarkerCustom.WAYPOINT);
		Register("waypoint2", SCR_EScenarioFrameworkMarkerCustom.WAYPOINT2);
		Register("ambush", SCR_EScenarioFrameworkMarkerCustom.AMBUSH);
		Register("ambush2", SCR_EScenarioFrameworkMarkerCustom.AMBUSH2);
		Register("flag", SCR_EScenarioFrameworkMarkerCustom.FLAG);
		Register("flag2", SCR_EScenarioFrameworkMarkerCustom.FLAG2);
		Register("cross", SCR_EScenarioFrameworkMarkerCustom.CROSS);
		Register("cross2", SCR_EScenarioFrameworkMarkerCustom.CROSS2);
		Register("circle", SCR_EScenarioFrameworkMarkerCustom.CIRCLE);
		Register("circle2", SCR_EScenarioFrameworkMarkerCustom.CIRCLE2);

		// Friendly words a mission maker is likely to type.
		Register("objective", SCR_EScenarioFrameworkMarkerCustom.OBJECTIVE_MARKER);
		Register("obj", SCR_EScenarioFrameworkMarkerCustom.OBJECTIVE_MARKER);
		Register("target", SCR_EScenarioFrameworkMarkerCustom.OBJECTIVE_MARKER);
		Register("task", SCR_EScenarioFrameworkMarkerCustom.OBJECTIVE_MARKER);

		Register("assault", SCR_EScenarioFrameworkMarkerCustom.ATTACK);
		Register("capture", SCR_EScenarioFrameworkMarkerCustom.ATTACK);
		Register("seize", SCR_EScenarioFrameworkMarkerCustom.ATTACK);
		Register("advance", SCR_EScenarioFrameworkMarkerCustom.ATTACK);

		Register("hold", SCR_EScenarioFrameworkMarkerCustom.DEFEND);
		Register("garrison", SCR_EScenarioFrameworkMarkerCustom.DEFEND);
		Register("fallback", SCR_EScenarioFrameworkMarkerCustom.DEFEND);

		Register("demolish", SCR_EScenarioFrameworkMarkerCustom.DESTROY);
		Register("demo", SCR_EScenarioFrameworkMarkerCustom.DESTROY);
		Register("sabotage", SCR_EScenarioFrameworkMarkerCustom.DESTROY);

		Register("move", SCR_EScenarioFrameworkMarkerCustom.WAYPOINT);
		Register("wp", SCR_EScenarioFrameworkMarkerCustom.WAYPOINT);
		Register("route", SCR_EScenarioFrameworkMarkerCustom.WAYPOINT);
		Register("phase_line", SCR_EScenarioFrameworkMarkerCustom.WAYPOINT);

		Register("poi", SCR_EScenarioFrameworkMarkerCustom.POINT_OF_INTEREST);
		Register("intel", SCR_EScenarioFrameworkMarkerCustom.POINT_OF_INTEREST);
		Register("contact", SCR_EScenarioFrameworkMarkerCustom.POINT_OF_INTEREST);

		Register("op", SCR_EScenarioFrameworkMarkerCustom.OBSERVATION_POST);
		Register("observe", SCR_EScenarioFrameworkMarkerCustom.OBSERVATION_POST);
		Register("overwatch", SCR_EScenarioFrameworkMarkerCustom.OBSERVATION_POST);
		Register("recon", SCR_EScenarioFrameworkMarkerCustom.OBSERVATION_POST);

		Register("rally", SCR_EScenarioFrameworkMarkerCustom.FLAG);
		Register("rally_point", SCR_EScenarioFrameworkMarkerCustom.FLAG);
		Register("base", SCR_EScenarioFrameworkMarkerCustom.FLAG);
		Register("hq", SCR_EScenarioFrameworkMarkerCustom.FLAG);
		Register("spawn", SCR_EScenarioFrameworkMarkerCustom.FLAG);

		Register("medical", SCR_EScenarioFrameworkMarkerCustom.CROSS);
		Register("medic", SCR_EScenarioFrameworkMarkerCustom.CROSS);
		Register("aid", SCR_EScenarioFrameworkMarkerCustom.CROSS);
		Register("casevac", SCR_EScenarioFrameworkMarkerCustom.CROSS);
		Register("medevac", SCR_EScenarioFrameworkMarkerCustom.CROSS);

		Register("area", SCR_EScenarioFrameworkMarkerCustom.CIRCLE);
		Register("zone", SCR_EScenarioFrameworkMarkerCustom.CIRCLE);
		Register("ao", SCR_EScenarioFrameworkMarkerCustom.CIRCLE);

		Register("point", SCR_EScenarioFrameworkMarkerCustom.DOT);
		Register("mark", SCR_EScenarioFrameworkMarkerCustom.DOT);
		Register("marker", SCR_EScenarioFrameworkMarkerCustom.DOT);
	}

	//! Register an alias under its normalised form, so an entry never disagrees with a lookup.
	protected static void Register(string alias, int entry)
	{
		s_mAliases.Set(Normalise(alias), entry);
	}

	//! Allocate the report latch on first use.
	protected static void EnsureReported()
	{
		if (!s_mReported)
			s_mReported = new map<string, bool>();
	}
}
