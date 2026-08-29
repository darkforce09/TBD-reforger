//! T-181.19 - turning the mission JSON's authored `icon` string into something the engine will
//! actually draw. T-276 closed the schema side: `#/$defs/marker.icon` is now an enum of the
//! 64 Register() alias keys below (authored contract). Runtime still bridges to the engine.
//!
//! -- The problem -----------------------------------------------------------------------------
//! Reforger's placed-marker system does not take a string: `SCR_MapMarkerBase.SetIconEntry(int)`
//! takes an INDEX into the icon array authored in the vanilla `Configs/Map/MapMarkerConfig.conf`.
//! Something has to bridge the two. Authored missions that pass the schema validator already
//! carry a known alias; FALLBACK still exists for empty/unknown strings that reach the loader
//! without schema validation (hand-edits, engine quad names that are not in the authored enum,
//! or a future game update that retires a name).
//!
//! -- Where the index vocabulary comes from (measured, not remembered) ------------------------
//! Vanilla ships a named enum for exactly these indices and uses it itself -
//! `SCR_BaseTutorialStage.CreateMarkerCustom()` does
//! `marker.SetType(PLACED_CUSTOM); marker.SetIconEntry(<SCR_EScenarioFrameworkMarkerCustom>)`,
//! and CRF's `CRF_RaidItemComponent.c:64` does the same thing with `.DESTROY2`. So the enum values
//! ARE the config indices, by vanilla's own construction rather than by our inference.
//!
//! That enum is NOT in any oracle: it is not in the vanilla symbol index, it is not in the cached
//! Doxygen (the identifier is not even hyperlinked there, meaning no indexed file defines it), and
//! `Configs/Map/MapMarkerConfig.conf` is not in the pak file table this repo can read. Its
//! vocabulary was therefore DISCOVERED BY COMPILATION: a probe naming one candidate member per
//! line, compiled once, where a surviving line names a real member and a failing one does not.
//! Two deliberate sentinels (`ZZ_DEFINITELY_NOT_A_MEMBER`, `ZZ_ROUND3_SENTINEL`) failed in the
//! same runs, which is what makes the survivors evidence rather than hope. Every one of the 23
//! members below is compile-verified against the retail runtime; nothing here is guessed.
//!
//! -- The wider surface: the icons the RUNNING game loaded ------------------------------------
//! The enum is not the whole picture. `SCR_MapMarkerEntryPlaced.GetIconEntries()` exposes the live
//! icon list with each entry's imageset-quad NAME, and a boot of the real scenario measured
//! **91** of them (`[TBD][Markers] marker-manager ok placedIcons=91`, world-boot 2026-07-25). So
//! `Resolve()` tries the engine's own names FIRST and the alias table second: that makes all 91
//! authorable without this file knowing any of them, and a game update that adds icons needs no
//! code change. The alias table stays because the website emits friendly words, not quad names.
//!
//! -- What is still NOT proven on this lane ---------------------------------------------------
//! Which picture each name draws. No tool in this program returns a framebuffer, so the mapping
//! from `"objective"` to a glyph an operator would call an objective is an educated reading of the
//! member NAME. If the operator says an icon looks wrong, the fix is one line in `Register()` -
//! not a redesign. The set of 91 quad names is likewise not enumerable offline, which is exactly
//! why `DumpVocabularyOnce()` publishes it from the running game.
class TBD_MarkerIcons
{
	//! Fallback for an icon string we cannot place. DOT is the least presumptuous glyph in the
	//! confirmed set: a marker whose icon we did not understand should still show WHERE it is.
	//! Never a silent drop, and never a hard failure - an unreadable icon must not cost the
	//! mission a marker.
	static const int FALLBACK_ICON = SCR_EScenarioFrameworkMarkerCustom.DOT;

	//! One colour for every mission marker. Per-marker colour is not authorable today (the schema
	//! has no colour field), and inventing a colour policy the operator did not ask for would be a
	//! silent product decision. REFORGER_ORANGE is what every vanilla example uses.
	static const int MARKER_COLOR = SCR_EScenarioFrameworkMarkerCustomColor.REFORGER_ORANGE;

	//! alias (already normalised) -> icon entry index.
	protected static ref map<string, int> s_mAliases;

