//------------------------------------------------------------------------------------------------
// TBD_M16Naming.c
//
// Reads the display name an M16 variant, magazine, or attachment declares, falling back to a
// readable stem derived from the prefab filename.
//
// The stem fallback carries more cases than the other domains need, because this scan names every
// candidate magazine and attachment in the pool, not only the weapons.
//------------------------------------------------------------------------------------------------

class TBD_M16Naming
{
	//------------------------------------------------------------------------------------------------
	static string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string n;
				if (disp.Get("Name", n) && !n.IsEmpty() && !n.StartsWith("#") && !n.StartsWith("AR-"))
					return n;
			}
		}

		array<string> uiHolders = {"WeaponComponent", "MagazineComponent"};
		foreach (string holder : uiHolders)
		{
			array<BaseContainer> bucket2 = comps.Get(holder);
			if (!bucket2)
				continue;
			foreach (BaseContainer c : bucket2)
			{
				BaseContainer ui = c.GetObject("UIInfo");
				if (!ui)
					continue;
				string n2;
				if (ui.Get("Name", n2) && !n2.IsEmpty() && !n2.StartsWith("#") && !n2.StartsWith("AR-"))
					return n2;
			}
		}

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	protected static string HumanizeStem(string filePath)
	{
		string stem = filePath;
		int slash = stem.LastIndexOf("/");
		if (slash >= 0)
			stem = stem.Substring(slash + 1, stem.Length() - slash - 1);
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		// Optics specific names
		if (stem == "Optic_4x20" || stem == "Optic_4x20_base")
			return "Colt 4x20 Scope";
		if (stem.StartsWith("Optic_4x20_"))
			return "Colt 4x20 Scope (" + HumanizeStem(stem.Substring(11, stem.Length() - 11)) + ")";
		if (stem == "Collim_AP2k" || stem == "Collim_AP2k_base")
			return "Aimpoint 2000 (AP2k)";
		if (stem.StartsWith("Collim_AP2k_"))
			return "Aimpoint 2000 (" + HumanizeStem(stem.Substring(12, stem.Length() - 12)) + ")";

		// Underbarrel
		if (stem == "UGL_M203_long" || stem == "UGL_M203_base")
			return "M203 40mm Grenade Launcher";
		if (stem.StartsWith("UGL_M203_"))
			return "M203 40mm (" + HumanizeStem(stem.Substring(9, stem.Length() - 9)) + ")";

		// Magazines
		if (stem.StartsWith("Magazine_556x45_STANAG_30rnd_"))
		{
			string variant = stem.Substring(30, stem.Length() - 30);
			variant.Replace("_", " ");
			return "5.56x45mm STANAG 30-round (" + variant + ")";
		}

		if (stem.StartsWith("Rifle_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Magazine_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Optic_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Collim_"))
			stem = stem.Substring(7, stem.Length() - 7);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}
}
