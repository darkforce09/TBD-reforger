/** Inventory, body mass and individual storage compartments used by the standard domain readers. */
class TBD_ItemInventoryExtractor
{
	static BaseContainer Attributes(map<string, ref array<BaseContainer>> components)
	{
		foreach (string className, array<BaseContainer> instances : components)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "InventoryItemComponent")) continue;
			foreach (BaseContainer instance : instances)
			{
				if (TBD_EquipmentComponentGraph.StructuralPath(instance).Contains("/children/")) continue;
				BaseContainer attributes = instance.GetObject("Attributes");
				if (attributes) return attributes;
			}
		}
		return null;
	}

	static string Inventory(map<string, ref array<BaseContainer>> components)
	{
		BaseContainer attributes = Attributes(components);
		if (!attributes) return "";
		BaseContainer physical = attributes.GetObject("ItemPhysAttributes");
		string json = TBD_EquipmentExportJson.Fields(physical, "mass=Weight|item_volume=ItemVolume|item_dimensions=ItemDimensions|size_setup_strategy=SizeSetupStrategy", "/inventory");
		json = TBD_EquipmentExportJson.Member(json, "inventory_size", TBD_EquipmentExportJson.Field(attributes, "m_Size", "/inventory/inventory_size"));
		json = TBD_EquipmentExportJson.Member(json, "stackable", TBD_EquipmentExportJson.Field(attributes, "m_bStackable", "/inventory/stackable"));
		json = TBD_EquipmentExportJson.Member(json, "equipment_slot", TBD_EquipmentExportJson.Field(attributes, "m_SlotType", "/inventory/equipment_slot"));
		json = TBD_EquipmentExportJson.Member(json, "common_item_type", TBD_EquipmentExportJson.Field(attributes, "CommonItemType", "/inventory/common_item_type"));
		json = TBD_EquipmentExportJson.Member(json, "refundable", TBD_EquipmentExportJson.Field(attributes, "m_bRefundable", "/inventory/refundable"));
		BaseContainerList custom = TBD_EquipmentDisplayAttributes.GetCustomAttributes(attributes);
		array<string> modifiers = {};
		if (custom)
		{
			for (int i = 0; i < custom.Count(); i++)
			{
				BaseContainer modifier = custom.Get(i);
				if (!modifier || !TBD_EquipmentComponentGraph.IsA(modifier.GetClassName(), "CharacterModifierAttributes")) continue;
				string path = "/inventory/character_modifiers/" + modifiers.Count().ToString();
				string bindings = "stance_limits=StanceLimits|speed_limit=SpeedLimit|ads_speed_limit=ADSSpeedLimit|primary_action_speed_limit=SpeedLimitItemPrimaryAction|high_ready_speed_limit=SpeedLimitHighready";
				bindings += "|turn_limit=TurnLimit|default_ads_speed_modifier=DefaultADSSpeedModifier|suppress_one_hand_shooting=Supress1hShooting|allow_gadget_use=AllowGadgetUse|can_equip_in_vehicle=CanBeEquippedInVehicle|allow_reloading_with_roll=AllowReloadingWithRoll|allow_jumping=AllowJumping";
				string entry = TBD_EquipmentExportJson.Fields(modifier, bindings, path);
				modifiers.Insert(TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(modifier)));
			}
		}
		if (!modifiers.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "character_modifiers", "[" + TBD_EquipmentExportJson.Join(modifiers) + "]");
		return json;
	}

	static string Physics(map<string, ref array<BaseContainer>> components)
	{
		array<BaseContainer> bodies = components.Get("RigidBody");
		if (!bodies || bodies.IsEmpty()) return "";
		array<string> entries = {};
		foreach (BaseContainer body : bodies)
		{
			string path = "/physics/bodies/" + entries.Count().ToString();
			string entry = TBD_EquipmentExportJson.Fields(body, "rigid_body_mass=Mass", path);
			entries.Insert(TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(body)));
		}
		return "{\"bodies\":[" + TBD_EquipmentExportJson.Join(entries) + "]}";
	}

	static string Storage(map<string, ref array<BaseContainer>> components)
	{
		return StorageAtPath(components, "/storage");
	}

	//! Keep storage ownership and metadata paths distinct when a pouch is installed more than once.
	static string StorageAtPath(map<string, ref array<BaseContainer>> components, string outputPath)
	{
		array<string> compartments = {};
		foreach (string className, array<BaseContainer> instances : components)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "BaseInventoryStorageComponent")) continue;
			foreach (BaseContainer storage : instances)
			{
				string path = outputPath + "/compartments/" + compartments.Count().ToString();
				string bindings = "max_cumulative_volume=MaxCumulativeVolume|max_item_dimensions=MaxItemSize|max_weight=m_fMaxWeight";
				bindings += "|use_capacity_coefficient=UseCapacityCoefficient|capacity_coefficient=CapacityCoefficient|storage_purpose=StoragePurpose|priority=Priority";
				string json = TBD_EquipmentExportJson.Fields(storage, bindings, path);
				json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(storage));
				json = TBD_EquipmentExportJson.Member(json, "slots", Slots(storage, path));
				json = TBD_EquipmentExportJson.Member(json, "initial_storage_slots", InitialStorageSlots(storage, path));
				if (TBD_EquipmentComponentGraph.IsA(className, "SCR_FilteredInventoryStorageComponent"))
					json = TBD_EquipmentExportJson.Member(json, "allowed_item_types", TBD_EquipmentExportJson.Field(storage, "m_aAllowedItemTypes", path + "/allowed_item_types"));
				compartments.Insert(json);
			}
		}
		string initialItems = InitialInventoryItems(components, outputPath + "/initial_inventory");
		string resources = ResourceStorage(components, outputPath + "/resources");
		if (compartments.IsEmpty() && initialItems.IsEmpty() && resources.IsEmpty()) return "";
		string result = "{\"compartments\":[" + TBD_EquipmentExportJson.Join(compartments) + "]}";
		if (!initialItems.IsEmpty()) result = TBD_EquipmentExportJson.Member(result, "initial_inventory", initialItems);
		if (!resources.IsEmpty()) result = TBD_EquipmentExportJson.Member(result, "resources", resources);
		return result;
	}

	//! Preserve supply/electricity resource containers separately from inventory volume and weight limits.
	protected static string ResourceStorage(map<string, ref array<BaseContainer>> components, string outputPath)
	{
		array<string> entries = {};
		foreach (string className, array<BaseContainer> instances : components)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "SCR_ResourceComponent")) continue;
			foreach (BaseContainer component : instances)
			{
				string path = outputPath + "/" + entries.Count().ToString();
				string json = TBD_EquipmentExportJson.Fields(component, "disabled_types=m_aDisabledResourceTypes|fixed_enable_types=m_aDisallowChangingEnableResource", path);
				json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(component));
				string bindings = "resource_type=m_eResourceType|resource_rights=m_eResourceRights|storage_type=m_eStorageType";
				bindings += "|initial_value=m_fResourceValueCurrent|maximum_value=m_fResourceValueMax|on_empty_behavior=m_eOnEmptyBehavior";
				bindings += "|gain_enabled=m_bEnableResourceGain|gain=m_fResourceGain|gain_tick_rate=m_fResourceGainTickrate|gain_timeout=m_fResourceGainTimeout";
				bindings += "|decay_enabled=m_bEnableResourceDecay|decay=m_fResourceDecay|decay_tick_rate=m_fResourceDecayTickrate|decay_timeout=m_fResourceDecayTimeout";
				string containers = TBD_EquipmentExportJson.ObjectArray(component, "m_aContainers", bindings, path + "/containers");
				entries.Insert(TBD_EquipmentExportJson.Member(json, "containers", containers));
			}
		}
		if (entries.IsEmpty()) return "";
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	static string Slots(BaseContainer storage, string path)
	{
		array<string> entries = {};
		BaseContainerList slots = storage.GetObjectArray("MultiSlots");
		if (!slots) return "null";
		for (int i = 0; i < slots.Count(); i++)
		{
			BaseContainer slot = slots.Get(i);
			if (!slot) { entries.Insert("null"); continue; }
			string slotPath = path + "/slots/" + i.ToString();
			string entry = TBD_EquipmentExportJson.Fields(slot, "initial_slot_count=NumSlots", slotPath);
			entry = TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(slot));
			BaseContainer template = slot.GetObject("SlotTemplate");
			if (template)
			{
				string data = StorageSlot(template, slotPath + "/template");
				entry = TBD_EquipmentExportJson.Member(entry, "template", data);
			}
			else entry = TBD_EquipmentExportJson.Member(entry, "template", "null");
			entries.Insert(entry);
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Capture authored slot restrictions and defaults without treating disabled slots as populated.
	protected static string StorageSlot(BaseContainer slot, string path)
	{
		string bindings = "enabled=Enabled|prefab=Prefab|pivot_id=PivotID|child_pivot_id=ChildPivotID|offset=Offset|angles=Angles";
		if (TBD_EquipmentComponentGraph.IsA(slot.GetClassName(), "EquipmentStorageSlot"))
			bindings += "|allowed_item_types=AllowedItemTypes|affected_by_occluders=AffectedByOcluders";
		string json = TBD_EquipmentExportJson.Fields(slot, bindings, path);
		return TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(slot));
	}

	//! Retain the native order, null entries and individual identity of fixed equipment slots.
	protected static string InitialStorageSlots(BaseContainer storage, string path)
	{
		BaseContainerList slots = storage.GetObjectArray("InitialStorageSlots");
		if (!slots) return "null";
		array<string> entries = {};
		for (int i = 0; i < slots.Count(); i++)
		{
			BaseContainer slot = slots.Get(i);
			if (!slot) entries.Insert("null");
			else entries.Insert(StorageSlot(slot, path + "/initial_storage_slots/" + i.ToString()));
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Keep initial-item target paths and ordered prefab repetitions exactly as authored.
	protected static string InitialInventoryItems(map<string, ref array<BaseContainer>> components, string path)
	{
		array<string> managers = {};
		foreach (string className, array<BaseContainer> instances : components)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "InventoryStorageManagerComponent")) continue;
			foreach (BaseContainer manager : instances)
			{
				string managerPath = path + "/" + managers.Count().ToString();
				string entry = "{}";
				entry = TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(manager));
				BaseContainerList configurations = manager.GetObjectArray("InitialInventoryItems");
				string items = "null";
				if (configurations)
				{
					array<string> entries = {};
					for (int i = 0; i < configurations.Count(); i++)
					{
						BaseContainer configuration = configurations.Get(i);
						if (!configuration) { entries.Insert("null"); continue; }
						string fields = "target_storage=TargetStorage|prefabs_to_spawn=PrefabsToSpawn";
						string data = TBD_EquipmentExportJson.Fields(configuration, fields, managerPath + "/items/" + i.ToString());
						entries.Insert(TBD_EquipmentExportJson.Member(data, "source", TBD_EquipmentExportJson.Context(configuration)));
					}
					items = "[" + TBD_EquipmentExportJson.Join(entries) + "]";
				}
				managers.Insert(TBD_EquipmentExportJson.Member(entry, "items", items));
			}
		}
		if (managers.IsEmpty()) return "";
		return "[" + TBD_EquipmentExportJson.Join(managers) + "]";
	}
}
