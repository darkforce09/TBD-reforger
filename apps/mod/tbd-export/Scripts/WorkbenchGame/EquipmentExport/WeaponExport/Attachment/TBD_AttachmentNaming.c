//------------------------------------------------------------------------------------------------
// TBD_AttachmentNaming.c
//
// Reads the player-facing strings an attachment declares - display name, description, inventory
// icon - and derives the catalog's family grouping from the prefab path.
//
// This sweep covers every non-optic attachment in every addon, so it falls back to a readable stem
// derived from the prefab filename far more often than a single-family domain does. That wider
// fallback is why it does not use the shared reader in Core.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentNaming
{
	//------------------------------------------------------------------------------------------------
	//! Extract localized display name.
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

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract localized description.
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

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract inventory icon texture path.
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
					return TBD_EquipmentResourceNames.NormalizePathSeparators(icon);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Derive canonical family identifier from file path.
	static string DeriveFamily(string filePath, string resName)
	{
		string p = filePath;
		if (p.IsEmpty())
			p = resName;

		p.Replace("\\", "/");
		array<string> parts = {};
		p.Split("/", parts, true);

		// Look for standard folder structure (e.g. Prefabs/Weapons/Attachments/Muzzle/Suppressor_M16/...)
		for (int i = 0; i < parts.Count(); i++)
		{
			string part = parts[i];
			if (part == "Muzzle" || part == "Bayonets" || part == "Stocks" || part == "Handguards" || part == "Underbarrel" || part == "Flashlights" || part == "Bipods" || part == "Mounts")
			{
				if (i + 1 < parts.Count())
				{
					string candidate = parts[i + 1];
					if (!candidate.EndsWith(".et"))
						return candidate;
				}
			}
		}

		// Fallback to filename stem
		string stem = parts[parts.Count() - 1];
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		return stem;
	}

	//------------------------------------------------------------------------------------------------
	//! Strip localization prefix '#AR-' or return original clean token.
	static string CleanLocalizationToken(string token)
	{
		if (token.IsEmpty())
			return string.Empty;

		if (token.StartsWith("#"))
			return token.Substring(1, token.Length() - 1);

		return token;
	}

	//------------------------------------------------------------------------------------------------
	//! Humanize a filename stem into a clean display title.
	static string HumanizeStem(string filePath)
	{
		string name = filePath;
		int slashIdx = name.LastIndexOf("/");
		if (slashIdx >= 0)
			name = name.Substring(slashIdx + 1, name.Length() - slashIdx - 1);

		if (name.EndsWith(".et"))
			name = name.Substring(0, name.Length() - 3);

		name.Replace("_", " ");
		return name;
	}
}
