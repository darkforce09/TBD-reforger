/**
 * TBD_WearableNaming.c
 *
 * Reads player-facing strings for wearable items: display name, description, and inventory icon.
 * Derives equipment family groupings and humanized name fallbacks from prefab paths when
 * localized strings are absent.
 */

class TBD_WearableNaming
{
	//------------------------------------------------------------------------------------------------
	//! Extract display name from ItemDisplayName across ancestry, falling back to humanized stem.
	static string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		string raw = TBD_EquipmentDisplayAttributes.RawDisplayNameFor(comps);
		if (!raw.IsEmpty())
			return raw;

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract description string across ancestry.
	static string DescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		return TBD_EquipmentDisplayAttributes.RawDescriptionFor(comps);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract inventory icon path with normalized forward slashes.
	static string IconFor(map<string, ref array<BaseContainer>> comps)
	{
		return TBD_EquipmentDisplayAttributes.RawIconFor(comps);
	}

	//------------------------------------------------------------------------------------------------
	//! Derive human-readable name from filename stem.
	static string HumanizeStem(string filePath)
	{
		string stem = filePath;
		int slash = stem.LastIndexOf("/");
		if (slash >= 0)
			stem = stem.Substring(slash + 1, stem.Length() - slash - 1);
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		// Strip common Reforger equipment prefixes
		if (stem.StartsWith("Vest_"))
			stem = stem.Substring(5, stem.Length() - 5);
		else if (stem.StartsWith("Helmet_"))
			stem = stem.Substring(7, stem.Length() - 7);
		else if (stem.StartsWith("Hat_"))
			stem = stem.Substring(4, stem.Length() - 4);
		else if (stem.StartsWith("Jacket_"))
			stem = stem.Substring(7, stem.Length() - 7);
		else if (stem.StartsWith("Pants_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Gloves_"))
			stem = stem.Substring(7, stem.Length() - 7);
		else if (stem.StartsWith("CombatBoots_"))
			stem = stem.Substring(12, stem.Length() - 12);
		else if (stem.StartsWith("TankerBoots_"))
			stem = stem.Substring(12, stem.Length() - 12);
		else if (stem.StartsWith("Goggles_"))
			stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("Backpack_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Pouch_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Holster_"))
			stem = stem.Substring(8, stem.Length() - 8);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract family name from prefab directory hierarchy.
	static string ExtractFamily(string filePath, string category)
	{
		string path = filePath;
		path.Replace("\\", "/");

		int catIdx = path.LastIndexOf("/" + category + "/");
		if (catIdx >= 0)
		{
			string sub = path.Substring(catIdx + category.Length() + 2, path.Length() - (catIdx + category.Length() + 2));
			int nextSlash = sub.IndexOf("/");
			if (nextSlash > 0)
				return sub.Substring(0, nextSlash);
		}

		// Fallback: use enclosing folder name
		int lastSlash = path.LastIndexOf("/");
		if (lastSlash > 0)
		{
			string parent = path.Substring(0, lastSlash);
			int prevSlash = parent.LastIndexOf("/");
			if (prevSlash >= 0)
				return parent.Substring(prevSlash + 1, parent.Length() - prevSlash - 1);
		}

		return category;
	}
}
