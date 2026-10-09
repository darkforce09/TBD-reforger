/**
 * TBD_VehicleExportNaming.c
 *
 * Generic string humanization, localization token sanitation, and platform
 * designation formatting for vehicle catalog export.
 */

class TBD_VehicleExportNaming
{
	//------------------------------------------------------------------------------------------------
	//! Dynamically format platform identifier into a clean display title (e.g. BTR70 -> BTR-70).
	static string FormatPlatformDisplayName(string platformId)
	{
		if (platformId.IsEmpty())
			return "Vehicle Platform";

		// Preserve standard single-letter military prefixes intact (M998, M923A1, M151A2)
		if (platformId.StartsWith("M") && platformId.Length() > 2 && IsDigit(platformId.Substring(1, 1)))
			return platformId;

		string formatted = "";
		int len = platformId.Length();
		for (int i = 0; i < len; i++)
		{
			string ch = platformId.Substring(i, 1);
			if (i > 0)
			{
				string prev = platformId.Substring(i - 1, 1);
				if (IsLetter(prev) && IsDigit(ch))
					formatted += "-";
			}
			formatted += ch;
		}
		formatted.Replace("_", " ");
		return formatted.Trim();
	}

	//------------------------------------------------------------------------------------------------
	//! Convert underscores to spaces and trim whitespace.
	static string HumanizeStem(string text)
	{
		string s = text;
		s.Replace("_", " ");
		return s.Trim();
	}

	//------------------------------------------------------------------------------------------------
	//! Strip standard Enfusion stringtable token prefixes and suffixes.
	static string CleanNameFromToken(string token)
	{
		string t = token;
		t.Replace("AR-Vehicle_", "");
		t.Replace("AR-Weapon_", "");
		t.Replace("AR-Item_", "");
		t.Replace("AR-", "");
		t.Replace("_Name", "");
		t.Replace("_", " ");
		return t.Trim();
	}

	//------------------------------------------------------------------------------------------------
	//! Check if a stem matches a generic engine vehicle base class.
	static bool IsGenericVehicleBaseStem(string stem)
	{
		string s = stem;
		s.ToLower();
		return (s == "vehicle_base" || s == "wheeled_base" || s == "wheeled_apc_base"
			|| s == "truck_base" || s == "car_base" || s == "helicopter_base"
			|| s == "plane_base" || s == "boat_base" || s == "tracked_base");
	}

	//------------------------------------------------------------------------------------------------
	static bool IsLetter(string s)
	{
		string alpha = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
		return alpha.Contains(s);
	}

	//------------------------------------------------------------------------------------------------
	static bool IsDigit(string s)
	{
		string digits = "0123456789";
		return digits.Contains(s);
	}
}
