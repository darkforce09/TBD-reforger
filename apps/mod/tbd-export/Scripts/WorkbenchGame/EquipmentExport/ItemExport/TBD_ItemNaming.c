/**
 * TBD_ItemNaming.c
 *
 * Reads player-facing strings for inventory items: display name, description, and inventory icon.
 * Derives item family groupings and humanized name fallbacks from prefab paths when
 * localized strings are absent.
 */

class TBD_ItemNaming
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

		// Strip common Reforger inventory item prefixes
		if (stem.StartsWith("MedicalKit_")) stem = stem.Substring(11, stem.Length() - 11);
		else if (stem.StartsWith("Tourniquet_")) stem = stem.Substring(11, stem.Length() - 11);
		else if (stem.StartsWith("FieldDressing_")) stem = stem.Substring(14, stem.Length() - 14);
		else if (stem.StartsWith("SalineBag_")) stem = stem.Substring(10, stem.Length() - 10);
		else if (stem.StartsWith("MorphineInjection_")) stem = stem.Substring(18, stem.Length() - 18);
		else if (stem.StartsWith("Gauze_")) stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Radio_")) stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Compass_")) stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("Watch_")) stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("PaperMap_")) stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Map_")) stem = stem.Substring(4, stem.Length() - 4);
		else if (stem.StartsWith("Binoculars_")) stem = stem.Substring(11, stem.Length() - 11);
		else if (stem.StartsWith("Flashlight_")) stem = stem.Substring(11, stem.Length() - 11);
		else if (stem.StartsWith("ETool_")) stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("MineFlag_")) stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("BlastingMachine_")) stem = stem.Substring(16, stem.Length() - 16);
		else if (stem.StartsWith("DemoBlock_")) stem = stem.Substring(10, stem.Length() - 10);
		else if (stem.StartsWith("Mine_")) stem = stem.Substring(5, stem.Length() - 5);
		else if (stem.StartsWith("Grenade_")) stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("Smoke_")) stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("RepairKit_")) stem = stem.Substring(10, stem.Length() - 10);
		else if (stem.StartsWith("RearmingKit_")) stem = stem.Substring(12, stem.Length() - 12);
		else if (stem.StartsWith("BallisticTable_")) stem = stem.Substring(15, stem.Length() - 15);
		else if (stem.StartsWith("Canteen_")) stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("Jerrycan_")) stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("SupplyPortableContainers_")) stem = stem.Substring(25, stem.Length() - 25);
		else if (stem.StartsWith("SupplyCrate_")) stem = stem.Substring(12, stem.Length() - 12);
		else if (stem.StartsWith("Part_")) stem = stem.Substring(5, stem.Length() - 5);
		else if (stem.StartsWith("PersonalBelongings_")) stem = stem.Substring(19, stem.Length() - 19);
		else if (stem.StartsWith("IntelligenceFolder_")) stem = stem.Substring(19, stem.Length() - 19);

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
