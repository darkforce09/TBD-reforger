/**
 * Reads effective magazine capacity, native wells, authored designations and
 * ordered ammunition configuration for the standard ammunition catalogs.
 */

//! Reads magazine source values without filename classifications or calculated masses.
class TBD_AmmoMagazineExtractor
{
	//! Export actual native well types without accumulating ancestor copies.
	static void ExtractMagazineWells(map<string, ref array<BaseContainer>> comps, notnull array<string> outWells)
	{
		outWells.Clear();
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMagazine(cls)) continue;
			foreach (BaseContainer component : bucket)
			{
				BaseContainer well = component.GetObject("MagazineWell");
				if (well) TBD_EquipmentComponentGraph.AddUniqueType(outWells, well.GetClassName());
				ReadWells(component.GetObjectArray("MagazineWells"), outWells);
				ReadWells(component.GetObjectArray("m_aMagazineWells"), outWells);
			}
		}
	}

	//! Read capacity and inventory ammunition mass; explicit zero remains a valid value.
	static void ExtractCapacity(map<string, ref array<BaseContainer>> comps, TBD_MagazineCapacityInfo outCap, string filePath)
	{
		outCap.m_iRoundCapacity = -1;
		outCap.m_fWeightPerRoundKg = -1;
		outCap.m_sAmmoStyle = string.Empty;
		bool capacityRead;
		bool massRead;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer component : bucket)
			{
				if (IsMagazine(cls) && !capacityRead)
					capacityRead = component.Get("MaxAmmo", outCap.m_iRoundCapacity);
				if (TBD_EquipmentComponentGraph.IsA(cls, "InventoryMagazineComponent") && !massRead)
					massRead = component.Get("WeightPerAmmo", outCap.m_fWeightPerRoundKg);
			}
		}
	}

	//! Keep original caliber/type text and native flags from MagazineUIInfo.
	static void ExtractCaliber(map<string, ref array<BaseContainer>> comps, TBD_MagazineCaliberInfo outCaliber, string filePath)
	{
		outCaliber.m_sCaliberName = string.Empty;
		outCaliber.m_sAmmoType = string.Empty;
		outCaliber.m_iAmmoTypeFlags = 0;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMagazine(cls)) continue;
			foreach (BaseContainer component : bucket)
			{
				BaseContainer ui = component.GetObject("UIInfo");
				if (!ui || !TBD_EquipmentComponentGraph.IsA(ui.GetClassName(), "MagazineUIInfo")) continue;
				ui.Get("m_sAmmoCaliber", outCaliber.m_sCaliberName);
				ui.Get("m_sAmmoType", outCaliber.m_sAmmoType);
				ui.Get("m_eAmmoTypeFlags", outCaliber.m_iAmmoTypeFlags);
				return;
			}
		}
	}

	//! Retain the first magazine's ordered projectile table; do not append its default as a table entry.
	static void ExtractAmmoConfig(map<string, ref array<BaseContainer>> comps, notnull array<string> outAmmoResources, out string outConfPath)
	{
		outConfPath = string.Empty;
		outAmmoResources.Clear();
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMagazine(cls)) continue;
			foreach (BaseContainer component : bucket)
			{
				if (!component.Get("AmmoConfig", outConfPath)) continue;
				if (outConfPath.IsEmpty()) return;
				Resource resource = Resource.Load(outConfPath);
				if (!resource || !resource.IsValid() || !resource.GetResource())
				{
					TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "AmmoConfig", "Cannot load " + outConfPath);
					return;
				}
				BaseContainer config = resource.GetResource().ToBaseContainer();
				array<ResourceName> resourceNames = {};
				if (!config || !config.Get("AmmoResourceArray", resourceNames))
					TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "AmmoConfig/AmmoResourceArray", "Cannot read " + outConfPath);
				else
				{
					foreach (ResourceName name : resourceNames)
						outAmmoResources.Insert(name);
				}
				return;
			}
		}
	}

	//! Preserve inventory measurements; loaded mass belongs to later processing.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, int capacity, float weightPerRound, TBD_MagazinePhysicalInfo outPhys)
	{
		outPhys.m_fWeightFullKg = -1;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "InventoryItemComponent")) continue;
			foreach (BaseContainer component : bucket)
			{
				BaseContainer attrs = component.GetObject("Attributes");
				if (!attrs) continue;
				int size;
				if (attrs.Get("m_Size", size)) outPhys.m_sInventorySize = size.ToString();
				BaseContainer physical = attrs.GetObject("ItemPhysAttributes");
				if (!physical) continue;
				physical.Get("Weight", outPhys.m_fWeightEmptyKg);
				physical.Get("ItemVolume", outPhys.m_fVolumeCm3);
				physical.Get("ItemDimensions", outPhys.m_vDimensions);
				return;
			}
		}
	}

	//! Emit separate magazine installations with exact ordered configuration and provenance.
	static string ExtractConfiguration(map<string, ref array<BaseContainer>> comps, string outputPath = "/magazines")
	{
		array<string> instances = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMagazine(cls)) continue;
			foreach (BaseContainer component : bucket)
			{
				string path = outputPath + "/" + instances.Count().ToString();
				string entry = "{\"instance_id\":" + TBD_EquipmentExportJson.Quote(TBD_EquipmentComponentGraph.InstanceId(component));
				entry += ",\"native_class\":" + TBD_EquipmentExportJson.Quote(cls);
				entry += ",\"capacity\":" + TBD_EquipmentExportJson.Field(component, "MaxAmmo", path + "/capacity");
				entry += ",\"ammo_config\":" + TBD_EquipmentExportJson.Field(component, "AmmoConfig", path + "/ammo_config");
				entry += ",\"ammo_mapping\":" + TBD_EquipmentExportJson.Field(component, "AmmoMapping", path + "/ammo_mapping");
				entry += ",\"default_projectile\":" + TBD_EquipmentExportJson.Field(component, "AmmoTemplate", path + "/default_projectile");
				BaseContainer ui = component.GetObject("UIInfo");
				entry += ",\"designation\":" + TBD_EquipmentExportJson.Fields(ui, "caliber=m_sAmmoCaliber|ammunition_type=m_sAmmoType|ammunition_type_flags=m_eAmmoTypeFlags", path + "/designation");
				BaseContainer well = component.GetObject("MagazineWell");
				string wellValue = "null";
				if (well) wellValue = TBD_EquipmentExportJson.Quote(well.GetClassName());
				entry += ",\"magazine_well\":" + wellValue;
				entry += ",\"ammo_resources\":" + ReadConfigurationResources(component, path + "/ammo_resources");
				entry += ",\"inventory_ammunition\":" + ReadInventoryAmmunition(comps, component, path + "/inventory_ammunition");
				instances.Insert(entry + "}");
			}
		}
		return "[" + TBD_EquipmentExportJson.Join(instances) + "]";
	}

	//! Retain ammunition mass from inventory instances owned by this magazine's entity.
	protected static string ReadInventoryAmmunition(map<string, ref array<BaseContainer>> comps, BaseContainer magazine, string path)
	{
		array<string> instances = {};
		string magazineOwner = ComponentOwnerPath(magazine);
		if (magazineOwner.IsEmpty())
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Magazine ownership is unavailable");
			return "null";
		}
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "InventoryMagazineComponent")) continue;
			foreach (BaseContainer inventory : bucket)
			{
				if (ComponentOwnerPath(inventory) != magazineOwner) continue;
				string itemPath = path + "/" + instances.Count().ToString();
				string entry = TBD_EquipmentExportJson.Context(inventory);
				string mass = TBD_EquipmentExportJson.Field(inventory, "WeightPerAmmo", itemPath + "/weight_per_ammunition");
				instances.Insert(TBD_EquipmentExportJson.Member(entry, "weight_per_ammunition", mass));
			}
		}
		return "[" + TBD_EquipmentExportJson.Join(instances) + "]";
	}

	//! Structural component paths identify the owning effective entity without using prefab filenames.
	protected static string ComponentOwnerPath(BaseContainer component)
	{
		string location = TBD_EquipmentComponentGraph.StructuralPath(component);
		int separator = location.LastIndexOf("/components/");
		if (separator < 0) return string.Empty;
		return TBD_EquipmentExportJson.PreserveSubstring(location, 0, separator);
	}

	//! Load the actual configured resource table, preserving order, duplicates and empty entries.
	protected static string ReadConfigurationResources(BaseContainer component, string path)
	{
		string configName;
		if (!component.Get("AmmoConfig", configName) || configName.IsEmpty()) return "null";
		Resource resource = Resource.Load(configName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Cannot load " + configName);
			return "null";
		}
		BaseContainer config = resource.GetResource().ToBaseContainer();
		if (!config)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "AmmoConfig is not a container: " + configName);
			return "null";
		}
		return TBD_EquipmentExportJson.Field(config, "AmmoResourceArray", path);
	}

	//! Existing catalog groups use explicit native well relationships; unknown types remain unclassified.
	static string CategorizeMagazine(array<string> wells, int capacity, string filePath)
	{
		foreach (string well : wells)
		{
			if (MatchesWell(well, "MagazineWellStanag556|MagazineWellAK545|MagazineWellM14|MagazineWellSVD|MagazineWellVZ58_762")) return "magazines_rifle";
			if (MatchesWell(well, "MagazineWellM249|MagazineWellM240|MagazineWellM60|MagazineWellPKM|MagazineWellUK59")) return "magazines_mg";
			if (MatchesWell(well, "MagazineWellM9Beretta|MagazineWellMakarovPM")) return "magazines_handgun";
			if (MatchesWell(well, "MagazineWellM2HB|MagazineWellNSV|MagazineWellKPVT|MagazineWell_Cannon_M242_APDST|MagazineWell_Cannon_M242_HEIT")) return "magazines_heavy";
			if (MatchesWell(well, "MagazineWellM203|MagazineWellUS_UGL|MagazineWellRU_GP")) return "magazines_grenades";
			if (MatchesWell(well, "MagazineWellRPG7")) return "magazines_rockets";
		}
		return "magazines_other";
	}

	//! Match verified native well classes and their addon-defined subclasses.
	protected static bool MatchesWell(string well, string accepted)
	{
		array<string> candidates = {};
		accepted.Split("|", candidates, true);
		foreach (string candidate : candidates)
			if (TBD_EquipmentComponentGraph.IsA(well, candidate)) return true;
		return false;
	}

	//! Match native magazine components rather than arbitrary class-name fragments.
	protected static bool IsMagazine(string cls)
	{
		return TBD_EquipmentComponentGraph.IsA(cls, "BaseMagazineComponent") || TBD_EquipmentComponentGraph.IsA(cls, "MagazineComponent");
	}

	//! Preserve explicitly configured well types without adding ancestor-prefab values.
	protected static void ReadWells(BaseContainerList wells, array<string> output)
	{
		if (!wells) return;
		for (int i = 0; i < wells.Count(); i++)
		{
			BaseContainer well = wells.Get(i);
			if (well) TBD_EquipmentComponentGraph.AddUniqueType(output, well.GetClassName());
		}
	}
}