	//! Normalised imageset-quad name -> icon entry index, read from the config the RUNNING game
	//! loaded. Null until first use; empty when the config could not be read (a headless machine
	//! never gets here, because it never draws a marker).
	protected static ref map<string, int> s_mConfigQuads;

	//! How many placed-marker icons the live config carries; -1 = not read yet.
	//! MEASURED at boot on the retail runtime: 91.
	protected static int s_iConfigIconCount = -1;

	//! Unknown icon strings already reported, so a 40-marker mission with one bad icon logs ONE
	//! line rather than forty. Keyed on the normalised form.
	protected static ref map<string, bool> s_mReported;

	//! The full vocabulary is dumped at most once per world, however many distinct typos there are.
	protected static bool s_bVocabularyDumped;

	//------------------------------------------------------------------------------------------------
	//! Lowercase, trim, and fold the separators an author might type. Returns the normalised key.
	//!
	//! RECORDED LANDMINE: `ToLower()` and `Replace()` MUTATE IN PLACE and return a COUNT - writing
	//! `s = s.ToLower()` does not compile. Proven both ways: `int n = s.ToLower();` compiles and
	//! `string x = s.ToLower();` is a hard compile error (negative control NC5).
	static string Normalise(string raw)
	{
		string key = raw;
		key.TrimInPlace();
		key.ToLower();
		key.Replace("-", "_");
		key.Replace(" ", "_");

		return key;
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve an authored icon string to a marker icon entry index.
	//!
	//! Two sources, tried in this order:
	//!   1. **The live config.** Every placed-marker icon the running game loaded, keyed by its own
	//!      imageset-quad name. MEASURED at boot: 91 of them. This is the widest possible surface
	//!      and it costs no maintenance - a game update that adds icons makes them authorable the
	//!      same day, with no code change here.
	//!   2. **The alias table.** The compile-verified enum members plus the friendly words a
	//!      mission author is likely to type ("objective", "medevac", "rally"), which are what the
	//!      website actually emits today and which no quad name is guaranteed to match.
	//!
	//! Config first, because a name the ENGINE recognises should never be overridden by a name we
	//! invented; the alias table is the fallback vocabulary, not the authority.
	//!
	//! Never fails: an unrecognised or empty string yields FALLBACK_ICON and `recognised` false, so
	//! the caller can log once and still draw the marker. Schema-validated missions (T-276) cannot
	//! author an empty or unknown alias - the enum is the authored contract - but Resolve still
	//! defends the runtime path for hand-edits and live engine quad names outside that enum.
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

	//------------------------------------------------------------------------------------------------
	//! Report an icon string we could not place - ONCE per distinct string.
	//!
	//! The line names the offending value AND prints the accepted vocabulary, because the person
	//! who has to fix this is authoring on the website and has no other way to learn what the game
	//! accepts. WARNING, not ERROR: the marker still drew, the round is fine, and `world-boot.sh`
	//! fails closed on any `SCRIPT (E)` line the mod owns.
	static void ReportUnknown(string authoredIcon)
	{
		EnsureReported();

		string key = Normalise(authoredIcon);

		bool seen;
		if (s_mReported.Find(key, seen))
			return;

		s_mReported.Set(key, true);

		// An EMPTY icon is not a typo, it is a document that bypassed schema validation (T-276
		// forbids `""` in the authored enum). Treat it as information, not as a mistake, and do
		// not bury the log in a 91-name dump for it.
		if (key.IsEmpty())
		{
			TBD_Log.Event(TBD_MarkerService.CH_MARKERS,
				"a marker was authored with no icon - drew the default dot.");
			return;
		}

		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS, string.Format(
			"icon '%1' is not a known marker icon - drew the fallback dot instead. The marker itself is still on the map.",
			authoredIcon));

		DumpVocabularyOnce();
	}

	//------------------------------------------------------------------------------------------------
	//! New mission, new set of complaints. Without this, an author who fixes a typo and reloads
	//! sees no confirmation because the old key is still latched.
	static void ResetReported()
	{
		s_mReported = null;
		s_bVocabularyDumped = false;
	}

	//------------------------------------------------------------------------------------------------
	//! Drop everything, including the cached read of the game's marker config.
	//!
	//! Statics outlive a world inside one process (recorded landmine), and the config cache holds
	//! indices into an array owned by a component that dies with the world. Keeping it across an
	//! in-process scenario restart would silently pin the previous world's icon numbering.
	static void ResetForWorld()
	{
		ResetReported();
		s_mConfigQuads = null;
		s_iConfigIconCount = -1;
	}

