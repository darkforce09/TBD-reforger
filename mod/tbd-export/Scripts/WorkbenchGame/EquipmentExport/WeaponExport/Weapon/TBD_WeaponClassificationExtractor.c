/** Effective weapon-component classifications and native capability inspection. */
class TBD_WeaponClassificationExtractor
{
	//! Keep each effective weapon component's native classification without category fallbacks.
	static void ExtractClassification(map<string, ref array<BaseContainer>> comps, TBD_WeaponClassificationInfo outClass, string category)
	{
		outClass.m_sSourceJson = "";
		outClass.m_sWeaponType = "";
		outClass.m_sWeaponSlotType = "";
		array<string> entries = {};
		foreach (string className, array<BaseContainer> instances : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "WeaponComponent")) continue;
			foreach (BaseContainer component : instances)
			{
				string path = "/classification/components/" + entries.Count().ToString();
				string entry = TBD_EquipmentExportJson.Fields(component, "weapon_type=WeaponType|weapon_slot_type=WeaponSlotType", path);
				entry = TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(component));
				entries.Insert(entry);
				if (entries.Count() == 1)
				{
					outClass.m_sWeaponType = NativeEnumName(component, "WeaponType");
					outClass.m_sWeaponSlotType = NativeEnumName(component, "WeaponSlotType");
				}
			}
		}
		if (!entries.IsEmpty()) outClass.m_sSourceJson = "{\"components\":[" + TBD_EquipmentExportJson.Join(entries) + "]}";
		if (entries.Count() > 1)
		{
			outClass.m_sWeaponType = "";
			outClass.m_sWeaponSlotType = "";
		}
	}

	//! Existing scalar callers receive only engine-provided enum names.
	protected static string NativeEnumName(BaseContainer source, string property)
	{
		int index = source.GetVarIndex(property);
		int value;
		if (index < 0 || !source.Get(property, value)) return "";
		array<string> names = {};
		array<int> values = {};
		source.GetEnumValues(index, names, values);
		for (int i = 0; i < values.Count() && i < names.Count(); i++)
			if (values[i] == value) return names[i];
		return "";
	}

	//! The native enum defines names; this helper does not maintain a second mapping.
	protected static string WeaponTypeIntToString(int wt)
	{
		return typename.EnumToString(EWeaponType, wt);
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
