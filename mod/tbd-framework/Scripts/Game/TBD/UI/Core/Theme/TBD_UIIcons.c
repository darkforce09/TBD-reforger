/**
 * @file TBD_UIIcons.c
 * @brief The one place an icon key becomes an image: the addon's own raster, else a vanilla quad.
 *
 * Role: loads a mockup icon key (`search`, `grid_view`, `water`, ...) into an `ImageWidget`, from
 * `UI/Textures/TBD/Icons/TBD_Icon_<key>_UI.edds` for the 38 shipped keys, else from a quad of the
 * vanilla `icons_wrapperUI-64.imageset`, else hides the slot.
 * Position: called by every component and panel that shows an icon; warns through `TBD_WarnOnce`.
 * State: the shipped key set, the pinned GUID map (empty), the texture misses and the quad table,
 * built on first use, for the life of the script VM on the client.
 * Invariants: a caller never special-cases a missing icon; `Load` returns true only when an image
 * shows; the quad table holds only quads that resolved on a real run, so every other key hides
 * without an engine log line.
 */

//! Icon key lookup. Each shipped raster is a 64 px white-on-alpha render of the Material Symbols
//! outlined glyph the mockups use; `s_mTextureGuids` pins no GUID, so `Load()` addresses each
//! `.edds` by bare path. Enfusion has no icon font, so the fallback is
//! `ImageWidget.LoadImageFromSet(0, imageset, quad)`. The `.imageset` is pak-only in a format
//! nothing offline opens, so quads are learned by trying them. Resolved: `search`, `player`,
//! `check`, `scenarios` (and `cancel`, `settings`, `general` from vanilla scripts). Rejected:
//! `group`, `lobby`, `briefing`, `map`, `addons`, `defend`, `flag`, `arrow_right`.
class TBD_UIIcons
{
	static const ResourceName IMAGESET = "{2EFEA2AF1F38E7F0}UI/Textures/Icons/icons_wrapperUI-64.imageset"; //!< the vanilla wrapper-UI imageset the quads come from
	static const string ICON_DIR = "UI/Textures/TBD/Icons/"; //!< folder of the addon's icon rasters; key -> `TBD_Icon_<key>_UI.edds`

	protected static ref map<string, ResourceName> s_mTextureGuids; //!< key -> "{GUID}UI/Textures/TBD/Icons/...edds"; empty, so bare paths are used
	protected static ref set<string> s_sShipped; //!< the keys with a shipped raster
	protected static ref set<string> s_sTextureMisses; //!< keys whose raster failed to load; never retried

	protected static ref map<string, string> m_mQuads; //!< key -> quad; built on first Quad()

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

	//! Load `key` into `w`. Hides the widget when the key has no quad or the quad does not resolve,
	//! so a caller never has to special-case a missing icon. Returns true when an image is showing.
	static bool Load(ImageWidget w, string key)
	{
		if (!w)
			return false;

		ResourceName texture = Texture(key);
		if (!texture.IsEmpty() && !TextureMissed(key))
		{
			if (w.LoadImageTexture(0, texture))
			{
				w.SetImage(0);
				w.SetVisible(true);
				return true;
			}

			// Not imported yet: remember the miss so the engine logs "Wrong GUID" once, not per widget.
			s_sTextureMisses.Insert(key);
		}

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

	//! @return true when `key`'s raster already failed to load
	protected static bool TextureMissed(string key)
	{
		if (!s_sTextureMisses)
			s_sTextureMisses = new set<string>();

		return s_sTextureMisses.Contains(key);
	}

	//! Our own raster for `key`, or empty when none was shipped for it.
	static ResourceName Texture(string key)
	{
		if (!s_sShipped)
			BuildShipped();

		if (!s_sShipped.Contains(key))
			return string.Empty;

		ResourceName pinned;
		if (s_mTextureGuids && s_mTextureGuids.Find(key, pinned))
			return pinned;

		return ICON_DIR + "TBD_Icon_" + key + "_UI.edds";
	}

	//! The 38 PNGs under UI/Textures/TBD/Icons/ (keep in step with the folder).
	protected static void BuildShipped()
	{
		s_sShipped = new set<string>();
		array<string> keys = {
			"map", "description", "groups", "edit_location_alt", "radio", "directions_car", "checkroom",
			"adjust", "warning", "tune", "cell_tower", "timer", "target", "visibility", "shield", "schedule",
			"hourglass_bottom", "thermostat", "my_location", "download", "assignment", "expand_more",
			"chevron_right", "person", "search", "check", "grid_view", "water", "landscape", "ac_unit",
			"extension", "flag", "notes", "meeting_room", "close", "lock", "play_arrow", "settings"
		};
		foreach (string key : keys)
		{
			s_sShipped.Insert(key);
		}

		s_mTextureGuids = new map<string, ResourceName>();
	}

	//! Warn once per key that its quad did not resolve.
	protected static void WarnOnce(string key, string quad)
	{
		TBD_WarnOnce.Warn("ui", key, string.Format("icon '%1' -> quad '%2' did not resolve in %3; slot hidden. Fix TBD_UIIcons.", key, quad, IMAGESET));
	}

	//! Fill the quad table with the quads that resolved on a real run.
	protected static void BuildTable()
	{
		m_mQuads = new map<string, string>();

		// Vanilla scripts (SCR_MapMarkersUI / SCR_MapMarkerEntryPlaced attribute defaults).
		m_mQuads.Insert("cancel",        "cancel");
		m_mQuads.Insert("settings",      "settings");
		m_mQuads.Insert("scenarios",     "scenarios");
		m_mQuads.Insert("general",       "general");

		// Resolved on a real run.
		m_mQuads.Insert("search",        "search");
		m_mQuads.Insert("person",        "player");
		m_mQuads.Insert("check",         "check");
		m_mQuads.Insert("check_circle",  "check");
		m_mQuads.Insert("grid_view",     "scenarios");

		// Every other key: no quad yet -> Quad() returns empty -> slot hidden, no log line.
	}
}