	//------------------------------------------------------------------------------------------------
	//! Print everything `Resolve()` accepts - once per world, however many bad icons there are.
	//!
	//! This is the whole answer to "what am I allowed to type?", and it is emitted at runtime for a
	//! reason: the engine's 91 icon names live in packed vanilla data that nothing in this repo can
	//! read offline, so the only honest place to publish them is the machine that loaded them.
	//! Split into two lines because they are two different vocabularies with two different
	//! stabilities - the engine's names can change with a game update, ours cannot.
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

	//------------------------------------------------------------------------------------------------
	//! Comma-separated keys of a lookup table, for the vocabulary dump.
	protected static string JoinKeys(map<string, int> table)
	{
		// `out` is a reserved word in Enforce Script (out parameters) - naming a local that way
		// fails with a bare `Broken expression (missing ';'?)` that never mentions the keyword.
		string list;
		foreach (string key, int entry : table)
		{
			if (!list.IsEmpty())
			{
				// Appended in steps on purpose. A long `+` chain hits `Formula too complex` at
				// around nine terms in this compiler, and its SECOND diagnostic is a misleading
				// `Incompatible parameter` that sends you hunting a type error that is not there.
				list = list + ", ";
			}

			list = list + key;
		}

		return list;
	}

	//------------------------------------------------------------------------------------------------
	//! Cross-check a resolved index against the icon array the RUNNING game actually loaded.
	//!
	//! The enum-is-the-index contract is vanilla's own, but the config is data we do not ship and
	//! cannot read offline. If a future game build ever shortens that array, `GetIconEntry()` would
	//! quietly set no image at all and every marker would render blank with no diagnostic. This
	//! turns that into one warning and a fallback. Returns the index to actually use.
	static int ClampToLoadedConfig(int entry)
	{
		EnsureConfigQuads();

		// Config unreadable: nothing to check against, so trust the caller rather than second-guess
		// it. A wrong index is a missing picture; refusing to draw would be a missing marker.
		if (s_iConfigIconCount <= 0)
			return entry;

		if (entry >= 0 && entry < s_iConfigIconCount)
			return entry;

		TBD_Log.Warn(TBD_MarkerService.CH_MARKERS,
			string.Format("icon entry %1 is outside the %2 icons this game build loaded - using 0.",
				entry, s_iConfigIconCount));

		return 0;
	}

	//------------------------------------------------------------------------------------------------
	//! Read the live placed-marker icon list once and index it by imageset-quad name.
	//!
	//! Best effort by design. On any machine where the marker system is not reachable this leaves
	//! an empty table and `s_iConfigIconCount` at 0, and everything downstream keeps working off
	//! the alias table alone - a headless server never reaches here at all, because it never draws.
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

			// First wins. Two entries can legitimately share a quad across categories, and the
			// lower index is the one vanilla's own selection menu shows first.
			if (s_mConfigQuads.Contains(key))
				continue;

			s_mConfigQuads.Set(key, i);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! The alias table. Left column = what a mission author may plausibly type on the website;
	//! right column = a COMPILE-VERIFIED member of `SCR_EScenarioFrameworkMarkerCustom`.
	//!
	//! Every confirmed member is registered under its own name too, so an author who types the
	//! engine's own vocabulary always wins regardless of what the friendly aliases do.
	protected static void EnsureAliases()
	{
		if (s_mAliases)
			return;

		s_mAliases = new map<string, int>();

		// -- the 23 compile-verified enum members, each under its own name -----------------------
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

		// -- mission-authoring vocabulary, folded onto the same members --------------------------
		// The website has no icon picker yet (T-069 is the slice that would add one), so these are
		// the words a mission maker is likely to reach for. Adding one is a one-line change and
		// costs nothing if it is never used.
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

	//------------------------------------------------------------------------------------------------
	//! Aliases are registered pre-normalised at the call sites above, but running them through
	//! `Normalise()` anyway means a table entry can never disagree with a lookup.
	protected static void Register(string alias, int entry)
	{
		s_mAliases.Set(Normalise(alias), entry);
	}

	//------------------------------------------------------------------------------------------------
	protected static void EnsureReported()
	{
		if (!s_mReported)
			s_mReported = new map<string, bool>();
	}
}
