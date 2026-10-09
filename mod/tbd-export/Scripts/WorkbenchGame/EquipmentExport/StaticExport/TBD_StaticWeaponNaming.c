/**
 * TBD_StaticWeaponNaming.c
 *
 * Extracts display names, descriptions, families, and factions for static
 * and crew-served weapons (mortars, tripods, bare mounts).
 */

class TBD_StaticWeaponNaming
{
	//------------------------------------------------------------------------------------------------
	//! Extract human-readable display name for static weapon.
	static string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		// 1. Check SCR_EditableVehicleUIInfo / SCR_EditableEntityUIInfo
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("Editable"))
				continue;

			foreach (BaseContainer edComp : bucket)
			{
				BaseContainer info = edComp.GetObject("m_UIInfo");
				if (!info)
					info = edComp.GetObject("UIInfo");
				if (!info)
					continue;

				string name;
				if (info.Get("Name", name) && !name.IsEmpty())
				{
					string cleaned = CleanToken(name);
					if (!cleaned.IsEmpty())
						return cleaned;
				}
			}
		}

		// 2. Check InventoryItemComponent -> ItemDisplayName
		foreach (string icls, array<BaseContainer> ibucket : comps)
		{
			if (!icls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : ibucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;

				string iname;
				if (disp.Get("Name", iname) && !iname.IsEmpty())
				{
					string cname = CleanToken(iname);
					if (!cname.IsEmpty())
						return cname;
				}
			}
		}

		// 3. Fallback to path humanization
		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract description string.
	static string DescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("Editable"))
				continue;

			foreach (BaseContainer edComp : bucket)
			{
				BaseContainer info = edComp.GetObject("m_UIInfo");
				if (!info)
					info = edComp.GetObject("UIInfo");
				if (!info)
					continue;

				string desc;
				if (info.Get("Description", desc) && !desc.IsEmpty())
					return CleanToken(desc);
			}
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Determine weapon family from path and filename.
	static string ExtractFamily(string filePath)
	{
		string upper = filePath;
		upper.ToUpper();

		if (upper.Contains("2B14") || upper.Contains("PODNOS")) return "2B14";
		if (upper.Contains("M252")) return "M252";
		if (upper.Contains("M2HB") || upper.Contains("M2_")) return "M2HB";
		if (upper.Contains("NSV")) return "NSV";
		if (upper.Contains("PKM")) return "PKM";
		if (upper.Contains("M60")) return "M60";
		if (upper.Contains("KPVT")) return "KPVT";
		if (upper.Contains("6T5")) return "6T5";
		if (upper.Contains("6T7")) return "6T7";
		if (upper.Contains("M3")) return "M3";
		if (upper.Contains("M122")) return "M122";

		return "StaticWeapon";
	}

	//------------------------------------------------------------------------------------------------
	//! Determine faction affiliation.
	static string ExtractFaction(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		// 1. Check FactionAffiliationComponent
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("FactionAffiliation"))
				continue;

			foreach (BaseContainer facComp : bucket)
			{
				string factionKey;
				if (facComp.Get("m_sAffiliatedFaction", factionKey) && !factionKey.IsEmpty())
					return factionKey;
			}
		}

		// 2. Path inference
		string upper = filePath;
		upper.ToUpper();
		if (upper.Contains("USSR") || upper.Contains("OPFOR") || upper.Contains("2B14") || upper.Contains("6T5") || upper.Contains("6T7") || upper.Contains("NSV"))
			return "USSR";
		if (upper.Contains("US_") || upper.Contains("BLUFOR") || upper.Contains("M252") || upper.Contains("M2HB") || upper.Contains("M122") || upper.Contains("M3_"))
			return "US";
		if (upper.Contains("FIA") || upper.Contains("INDFOR"))
			return "FIA";

		return "UNKNOWN";
	}

	//------------------------------------------------------------------------------------------------
	//! Clean localization token into plain English text.
	static string CleanToken(string raw)
	{
		if (raw.IsEmpty())
			return string.Empty;

		string s = raw;
		if (s.StartsWith("#"))
			s = s.Substring(1, s.Length() - 1);
		if (s.StartsWith("AR-"))
			s = s.Substring(3, s.Length() - 3);

		if (s.StartsWith("EditableEntity_"))
			s = s.Substring(15, s.Length() - 15);
		else if (s.StartsWith("Weapon_"))
			s = s.Substring(7, s.Length() - 7);
		else if (s.StartsWith("Vehicle_"))
			s = s.Substring(8, s.Length() - 8);

		if (s.EndsWith("_Name"))
			s = s.Substring(0, s.Length() - 5);

		s.Replace("_USSR", "");
		s.Replace("_US", "");
		s.Replace("_FIA", "");
		s.Replace("_", " ");
		return s.Trim();
	}

	//------------------------------------------------------------------------------------------------
	//! Convert filename stem into a clean readable title.
	static string HumanizeStem(string filePath)
	{
		string s = filePath;
		int slash = s.LastIndexOf("/");
		if (slash >= 0)
			s = s.Substring(slash + 1, s.Length() - slash - 1);
		if (s.EndsWith(".et"))
			s = s.Substring(0, s.Length() - 3);

		s.Replace("_", " ");
		return s.Trim();
	}
}
