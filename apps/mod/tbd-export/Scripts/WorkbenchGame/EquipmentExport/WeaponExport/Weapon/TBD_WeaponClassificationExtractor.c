//------------------------------------------------------------------------------------------------
// TBD_WeaponClassificationExtractor.c
//
// Decides what kind of weapon a prefab is: its weapon type, whether it is a launcher or a mine,
// and whether it carries a bipod.
//
// The engine's weapon type arrives as an integer enum and is mapped to the catalog's string here.
// Bipod detection is a recursive container search rather than a component lookup, because a bipod
// reaches a weapon as a nested attachment rather than as a component of its own.
//------------------------------------------------------------------------------------------------

class TBD_WeaponClassificationExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract classification: weapon type and equip slot.
	static void ExtractClassification(map<string, ref array<BaseContainer>> comps, TBD_WeaponClassificationInfo outClass, string category)
	{
		string weaponType = "";
		string slotType = "";

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("WeaponComponent"))
				continue;

			foreach (BaseContainer wc : bucket)
			{
				BaseContainer cur = wc;
				while (cur)
				{
					if (weaponType.IsEmpty())
					{
						int wtInt = -1;
						if (cur.Get("WeaponType", wtInt) && wtInt >= 0)
							weaponType = WeaponTypeIntToString(wtInt);
						else
							cur.GetDefaultAsString("WeaponType", weaponType);
					}

					if (slotType.IsEmpty())
					{
						cur.Get("WeaponSlotType", slotType);
						if (slotType.IsEmpty())
							cur.GetDefaultAsString("WeaponSlotType", slotType);
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Normalize weaponType
		if (weaponType.StartsWith("WT_"))
			weaponType = weaponType.Substring(3, weaponType.Length() - 3);

		if (weaponType.IsEmpty())
		{
			if (category == "rifles") weaponType = "Rifle";
			else if (category == "machine_guns") weaponType = "MachineGun";
			else if (category == "handguns") weaponType = "Handgun";
			else if (category == "launchers") weaponType = "RocketLauncher";
			else if (category == "flares") weaponType = "FlareLauncher";
			else if (category == "heavy_weapons") weaponType = "HeavyMachineGun";
			else if (category == "grenades") weaponType = "FragGrenade";
			else if (category == "explosives") weaponType = "Mine";
			else if (category == "underbarrel") weaponType = "GrenadeLauncher";
			else weaponType = "Weapon";
		}

		// Normalize slotType
		if (slotType.IsEmpty())
		{
			if (category == "handguns") slotType = "secondary";
			else if (category == "launchers") slotType = "launcher";
			else if (category == "flares") slotType = "item";
			else if (category == "heavy_weapons") slotType = "csw";
			else if (category == "grenades") slotType = "grenade";
			else if (category == "explosives") slotType = "item";
			else if (category == "underbarrel") slotType = "attachment";
			else slotType = "primary";
		}

		outClass.m_sWeaponType = weaponType;
		outClass.m_sWeaponSlotType = slotType;
	}

	//------------------------------------------------------------------------------------------------
	protected static string WeaponTypeIntToString(int wt)
	{
		switch (wt)
		{
			case 1: return "Rifle";
			case 2: return "GrenadeLauncher";
			case 3: return "SniperRifle";
			case 4: return "RocketLauncher";
			case 5: return "MachineGun";
			case 6: return "Handgun";
			case 7: return "FragGrenade";
			case 8: return "SmokeGrenade";
			case 9: return "Autocannon";
		}
		return "";
	}

	//------------------------------------------------------------------------------------------------
	//! Recursively inspects a container's properties, sub-objects, and arrays for bipod definitions.
	//! Detects bipod stabilization points, deployment points, and bone references (e.g. w_bipodleg).
	static bool ContainerContainsBipod(BaseContainer cont, int depth = 0)
	{
		if (!cont || depth > 3)
			return false;

		int nv = cont.GetNumVars();
		for (int v = 0; v < nv; v++)
		{
			string varName = cont.GetVarName(v);
			if (varName.IsEmpty())
				continue;

			string lower = varName;
			lower.ToLower();
			if (lower.Contains("bipod"))
			{
				bool bVal = false;
				if (cont.Get(varName, bVal) && bVal)
					return true;
				string sVal = "";
				if (cont.Get(varName, sVal) && !sVal.IsEmpty())
					return true;
			}

			// Sub-object
			BaseContainer subObj = cont.GetObject(varName);
			if (subObj)
			{
				string subCls = subObj.GetClassName();
				string subClsLower = subCls;
				subClsLower.ToLower();
				if (subClsLower.Contains("bipod"))
					return true;
				if (ContainerContainsBipod(subObj, depth + 1))
					return true;
			}

			// Sub-object array
			BaseContainerList list = cont.GetObjectArray(varName);
			if (list)
			{
				for (int i = 0, n = list.Count(); i < n; i++)
				{
					BaseContainer elem = list.Get(i);
					if (!elem)
						continue;
					string elemCls = elem.GetClassName();
					string elemClsLower = elemCls;
					elemClsLower.ToLower();
					if (elemClsLower.Contains("bipod"))
						return true;
					int elemNv = elem.GetNumVars();
					for (int ev = 0; ev < elemNv; ev++)
					{
						string eVar = elem.GetVarName(ev);
						string strVal;
						if (elem.Get(eVar, strVal) && !strVal.IsEmpty())
						{
							string strLower = strVal;
							strLower.ToLower();
							if (strLower.Contains("bipod"))
								return true;
						}
					}
					if (ContainerContainsBipod(elem, depth + 1))
						return true;
				}
			}
		}
		return false;
	}
}
