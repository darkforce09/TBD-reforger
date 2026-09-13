//! Pre-game UI rebuild (2026-09-12) — the one place an icon key becomes an imageset quad.
//!
//! The Stitch mockups draw Material Symbols (`search`, `grid_view`, `water`, …). Enfusion has no
//! icon font, so every icon slot is an `ImageWidget` fed from the vanilla wrapper-UI imageset via
//! `ImageWidget.LoadImageFromSet(0, imageset, quad)`. Screens name icons by the mockup key and
//! this table decides the quad.
//!
//! ── HONEST STATUS (measured on the 2026-09-12 Workbench run) ────────────────────────────
//! The `.imageset` is pak-only under a codec nothing offline can open, so quads are learned by
//! trying them. `Load()` hides the slot and warns once when a quad does not resolve; the engine
//! adds its own `GUI (E): Can't find image` line. To keep the log clean the table below holds
//! ONLY quads that resolved on a real run; every other key maps to nothing and hides silently.
//! Resolved: `search`, `player`, `check`, `scenarios` (+ `cancel`, `settings`, `general` from
//! vanilla scripts). Tried and REJECTED: `group`, `lobby`, `briefing`, `map`, `addons`, `defend`,
//! `flag`, `arrow_right`. Still wanting a quad: group/groups, meeting_room, description, notes,
//! extension, shield, flag, chevron_right, expand_more, unfold_more, water, landscape, ac_unit.
class TBD_UIIcons
{
	static const ResourceName IMAGESET = "{2EFEA2AF1F38E7F0}UI/Textures/Icons/icons_wrapperUI-64.imageset";

	protected static ref map<string, string> m_mQuads;
	protected static ref set<string> m_sWarned;

	//------------------------------------------------------------------------------------------------
	//! Mockup icon key -> imageset quad. Empty string = draw nothing for this key.
	static string Quad(string key)
	{
		if (!m_mQuads)
			BuildTable();

		string quad;
		if (m_mQuads.Find(key, quad))
			return quad;

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Load `key` into `w`. Hides the widget when the key has no quad or the quad does not resolve,
	//! so a caller never has to special-case a missing icon. Returns true when an image is showing.
	static bool Load(ImageWidget w, string key)
	{
		if (!w)
			return false;

		string quad = Quad(key);
		if (quad.IsEmpty())
		{
			w.SetVisible(false);
			return false;
		}

		bool loaded = w.LoadImageFromSet(0, IMAGESET, quad);
		w.SetVisible(loaded);

		if (!loaded)
			WarnOnce(key, quad);

		return loaded;
	}

	//------------------------------------------------------------------------------------------------
	protected static void WarnOnce(string key, string quad)
	{
		if (!m_sWarned)
			m_sWarned = new set<string>();

		if (m_sWarned.Contains(key))
			return;

		m_sWarned.Insert(key);
		Print(string.Format("[TBD][ui] icon '%1' -> quad '%2' did not resolve in %3; slot hidden. Fix TBD_UIIcons.", key, quad, IMAGESET), LogLevel.WARNING);
	}

	//------------------------------------------------------------------------------------------------
	protected static void BuildTable()
	{
		m_mQuads = new map<string, string>();

		// Vanilla scripts (SCR_MapMarkersUI / SCR_MapMarkerEntryPlaced attribute defaults).
		m_mQuads.Insert("cancel",        "cancel");
		m_mQuads.Insert("settings",      "settings");
		m_mQuads.Insert("scenarios",     "scenarios");
		m_mQuads.Insert("general",       "general");

		// Resolved on the 2026-09-12 run.
		m_mQuads.Insert("search",        "search");
		m_mQuads.Insert("person",        "player");
		m_mQuads.Insert("check",         "check");
		m_mQuads.Insert("check_circle",  "check");
		m_mQuads.Insert("grid_view",     "scenarios");

		// Every other key: no quad yet -> Quad() returns empty -> slot hidden, no log line.
	}
}
