//------------------------------------------------------------------------------------------------
// TBD_RifleNaming.c
//
// Reads the display name a rifle declares and derives its family from the prefab path.
//
// A name is taken from InventoryItemComponent first and the weapon's own UIInfo second. Only when
// neither declares one is a readable stem derived from the prefab filename. A localization token
// is stripped to its meaningful part rather than exported raw, because this catalog is read by the
// platform rather than by the engine.
//------------------------------------------------------------------------------------------------

class TBD_RifleNaming
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
				if (disp.Get("Name", n) && !n.IsEmpty())
				{
					string cleaned = CleanLocalizationToken(n);
					if (!cleaned.IsEmpty())
						return cleaned;
				}
			}
		}

		array<BaseContainer> weaponComps = comps.Get("WeaponComponent");
		if (weaponComps)
		{
			foreach (BaseContainer wc : weaponComps)
			{
				BaseContainer ui = wc.GetObject("UIInfo");
				if (!ui)
					continue;
				string n2;
				if (ui.Get("Name", n2) && !n2.IsEmpty())
				{
					string cleaned2 = CleanLocalizationToken(n2);
					if (!cleaned2.IsEmpty())
						return cleaned2;
				}
			}
		}

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	protected static string CleanLocalizationToken(string token)
	{
		if (!token.StartsWith("#") && !token.StartsWith("AR-"))
			return token;

		string s = token;
		if (s.StartsWith("#"))
			s = s.Substring(1, s.Length() - 1);
		if (s.StartsWith("AR-"))
			s = s.Substring(3, s.Length() - 3);

		if (s.StartsWith("Weapon_"))
			s = s.Substring(7, s.Length() - 7);
		else if (s.StartsWith("Item_"))
			s = s.Substring(5, s.Length() - 5);
		else if (s.StartsWith("Magazine_"))
			s = s.Substring(9, s.Length() - 9);

		if (s.EndsWith("_Name"))
			s = s.Substring(0, s.Length() - 5);

		s.Replace("_", " ");
		s.Trim();
		return s;
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

		if (stem.StartsWith("Rifle_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Weapon_"))
			stem = stem.Substring(7, stem.Length() - 7);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	static string ExtractFamily(string filePath)
	{
		int riflesIdx = filePath.IndexOf("Weapons/Rifles/");
		if (riflesIdx >= 0)
		{
			string sub = filePath.Substring(riflesIdx + 15, filePath.Length() - riflesIdx - 15);
			int slash = sub.IndexOf("/");
			if (slash > 0)
				return sub.Substring(0, slash);
			return sub;
		}
		return "Rifle";
	}
}
