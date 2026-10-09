//------------------------------------------------------------------------------------------------
// TBD_WeaponNaming.c
//
// Reads the player-facing strings a weapon declares - display name, description, inventory icon -
// and derives the catalog's family grouping from the prefab path.
//
// A weapon's strings sit on UIInfo rather than on the ItemDisplayName node the attachment domains
// read, which is why this domain does not use the shared reader in Core. Localization tokens are
// cleaned to a readable stem only when nothing declares a real name.
//------------------------------------------------------------------------------------------------

class TBD_WeaponNaming
{
	//------------------------------------------------------------------------------------------------
	//! Extract human-readable display name.
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
	//! Extract localized description string.
	static string DescriptionFor(map<string, ref array<BaseContainer>> comps)
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
				string d;
				if (disp.Get("Description", d) && !d.IsEmpty())
					return CleanLocalizationToken(d);
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
				string icon;
				if (disp.Get("Icon", icon) && !icon.IsEmpty())
					return TBD_EquipmentResourceNames.ResolveCanonicalResourceName(icon);
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
				string icon2;
				if (ui.Get("Icon", icon2) && !icon2.IsEmpty())
					return TBD_EquipmentResourceNames.ResolveCanonicalResourceName(icon2);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Clean localization tokens into human-readable strings.
	static string CleanLocalizationToken(string token)
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

		if (stem.StartsWith("Rifle_"))
			stem = stem.Substring(6, stem.Length() - 6);
		else if (stem.StartsWith("Weapon_"))
			stem = stem.Substring(7, stem.Length() - 7);
		else if (stem.StartsWith("Launcher_"))
			stem = stem.Substring(9, stem.Length() - 9);
		else if (stem.StartsWith("Grenade_"))
			stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("Handgun_"))
			stem = stem.Substring(8, stem.Length() - 8);
		else if (stem.StartsWith("UGL_"))
			stem = stem.Substring(4, stem.Length() - 4);
		else if (stem.StartsWith("Mine_"))
			stem = stem.Substring(5, stem.Length() - 5);

		stem.Replace("_", " ");
		stem.Trim();
		return stem;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract weapon family from folder hierarchy.
	static string ExtractFamily(string filePath, string category)
	{
		string marker = "Weapons/" + category + "/";
		if (category == "rifles") marker = "Weapons/Rifles/";
		else if (category == "machine_guns") marker = "Weapons/MachineGuns/";
		else if (category == "handguns") marker = "Weapons/Handguns/";
		else if (category == "launchers") marker = "Weapons/Launchers/";
		else if (category == "grenades") marker = "Weapons/Grenades/";
		else if (category == "explosives") marker = "Weapons/Explosives/";
		else if (category == "underbarrel") marker = "Attachments/Underbarrel/";

		int idx = filePath.IndexOf(marker);
		if (idx >= 0)
		{
			string sub = filePath.Substring(idx + marker.Length(), filePath.Length() - idx - marker.Length());
			int slash = sub.IndexOf("/");
			if (slash > 0)
				return sub.Substring(0, slash);
			if (sub.EndsWith(".et"))
			{
				sub = sub.Substring(0, sub.Length() - 3);
				return sub;
			}
			return sub;
		}

		// Fallback to stem prefix
		string stem = HumanizeStem(filePath);
		int sp = stem.IndexOf(" ");
		if (sp > 0)
			return stem.Substring(0, sp);
		return stem;
	}
}
