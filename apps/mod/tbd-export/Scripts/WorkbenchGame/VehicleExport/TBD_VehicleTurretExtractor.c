/**
 * TBD_VehicleTurretExtractor.c
 *
 * Dedicated extractor for vehicle turrets, cupolas, weapon stations, mounted armaments,
 * ballistic muzzles, ammunition capacities, fire modes, and optical sights.
 * Operates purely via Enfusion BaseContainer reflection without hardcoded weapon tables.
 */

class TBD_VehicleTurretExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspect all mounted turrets and weapons on the vehicle variant.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		array<string> ancestry = {};
		array<string> entries = {};
		ancestry.Insert(varData.m_sResourceName);
		ReadInstallations(comps, varData, varData.m_sResourceName, "root", ancestry, entries);
		varData.m_sTurretsJson = "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Traverse configured installation references; the active path detects cycles, not repeated prefabs.
	protected static void ReadInstallations(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant vehicle, string resourceName, string installation, array<string> ancestry, array<string> entries, BaseContainer configuredSlot = null, string prefabProperty = "")
	{
		if (ancestry.Count() > 32)
		{
			TBD_EquipmentExportJson.ExtractionError(vehicle.m_sResourceName, installation, "Installation traversal limit");
			return;
		}
		int firstEntry = entries.Count();
		AppendCurrentInstallations(comps, resourceName, installation, entries);
		if (configuredSlot && entries.Count() > firstEntry)
		{
			string configurationPath = "/installations/" + firstEntry.ToString();
			string configured = TBD_EquipmentExportJson.Fields(configuredSlot, "enabled=Enabled", configurationPath + "/configured_slot");
			if (TBD_EquipmentComponentGraph.IsA(configuredSlot.GetClassName(), "WeaponSlotComponent"))
			{
				string controls = TBD_EquipmentExportJson.Fields(configuredSlot, "aiming_type=useAimingType|weapon_slot_index=WeaponSlotIndex", configurationPath + "/configured_slot/controls");
				configured = TBD_EquipmentExportJson.Member(configured, "controls", controls);
			}
			configured = TBD_EquipmentExportJson.Member(configured, "source", TBD_EquipmentExportJson.Context(configuredSlot));
			TBD_EquipmentExportJson.Field(configuredSlot, prefabProperty, configurationPath + "/resource_name");
			entries[firstEntry] = TBD_EquipmentExportJson.Member(entries[firstEntry], "configured_slot", configured);
		}
		array<string> slots = {};
		array<string> slotIds = {};
		array<BaseContainer> slotSources = {};
		array<string> prefabProperties = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer component : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "TurretControllerComponent") || TBD_EquipmentComponentGraph.IsA(cls, "TurretComponent"))
				{
					string path = "/installations/" + entries.Count().ToString();
					string bindings = "horizontal_limits=LimitsHoriz|vertical_limits=LimitsVert|reload_position=TurretReloadPosition|return_before_reload=ReturnToPositionBeforeReload";
					bindings += "|aiming_max_speed=AimingMaxSpeed|aiming_speed_controls=AimingSpeedControls|provided_aiming_type=ProvideAimingType|aim_only_in_ads=CanAimOnlyInADS";
					string settings = TBD_EquipmentExportJson.Fields(component, bindings, path);
					settings = TBD_EquipmentExportJson.Member(settings, "installation", TBD_EquipmentExportJson.Quote(OwnedInstallationId(installation, EntitySourcePath(component))));
					settings = TBD_EquipmentExportJson.Member(settings, "source", TBD_EquipmentExportJson.Context(component));
					entries.Insert(settings);
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "SlotManagerComponent"))
				{
					BaseContainerList configured = component.GetObjectArray("Slots");
					if (!configured) continue;
					for (int i = 0; i < configured.Count(); i++)
					{
						BaseContainer slot = configured.Get(i);
						string prefab;
						if (!slot || !slot.Get("Prefab", prefab) || prefab.IsEmpty()) continue;
						slots.Insert(prefab);
						slotSources.Insert(slot);
						prefabProperties.Insert("Prefab");
						string slotOwner = OwnedInstallationId(installation, EntitySourcePath(component));
						slotIds.Insert(slotOwner + "/" + TBD_EquipmentComponentGraph.InstanceId(component) + "/slots/" + i.ToString());
					}
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "WeaponSlotComponent"))
				{
					string weapon;
					if (!component.Get("WeaponTemplate", weapon) || weapon.IsEmpty()) continue;
					slots.Insert(weapon);
					slotSources.Insert(component);
					prefabProperties.Insert("WeaponTemplate");
					string weaponOwner = OwnedInstallationId(installation, EntitySourcePath(component));
					slotIds.Insert(weaponOwner + "/" + TBD_EquipmentComponentGraph.InstanceId(component));
				}
			}
		}
		for (int child = 0; child < slots.Count(); child++)
		{
			string resource = slots[child];
			if (ancestry.Find(resource) >= 0)
			{
				TBD_EquipmentExportJson.ExtractionError(vehicle.m_sResourceName, slotIds[child], "Installation reference cycle: " + resource);
				continue;
			}
			Resource loaded = Resource.Load(resource);
			if (!loaded || !loaded.IsValid() || !loaded.GetResource())
			{
				TBD_EquipmentExportJson.ExtractionError(vehicle.m_sResourceName, slotIds[child], "Cannot load installation: " + resource);
				continue;
			}
			BaseContainer root = loaded.GetResource().ToBaseContainer();
			if (!root)
			{
				TBD_EquipmentExportJson.ExtractionError(vehicle.m_sResourceName, slotIds[child], "Installation has no source container: " + resource);
				continue;
			}
			TBD_EquipmentComponentContext previous = TBD_EquipmentComponentGraph.SaveContext();
			map<string, ref array<BaseContainer>> childComponents = new map<string, ref array<BaseContainer>>();
			TBD_EquipmentComponentGraph.CollectComponentChain(root, childComponents, 128);
			TBD_EquipmentComponentGraph.m_CurrentResource = vehicle.m_sResourceName;
			TBD_VehicleCompartmentExtractor.Extract(childComponents, vehicle, slotIds[child]);
			ancestry.Insert(resource);
			ReadInstallations(childComponents, vehicle, resource, slotIds[child], ancestry, entries, slotSources[child], prefabProperties[child]);
			ancestry.Remove(ancestry.Count() - 1);
			TBD_EquipmentComponentGraph.RestoreContext(previous);
		}
	}

	//! Keep integral weapons and independently mounted sights with their owning entity installation.
	protected static void AppendCurrentInstallations(map<string, ref array<BaseContainer>> comps, string resourceName, string installation, array<string> entries)
	{
		array<string> entityPaths = {"root"};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			bool relevant = TBD_EquipmentComponentGraph.IsA(cls, "WeaponComponent");
			relevant = relevant || TBD_EquipmentComponentGraph.IsA(cls, "BaseSightsComponent");
			relevant = relevant || TBD_WeaponMuzzleExtractor.IsMuzzleComponentClass(cls);
			if (!relevant) continue;
			foreach (BaseContainer component : bucket)
			{
				string ownerPath = EntitySourcePath(component);
				if (!ownerPath.IsEmpty() && entityPaths.Find(ownerPath) < 0) entityPaths.Insert(ownerPath);
			}
		}
		foreach (string entityPath : entityPaths)
		{
			map<string, ref array<BaseContainer>> entityComponents = new map<string, ref array<BaseContainer>>();
			foreach (string className, array<BaseContainer> sources : comps)
			{
				array<BaseContainer> matches = {};
				foreach (BaseContainer source : sources)
					if (EntitySourcePath(source) == entityPath) matches.Insert(source);
				if (!matches.IsEmpty()) entityComponents.Insert(className, matches);
			}
			string entry = "{}";
			entry = TBD_EquipmentExportJson.Member(entry, "installation_id", TBD_EquipmentExportJson.Quote(OwnedInstallationId(installation, entityPath)));
			entry = TBD_EquipmentExportJson.Member(entry, "resource_name", TBD_EquipmentExportJson.Quote(resourceName));
			entry = TBD_EquipmentExportJson.Member(entry, "source_entity_path", TBD_EquipmentExportJson.Quote(entityPath));
			bool hasWeapon = TBD_EquipmentComponentGraph.HasCompSuffix(entityComponents, "WeaponComponent");
			foreach (string candidateClass, array<BaseContainer> candidates : entityComponents)
				if (TBD_WeaponMuzzleExtractor.IsMuzzleComponentClass(candidateClass)) hasWeapon = true;
			string previousPrefix = TBD_EquipmentExportJson.m_sFieldPrefix;
			TBD_EquipmentExportJson.m_sFieldPrefix = previousPrefix + "/installations/" + entries.Count().ToString();
			if (installation != "root" || entityPath != "root")
			{
				TBD_ItemToolInfo support = new TBD_ItemToolInfo();
				TBD_ItemExtractor.ExtractTool(entityComponents, support, resourceName, "/support");
				if (!support.m_sJson.IsEmpty()) entry = TBD_EquipmentExportJson.Member(entry, "support", support.m_sJson);
				string storage = TBD_ItemInventoryExtractor.Storage(entityComponents);
				if (!storage.IsEmpty()) entry = TBD_EquipmentExportJson.Member(entry, "storage", storage);
				if (TBD_EquipmentComponentGraph.HasCompSuffix(entityComponents, "DamageManagerComponent"))
				{
					TBD_VehicleDeepVariant installationData = new TBD_VehicleDeepVariant();
					installationData.m_sResourceName = resourceName;
					TBD_VehicleArmorExtractor.Extract(entityComponents, installationData);
					if (!installationData.m_sDamageJson.IsEmpty()) entry = TBD_EquipmentExportJson.Member(entry, "damage", installationData.m_sDamageJson);
				}
			}
			if (hasWeapon)
			{
				TBD_EquipmentExportJson.m_sFieldPrefix += "/weapon";
				TBD_WeaponInfo weaponInfo = new TBD_WeaponInfo();
				weaponInfo.m_sResourceName = resourceName;
				weaponInfo.m_sNamesJson = TBD_EquipmentDisplayAttributes.Names(entityComponents);
				TBD_WeaponClassificationExtractor.ExtractClassification(entityComponents, weaponInfo.m_Classification, string.Empty);
				TBD_WeaponExtractor.ExtractPhysical(entityComponents, weaponInfo.m_Physical);
				TBD_WeaponExtractor.ExtractSights(entityComponents, weaponInfo.m_Sights);
				TBD_WeaponMuzzleExtractor.ExtractMuzzles(entityComponents, weaponInfo.m_aMuzzles);
				TBD_WeaponMountingExtractor.ExtractAttachmentSlots(entityComponents, weaponInfo.m_aAttachmentSlots);
				entry = TBD_EquipmentExportJson.Member(entry, "weapon", weaponInfo.SerializeJson(""));
			}
			else if (TBD_EquipmentComponentGraph.HasCompSuffix(entityComponents, "BaseSightsComponent"))
			{
				TBD_WeaponSightsInfo sights = new TBD_WeaponSightsInfo();
				TBD_WeaponExtractor.ExtractSights(entityComponents, sights);
				entry = TBD_EquipmentExportJson.Member(entry, "sights", "[" + TBD_EquipmentExportJson.Join(sights.m_aInstances) + "]");
			}
			TBD_EquipmentExportJson.m_sFieldPrefix = previousPrefix;
			entries.Insert(entry);
		}
	}

	//! Locate the owning entity in the collector's structural path; component nesting keeps that owner.
	protected static string EntitySourcePath(BaseContainer component)
	{
		string path = TBD_EquipmentComponentGraph.StructuralPath(component);
		if (path.IsEmpty())
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "installations", "Missing component source path");
			return string.Empty;
		}
		string entityPath = "root";
		int childIndex = path.IndexOf("/children/");
		while (childIndex >= 0)
		{
			int end = path.IndexOfFrom(childIndex + 10, "/");
			if (end < 0) return path;
			entityPath = TBD_EquipmentExportJson.PreserveSubstring(path, 0, end);
			childIndex = path.IndexOfFrom(end, "/children/");
		}
		return entityPath;
	}

	//! Preserve the reference installation for the prefab root and qualify actual child entities.
	protected static string OwnedInstallationId(string installation, string entityPath)
	{
		if (entityPath == "root") return installation;
		return installation + "/" + entityPath;
	}

	//------------------------------------------------------------------------------------------------
	//! Inspect a child prefab attached to a slot: determine if it is a turret assembly.
	protected static void InspectChildSlot(string prefabPath, string slotName, TBD_VehicleDeepVariant varData, array<string> seen)
	{
		// Installation traversal is owned by ReadInstallations, which preserves each slot path.
	}
}
