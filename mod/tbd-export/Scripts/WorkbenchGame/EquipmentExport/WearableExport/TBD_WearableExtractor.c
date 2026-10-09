/**
 * Reads native loadout, storage, armor and visual configuration for wearable equipment.
 * Effective containers preserve child overrides and repeated installed slot identities.
 */
//! Extract wearable-specific gameplay values using native component inheritance.
class TBD_WearableExtractor
{
	//! Retain the root cloth area and ordered blocked slot types for category selection.
	static void ExtractWearableArea(map<string, ref array<BaseContainer>> comps, out string outAreaType, notnull array<string> outBlockedSlots)
	{
		outAreaType = string.Empty;
		outBlockedSlots.Clear();
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseLoadoutClothComponent")) continue;
			foreach (BaseContainer cloth : bucket)
			{
				if (TBD_EquipmentComponentGraph.StructuralPath(cloth).Contains("/children/")) continue;
				BaseContainer area = cloth.GetObject("AreaType");
				if (area) outAreaType = area.GetClassName();
				BaseContainerList blocked = cloth.GetObjectArray("BlockedSlots");
				if (blocked)
				{
					for (int b = 0; b < blocked.Count(); b++)
					{
						BaseContainer slot = blocked.Get(b);
						if (slot) outBlockedSlots.Insert(slot.GetClassName());
					}
				}
				return;
			}
		}
	}

	//! Preserve each cloth's native area, restrictions and model-dependent area changes.
	static string ExtractLoadout(map<string, ref array<BaseContainer>> comps)
	{
		array<string> entries = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseLoadoutClothComponent")) continue;
			foreach (BaseContainer cloth : bucket)
			{
				string path = "/loadout/components/" + entries.Count().ToString();
				string json = TBD_EquipmentExportJson.Fields(cloth, "visible_in_vehicle=VisibleInVehicle", path);
				json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(cloth));
				json = TBD_EquipmentExportJson.Member(json, "area_type", TBD_EquipmentExportJson.Context(cloth.GetObject("AreaType")));
				json = TBD_EquipmentExportJson.Member(json, "blocked_slots", TypeList(cloth, "BlockedSlots"));
				json = TBD_EquipmentExportJson.Member(json, "areas_when_model_changes", TypeList(cloth, "AreaTypeWhenModelChange"));
				entries.Insert(json);
			}
		}
		if (entries.IsEmpty()) return "";
		return "{\"components\":[" + TBD_EquipmentExportJson.Join(entries) + "]}";
	}

	//! Extract each effective storage compartment and its native capacity limits.
	static void ExtractStorage(map<string, ref array<BaseContainer>> comps, TBD_WearableStorageInfo outStorage)
	{
		outStorage.m_sJson = TBD_ItemInventoryExtractor.Storage(comps);
		outStorage.m_bHasStorage = !outStorage.m_sJson.IsEmpty();
	}

	//! Storage remains owned by the standard inventory reader for both selection passes.
	protected static void ExtractStoragePass(map<string, ref array<BaseContainer>> comps, TBD_WearableStorageInfo outStorage, bool universalOnly)
	{
		ExtractStorage(comps, outStorage);
	}

	//! Inventory measurements and physics-body measurements remain separate.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_WearablePhysicalInfo outPhys)
	{
		outPhys.m_sInventoryJson = TBD_ItemInventoryExtractor.Inventory(comps);
		outPhys.m_sPhysicsJson = TBD_ItemInventoryExtractor.Physics(comps);
	}

	//! Preserve native armor managers, hit zones and configured damage propagation rules.
	static void ExtractArmor(map<string, ref array<BaseContainer>> comps, TBD_WearableArmorInfo outArmor)
	{
		array<string> managers = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "SCR_ArmorDamageManagerComponent")) continue;
			foreach (BaseContainer manager : bucket)
			{
				string path = "/armor/managers/" + managers.Count().ToString();
				string json = TBD_EquipmentExportJson.Fields(manager, "passed_damage_scale=m_fPassedDamageScale|detachable=m_bIsDetachable", path);
				json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(manager));
				BaseContainer defaultZone = manager.GetObject("DefaultHitZone");
				if (defaultZone) json = TBD_EquipmentExportJson.Member(json, "default_hit_zone", SerializeArmorHitZone(defaultZone, path + "/default_hit_zone"));
				BaseContainerList zones = manager.GetObjectArray("Additional hit zones");
				string zoneJson = "null";
				if (zones)
				{
					array<string> entries = {};
					for (int h = 0; h < zones.Count(); h++)
					{
						BaseContainer zone = zones.Get(h);
						if (!zone) entries.Insert("null");
						else entries.Insert(SerializeArmorHitZone(zone, path + "/hit_zones/" + h.ToString()));
					}
					zoneJson = "[" + TBD_EquipmentExportJson.Join(entries) + "]";
				}
				managers.Insert(TBD_EquipmentExportJson.Member(json, "hit_zones", zoneJson));
			}
		}
		outArmor.m_bHasArmor = !managers.IsEmpty();
		if (outArmor.m_bHasArmor) outArmor.m_sJson = "{\"managers\":[" + TBD_EquipmentExportJson.Join(managers) + "]}";
	}

	//! Read the armor's authored health, collision coverage and damage multipliers.
	protected static string SerializeArmorHitZone(BaseContainer zone, string path)
	{
		string bindings = "is_default=HZDefault|max_health=MaxHealth|damage_reduction=DamageReduction|damage_threshold=DamageThreshold";
		bindings += "|collider_names=ColliderNames|hit_zone_group=m_eHitZoneGroup|damage_state_thresholds=DamageState threshold";
		string json = TBD_EquipmentExportJson.Fields(zone, bindings, path);
		json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(zone));
		string multipliers = "collision=Collision multiplier|melee=Melee multiplier|kinetic=Kinetic multiplier";
		multipliers += "|fragmentation=Fragmentation multiplier|explosive=Explosive multiplier|incendiary=Incendiary multiplier";
		json = TBD_EquipmentExportJson.Member(json, "damage_multipliers", TBD_EquipmentExportJson.Fields(zone, multipliers, path + "/damage_multipliers"));
		BaseContainerList rules = zone.GetObjectArray("m_aDamagePassRules");
		string ruleJson = "null";
		if (rules)
		{
			array<string> entries = {};
			for (int r = 0; r < rules.Count(); r++)
			{
				BaseContainer rule = rules.Get(r);
				if (!rule) { entries.Insert("null"); continue; }
				string fields = "damage_states=m_aDamageStates|source_damage_types=m_aSourceDamageTypes|output_damage_type=m_eOutputDamageType|multiplier=m_fMultiplier";
				fields += "|pass_to_root=m_bPassToRoot|pass_to_parent=m_bPassToParent|pass_to_default_hit_zone=m_bPassToDefaultHitZone";
				string entry = TBD_EquipmentExportJson.Fields(rule, fields, path + "/damage_pass_rules/" + r.ToString());
				entries.Insert(TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(rule)));
			}
			ruleJson = "[" + TBD_EquipmentExportJson.Join(entries) + "]";
		}
		return TBD_EquipmentExportJson.Member(json, "damage_pass_rules", ruleJson);
	}

	//! Keep every effective loadout slot, its owning cloth and exact native default reference.
	static void ExtractSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_WearableSlotInfo> outSlots)
	{
		outSlots.Clear();
		array<string> ancestry = {};
		ancestry.Insert(TBD_EquipmentComponentGraph.m_CurrentResource);
		ReadLoadoutSlots(comps, outSlots, "/slots", ancestry);
	}

	//! Preserve separate slot installations while following only their gameplay storage relationships.
	protected static void ReadLoadoutSlots(map<string, ref array<BaseContainer>> comps, array<ref TBD_WearableSlotInfo> outSlots, string outputPath, array<string> ancestry)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseLoadoutClothComponent")) continue;
			foreach (BaseContainer cloth : bucket)
			{
				BaseContainerList slots = cloth.GetObjectArray("Slots");
				if (!slots) continue;
				for (int i = 0; i < slots.Count(); i++)
				{
					TBD_WearableSlotInfo info = new TBD_WearableSlotInfo();
					BaseContainer slot = slots.Get(i);
					if (!slot) { info.m_sJson = "null"; outSlots.Insert(info); continue; }
					info.m_sSlotName = slot.GetName();
					string path = outputPath + "/" + outSlots.Count().ToString();
					string bindings = "default_prefab=Prefab|enabled=Enabled|pivot_id=PivotID|child_pivot_id=ChildPivotID|offset=Offset|angles=Angles";
					info.m_sJson = TBD_EquipmentExportJson.Fields(slot, bindings, path);
					info.m_sJson = TBD_EquipmentExportJson.Member(info.m_sJson, "source", TBD_EquipmentExportJson.Context(slot));
					info.m_sJson = TBD_EquipmentExportJson.Member(info.m_sJson, "owning_cloth", TBD_EquipmentExportJson.Context(cloth));
					info.m_sJson = TBD_EquipmentExportJson.Member(info.m_sJson, "slot_index", i.ToString());
					BaseContainer area = slot.GetObject("AreaType");
					info.m_sJson = TBD_EquipmentExportJson.Member(info.m_sJson, "area_type", TBD_EquipmentExportJson.Context(area));
					if (area) info.m_sAreaType = area.GetClassName();
					slot.Get("Prefab", info.m_sDefaultPrefab);
					if (!info.m_sDefaultPrefab.IsEmpty())
						info.m_sJson = TBD_EquipmentExportJson.Member(info.m_sJson, "default_item", ReadInstalledStorage(info.m_sDefaultPrefab, path + "/default_item", ancestry));
					outSlots.Insert(info);
				}
			}
		}
	}

	//! Resolve a pouch's own storage and slots, restoring the host's metadata context on return.
	protected static string ReadInstalledStorage(string resource, string path, array<string> ancestry)
	{
		if (ancestry.Count() >= 32)
			return StorageReferenceFailure(resource, path, "Loadout storage traversal limit");
		Resource loaded = Resource.Load(resource);
		if (!loaded || !loaded.IsValid() || !loaded.GetResource())
			return StorageReferenceFailure(resource, path, "Cannot load configured loadout item");
		BaseContainer root = loaded.GetResource().ToBaseContainer();
		if (!root) return StorageReferenceFailure(resource, path, "Configured loadout item has no source container");
		string identity = root.GetResourceName();
		if (identity.IsEmpty()) identity = resource;
		if (ancestry.Find(identity) >= 0)
			return StorageReferenceFailure(resource, path, "Loadout storage reference cycle");
		TBD_EquipmentComponentContext previous = TBD_EquipmentComponentGraph.SaveContext();
		map<string, ref array<BaseContainer>> components = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, components, 128);
		TBD_EquipmentComponentGraph.m_CurrentResource = previous.m_sResource;
		string json = "{\"resource_name\":" + TBD_EquipmentExportJson.Quote(resource) + "}";
		string storage = TBD_ItemInventoryExtractor.StorageAtPath(components, path + "/storage");
		if (!storage.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "storage", storage);
		ancestry.Insert(identity);
		array<ref TBD_WearableSlotInfo> slots = {};
		ReadLoadoutSlots(components, slots, path + "/slots", ancestry);
		ancestry.Remove(ancestry.Count() - 1);
		array<string> entries = {};
		foreach (TBD_WearableSlotInfo slot : slots) entries.Insert(slot.m_sJson);
		if (!entries.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "slots", "[" + TBD_EquipmentExportJson.Join(entries) + "]");
		TBD_EquipmentComponentGraph.RestoreContext(previous);
		return json;
	}

	//! A failed gameplay relationship remains visible and prevents a successful export result.
	protected static string StorageReferenceFailure(string resource, string path, string reason)
	{
		TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, reason + ": " + resource);
		string json = "{\"resource_name\":" + TBD_EquipmentExportJson.Quote(resource) + "}";
		json = TBD_EquipmentExportJson.Member(json, "status", "\"error\"");
		return TBD_EquipmentExportJson.Member(json, "reason", TBD_EquipmentExportJson.Quote(reason));
	}

	//! Read separate model references and inventory preview resources without normalizing their names.
	static void ExtractVisual(map<string, ref array<BaseContainer>> comps, TBD_WearableVisualInfo outVisual)
	{
		array<string> models = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseLoadoutClothComponent")) continue;
			foreach (BaseContainer cloth : bucket)
			{
				string path = "/visuals/models/" + models.Count().ToString();
				string json = TBD_EquipmentExportJson.Fields(cloth, "worn_model=WornModel|item_model=ItemModel|deflated_model=DeflatedModel", path);
				models.Insert(TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(cloth)));
			}
		}
		string visual = "{\"models\":[" + TBD_EquipmentExportJson.Join(models) + "]}";
		BaseContainer attributes = TBD_ItemInventoryExtractor.Attributes(comps);
		if (attributes)
		{
			BaseContainer ui = attributes.GetObject("ItemDisplayName");
			visual = TBD_EquipmentExportJson.Member(visual, "icon", TBD_EquipmentExportJson.Field(ui, "Icon", "/visuals/icon"));
			BaseContainerList custom = attributes.GetObjectArray("CustomAttributes");
			if (custom)
			{
				array<string> previews = {};
				for (int c = 0; c < custom.Count(); c++)
				{
					BaseContainer entry = custom.Get(c);
					if (!entry || !TBD_EquipmentComponentGraph.IsA(entry.GetClassName(), "PreviewRenderAttributes")) continue;
					string fields = "preview_model=PreviewModel|preview_prefab=PreviewPrefab|preview_worn_model=PreviewWornModel";
					previews.Insert(TBD_EquipmentExportJson.Fields(entry, fields, "/visuals/previews/" + previews.Count().ToString()));
				}
				visual = TBD_EquipmentExportJson.Member(visual, "previews", "[" + TBD_EquipmentExportJson.Join(previews) + "]");
			}
		}
		outVisual.m_sJson = visual;
	}

	//! Preserve an ordered native loadout-type list, including explicit null entries.
	protected static string TypeList(BaseContainer owner, string property)
	{
		BaseContainerList list = owner.GetObjectArray(property);
		if (!list) return "null";
		array<string> entries = {};
		for (int i = 0; i < list.Count(); i++) entries.Insert(TBD_EquipmentExportJson.Context(list.Get(i)));
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}
}
