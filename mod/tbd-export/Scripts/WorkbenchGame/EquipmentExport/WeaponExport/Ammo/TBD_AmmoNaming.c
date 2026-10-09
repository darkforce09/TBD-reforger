//------------------------------------------------------------------------------------------------
// TBD_AmmoNaming.c
//
// Reads the player-facing strings a magazine or projectile declares - display name, description,
// inventory icon - and derives the catalog's family grouping from the prefab path.
//
// This domain reads UIInfo alongside the ItemDisplayName node, so it does not use the shared
// reader in Core. A localization token is cleaned to a readable stem only when nothing in the
// ancestry declares a real name.
//------------------------------------------------------------------------------------------------

class TBD_AmmoNaming
{
	//------------------------------------------------------------------------------------------------
	//! Extract human-readable display name.
	static string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("InventoryMagazineComponent"))
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

		array<BaseContainer> magComps = comps.Get("MagazineComponent");
		if (magComps)
		{
			foreach (BaseContainer mc : magComps)
			{
				BaseContainer ui = mc.GetObject("UIInfo");
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
	//! Extract localized description.
	static string DescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("InventoryMagazineComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string d;
				if (disp.Get("Description", d) && !d.IsEmpty())
					return CleanLocalizationToken(d);
			}
		}

		array<BaseContainer> magComps = comps.Get("MagazineComponent");
		if (magComps)
		{
			foreach (BaseContainer mc : magComps)
			{
				BaseContainer ui = mc.GetObject("UIInfo");
				if (!ui)
					continue;
				string d2;
				if (ui.Get("Description", d2) && !d2.IsEmpty())
					return CleanLocalizationToken(d2);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract icon texture resource path.
	static string IconFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("InventoryMagazineComponent"))
				continue;
			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string icon;
				if (disp.Get("Icon", icon) && !icon.IsEmpty())
					return TBD_EquipmentResourceNames.ResolveCanonicalResourceName(icon);
			}
		}

		array<BaseContainer> magComps = comps.Get("MagazineComponent");
		if (magComps)
		{
			foreach (BaseContainer mc : magComps)
			{
				BaseContainer ui = mc.GetObject("UIInfo");
				if (!ui)
					continue;
				string icon2;
				if (ui.Get("Icon", icon2) && !icon2.IsEmpty())
					return TBD_EquipmentResourceNames.ResolveCanonicalResourceName(icon2);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract magazine family (STANAG, AK74, AK47, M60, PKM, etc.).
	static string ExtractFamily(string filePath, string category)
	{
		string p = filePath;
		p.ToLower();

		if (p.Contains("stanag")) return "STANAG";
		if (p.Contains("545") || p.Contains("ak74")) return "AK-74";
		if (p.Contains("ak47") || p.Contains("akm") || p.Contains("762x39")) return "AK-47";
		if (p.Contains("m14") || p.Contains("m21")) return "M14";
		if (p.Contains("svd")) return "SVD";
		if (p.Contains("vz58") || p.Contains("sa58")) return "VZ-58";
		if (p.Contains("mosin")) return "Mosin";
		if (p.Contains("m249")) return "M249";
		if (p.Contains("m60")) return "M60";
		if (p.Contains("pkm")) return "PKM";
		if (p.Contains("uk59")) return "UK-59";
		if (p.Contains("m9") || p.Contains("beretta")) return "M9";
		if (p.Contains("makarov") || p.Contains("pm")) return "Makarov";
		if (p.Contains("1911") || p.Contains("colt")) return "1911";
		if (p.Contains("m203") || p.Contains("40x46")) return "M203";
		if (p.Contains("gp25") || p.Contains("vog")) return "GP-25";
		if (p.Contains("rpg7") || p.Contains("pg7")) return "RPG-7";
		if (p.Contains("m72")) return "M72 LAW";
		if (p.Contains("m242")) return "M242";
		if (p.Contains("2a42")) return "2A42";
		if (p.Contains("m2hb")) return "M2HB";
		if (p.Contains("nsv")) return "NSV";

		string stem = HumanizeStem(filePath);
		int sp = stem.IndexOf(" ");
		if (sp > 0)
			return stem.Substring(0, sp);
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	static string CleanLocalizationToken(string token)
	{
		if (!token.StartsWith("#") && !token.StartsWith("AR-"))
			return token;

		string s = token;
		if (s.StartsWith("#"))
			s = s.Substring(1, s.Length() - 1);
		if (s.StartsWith("AR-"))
			s = s.Substring(3, s.Length() - 3);

		if (s.StartsWith("Magazine_"))
			s = s.Substring(9, s.Length() - 9);
		else if (s.StartsWith("Item_"))
			s = s.Substring(5, s.Length() - 5);
		else if (s.StartsWith("AmmoType_"))
			s = s.Substring(9, s.Length() - 9);
		else if (s.StartsWith("AmmunitionID_"))
			s = s.Substring(13, s.Length() - 13);

		if (s.EndsWith("_Name"))
			s = s.Substring(0, s.Length() - 5);
		else if (s.EndsWith("_Description"))
			s = s.Substring(0, s.Length() - 12);

		s.Replace("_", " ");
		s.Trim();
		return s;
	}

	//------------------------------------------------------------------------------------------------
	static string HumanizeStem(string filePath)
	{
		string stem = filePath;
		int slash = stem.LastIndexOf("/");
		if (slash >= 0)
			stem = stem.Substring(slash + 1, stem.Length() - slash - 1);
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		if (stem.StartsWith("Magazine_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Box_"))
			stem = stem.Substring(4, stem.Length() - 4);
		else if (stem.StartsWith("Ammo_"))
			stem = stem.Substring(5, stem.Length() - 5);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}
}
