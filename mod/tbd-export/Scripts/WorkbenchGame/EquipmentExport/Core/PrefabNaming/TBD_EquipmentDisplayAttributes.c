/** Reads effective resource-owned UI attributes and resolves official English strings. */
class TBD_EquipmentDisplayAttributes
{
	static BaseContainer DisplayContainer(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string vehicleClass, array<BaseContainer> vehicleInstances : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(vehicleClass, "SCR_EditableVehicleComponent")) continue;
			foreach (BaseContainer vehicleInstance : vehicleInstances)
			{
				if (TBD_EquipmentComponentGraph.StructuralPath(vehicleInstance).Contains("/children/")) continue;
				BaseContainer vehicleUI = vehicleInstance.GetObject("m_UIInfo");
				if (vehicleUI) return vehicleUI;
			}
		}
		BaseContainer attributes = TBD_ItemInventoryExtractor.Attributes(comps);
		if (attributes) return attributes.GetObject("ItemDisplayName");
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "WeaponComponent") && !TBD_EquipmentComponentGraph.IsA(cls, "SCR_EditableEntityComponent")) continue;
			foreach (BaseContainer component : bucket)
			{
				if (TBD_EquipmentComponentGraph.StructuralPath(component).Contains("/children/")) continue;
				BaseContainer ui = component.GetObject("UIInfo");
				if (ui) return ui;
			}
		}
		return null;
	}

	static string RawDisplayNameFor(map<string, ref array<BaseContainer>> comps)
	{
		BaseContainer display = DisplayContainer(comps);
		string value;
		if (display) display.Get("Name", value);
		return value;
	}

	static string RawDescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		BaseContainer display = DisplayContainer(comps);
		string value;
		if (display) display.Get("Description", value);
		return value;
	}

	static string RawIconFor(map<string, ref array<BaseContainer>> comps)
	{
		BaseContainer display = DisplayContainer(comps);
		string value;
		if (display) display.Get("Icon", value);
		return value;
	}

	static string English(string raw)
	{
		if (raw.IsEmpty()) return string.Empty;
		string language;
		WidgetManager.GetLanguage(language);
		WidgetManager.SetLanguage("en_us");
		string resolved = WidgetManager.Translate(raw);
		WidgetManager.SetLanguage(language);
		if (raw.StartsWith("#") && (resolved == raw || resolved.IsEmpty())) return string.Empty;
		return resolved;
	}

	static string Names(map<string, ref array<BaseContainer>> comps)
	{
		BaseContainer display = DisplayContainer(comps);
		string json = TBD_EquipmentExportJson.Fields(display, "original=Name|description_original=Description", "/names");
		string raw = RawDisplayNameFor(comps);
		string resolved = English(raw);
		string key = "null";
		if (raw.StartsWith("#")) key = TBD_EquipmentExportJson.Quote(raw);
		json = TBD_EquipmentExportJson.Member(json, "localization_key", key);
		string english = "null";
		string status = "unavailable";
		if (!resolved.IsEmpty()) { english = TBD_EquipmentExportJson.Quote(resolved); status = "present"; }
		json = TBD_EquipmentExportJson.Member(json, "display_name_en", english);
		json = TBD_EquipmentExportJson.Member(json, "locale", "\"en_us\"");
		json = TBD_EquipmentExportJson.Member(json, "resolution_status", TBD_EquipmentExportJson.Quote(status));
		return json;
	}
	//------------------------------------------------------------------------------------------------
	//! Custom attribute list off an Attributes container, flat or wrapped one level deep.
	static BaseContainerList GetCustomAttributes(BaseContainer attrs)
	{
		if (!attrs)
			return null;

		BaseContainerList list = attrs.GetObjectArray("CustomAttributes");
		if (list)
			return list;

		BaseContainer subCustom = attrs.GetObject("CustomAttributes");
		if (subCustom)
		{
			BaseContainerList wrapped = subCustom.GetObjectArray("CustomAttributes");
			if (wrapped)
				return wrapped;
		}

		return null;
	}
}
