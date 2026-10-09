/**
 * Static and crew-served weapon extraction retains individual installations.
 * Weapon capabilities use the same domain readers as carried weapons.
 */
//! Read static weapons without substituting family specifications or destruction debris.
class TBD_StaticWeaponExtractor
{
	//! Extract effective static systems, native weapon capabilities and source measurements.
	static void ExtractSpecs(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		ExtractTurret(comps, info);
		ExtractCrew(comps, info);
		ExtractArmament(comps, info);
		ExtractDeployment(comps, info);
		ExtractPhysical(comps, info);
		info.m_sVisualsJson = TBD_ItemExtractor.ExtractVisuals(comps);
		ExtractFaction(comps, info);
		array<string> configurations = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "SCR_BaseHUDComponent")) continue;
			foreach (BaseContainer hud : bucket)
			{
				array<BaseContainer> active = {};
				ExtractBallisticConfigurations(hud, configurations, active, 0);
			}
		}
		if (!configurations.IsEmpty()) info.m_sBallisticConfigurationsJson = "[" + TBD_EquipmentExportJson.Join(configurations) + "]";
	}

	//! Keep separate native turret mechanics and controller limits; units are not inferred.
	protected static void ExtractTurret(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		array<string> turrets = {};
		array<string> controllers = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer comp : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "TurretComponent"))
				{
					string path = "/turret/installations/" + turrets.Count().ToString();
					string fields = "aiming_max_speed=AimingMaxSpeed|aiming_speed_controls=AimingSpeedControls";
					fields += "|speed=TurretSpeed|acceleration=TurretAcceleration|deceleration=TurretDeceleration|aiming_type=ProvideAimingType";
					turrets.Insert(Describe(comp, fields, path));
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "TurretControllerComponent"))
				{
					string controllerPath = "/turret/controllers/" + controllers.Count().ToString();
					string controllerFields = "horizontal_limits=LimitsHoriz|vertical_limits=LimitsVert|reload_position=TurretReloadPosition";
					controllerFields += "|return_before_reload=ReturnToPositionBeforeReload|aim_only_in_ads=CanAimOnlyInADS|moveable_base=HasMoveableBase";
					controllers.Insert(Describe(comp, controllerFields, controllerPath));
				}
			}
		}
		TBD_WeaponExtractor.ExtractSights(comps, info.m_Sights);
		info.m_bHasTurret = !turrets.IsEmpty() || !controllers.IsEmpty();
		if (!info.m_bHasTurret) return;
		info.m_sTurretJson = "{\"installations\":[" + TBD_EquipmentExportJson.Join(turrets) + "]";
		info.m_sTurretJson += ",\"controllers\":[" + TBD_EquipmentExportJson.Join(controllers) + "]}";
	}

	//! Reuse the standard sight reader for each effective static sight installation.
	protected static void ExtractSightsComponent(BaseContainer sight, TBD_StaticWeaponInfo info)
	{
		map<string, ref array<BaseContainer>> components = new map<string, ref array<BaseContainer>>();
		array<BaseContainer> instances = {sight};
		components.Insert(sight.GetClassName(), instances);
		TBD_WeaponExtractor.ExtractSights(components, info.m_Sights);
	}

	//! Preserve all native crew slots and their door, position and occupant relationships.
	protected static void ExtractCrew(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		array<string> managers = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseCompartmentManagerComponent")) continue;
			foreach (BaseContainer manager : bucket)
			{
				string managerPath = "/crew/managers/" + managers.Count().ToString();
				string data = TBD_EquipmentExportJson.Context(manager);
				BaseContainerList slots = manager.GetObjectArray("CompartmentSlots");
				if (slots)
				{
					array<string> entries = {};
					for (int i = 0; i < slots.Count(); i++)
					{
						BaseContainer slot = slots.Get(i);
						if (!slot) { entries.Insert("null"); continue; }
						string path = managerPath + "/stations/" + i.ToString();
						string fields = "seat_type=SeatType|door_indices=DoorInfoList|restricted_item_types=RestrictedItemTypes|allow_aiming=AllowAiming";
						string entry = Describe(slot, fields, path);
						entry = TBD_EquipmentExportJson.Member(entry, "passenger_position", Point(slot.GetObject("PassengerPositionInfo"), path + "/passenger_position"));
						BaseContainer occupant = slot.GetObject("m_DefaultOccupantData");
						if (occupant) entry = TBD_EquipmentExportJson.Member(entry, "default_occupant", Describe(occupant, "prefab=m_sDefaultOccupantPrefab|enabled=m_bEnabled", path + "/default_occupant"));
						entries.Insert(entry);
					}
					data = TBD_EquipmentExportJson.Member(data, "stations", "[" + TBD_EquipmentExportJson.Join(entries) + "]");
					info.m_bHasCrewSlot = info.m_bHasCrewSlot || !entries.IsEmpty();
				}
				BaseContainerList doors = manager.GetObjectArray("DoorInfoList");
				if (doors)
				{
					array<string> doorEntries = {};
					for (int d = 0; d < doors.Count(); d++)
					{
						BaseContainer door = doors.Get(d);
						if (!door) { doorEntries.Insert("null"); continue; }
						string doorPath = managerPath + "/doors/" + d.ToString();
						string doorJson = Describe(door, "fake_door=FakeDoor|get_in_teleport=GetInTeleport|get_out_teleport=GetOutTeleport", doorPath);
						doorJson = TBD_EquipmentExportJson.Member(doorJson, "entry_position", Point(door.GetObject("EntryPositionInfo"), doorPath + "/entry_position"));
						doorJson = TBD_EquipmentExportJson.Member(doorJson, "exit_position", Point(door.GetObject("ExitPositionInfo"), doorPath + "/exit_position"));
						doorEntries.Insert(doorJson);
					}
					data = TBD_EquipmentExportJson.Member(data, "doors", "[" + TBD_EquipmentExportJson.Join(doorEntries) + "]");
				}
				managers.Insert(data);
			}
		}
		if (!managers.IsEmpty()) info.m_sCrewJson = "{\"managers\":[" + TBD_EquipmentExportJson.Join(managers) + "]}";
	}

	//! Use the standard weapon readers; native defaults remain separate from compatibility rules.
	protected static void ExtractArmament(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		TBD_WeaponMuzzleExtractor.ExtractMuzzles(comps, info.m_aMuzzles);
		TBD_WeaponMountingExtractor.ExtractAttachmentSlots(comps, info.m_aAttachmentSlots);
		info.m_bIsIntegral = !info.m_aMuzzles.IsEmpty();
		array<string> slots = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "WeaponSlotComponent")) continue;
			foreach (BaseContainer slot : bucket)
			{
				string path = "/armament/weapon_slots/" + slots.Count().ToString();
				slots.Insert(Describe(slot, "weapon_template=WeaponTemplate|enabled=Enabled", path));
				string resource;
				bool enabled;
				bool hasEnabled = slot.Get("Enabled", enabled);
				if (slot.Get("WeaponTemplate", resource) && !resource.IsEmpty() && (!hasEnabled || enabled))
				{
					info.m_bHasArmament = true;
					if (info.m_sMountedWeaponTemplate.IsEmpty()) info.m_sMountedWeaponTemplate = resource;
					ReadMountedWeapon(slot, resource, info);
				}
			}
		}
		info.m_bHasArmament = info.m_bHasArmament || info.m_bIsIntegral;
		if (!slots.IsEmpty()) info.m_sArmamentJson = "{\"weapon_slots\":[" + TBD_EquipmentExportJson.Join(slots) + "]}";
	}

	//! Read an enabled mounted weapon with the same readers and retain its source resource.
	protected static void ReadMountedWeapon(BaseContainer installation, ResourceName resourceName, TBD_StaticWeaponInfo info)
	{
		string path = "/mounted_weapons/" + info.m_aMountedWeaponsJson.Count().ToString();
		string json = "{\"resource_name\":" + TBD_EquipmentExportJson.Quote(resourceName) + "}";
		json = TBD_EquipmentExportJson.Member(json, "installation", TBD_EquipmentExportJson.Context(installation));
		Resource resource = Resource.Load(resourceName);
		if (!resource || !resource.IsValid())
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Cannot load mounted weapon " + resourceName);
			info.m_aMountedWeaponsJson.Insert(TBD_EquipmentExportJson.Member(json, "status", "\"error\""));
			return;
		}
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Mounted weapon has no container " + resourceName);
			info.m_aMountedWeaponsJson.Insert(TBD_EquipmentExportJson.Member(json, "status", "\"error\""));
			return;
		}
		TBD_EquipmentComponentContext savedContext = TBD_EquipmentComponentGraph.SaveContext();
		string savedPrefix = TBD_EquipmentExportJson.m_sFieldPrefix;
		map<string, ref array<BaseContainer>> components = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, components, 128);
		TBD_EquipmentComponentGraph.m_CurrentResource = savedContext.m_sResource;
		TBD_EquipmentExportJson.m_sFieldPrefix = savedPrefix + path;
		array<ref TBD_WeaponMuzzleInfo> muzzles = {};
		TBD_WeaponMuzzleExtractor.ExtractMuzzles(components, muzzles);
		array<string> muzzleJson = {};
		foreach (TBD_WeaponMuzzleInfo muzzle : muzzles) muzzleJson.Insert(muzzle.m_sSourceJson);
		json = TBD_EquipmentExportJson.Member(json, "muzzles", "[" + TBD_EquipmentExportJson.Join(muzzleJson) + "]");
		TBD_WeaponSightsInfo sights = new TBD_WeaponSightsInfo();
		TBD_WeaponExtractor.ExtractSights(components, sights);
		json = TBD_EquipmentExportJson.Member(json, "sights", "[" + TBD_EquipmentExportJson.Join(sights.m_aInstances) + "]");
		array<ref TBD_WeaponAttachmentSlotInfo> slots = {};
		TBD_WeaponMountingExtractor.ExtractAttachmentSlots(components, slots);
		array<string> slotJson = {};
		foreach (TBD_WeaponAttachmentSlotInfo slot : slots) slotJson.Insert(slot.m_sSourceJson);
		json = TBD_EquipmentExportJson.Member(json, "attachment_slots", "[" + TBD_EquipmentExportJson.Join(slotJson) + "]");
		string deployment = TBD_WeaponExtractor.ExtractDeployment(components);
		if (!deployment.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "deployment", deployment);
		json = TBD_EquipmentExportJson.Member(json, "visuals", TBD_ItemExtractor.ExtractVisuals(components));
		TBD_EquipmentExportJson.m_sFieldPrefix = savedPrefix;
		TBD_EquipmentComponentGraph.RestoreContext(savedContext);
		info.m_aMountedWeaponsJson.Insert(json);
	}

	//! Preserve all configured deployment variants and exact part quantities.
	protected static void ExtractDeployment(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		array<string> parts = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "SCR_MultiPartDeployableItemComponent")) continue;
			foreach (BaseContainer comp : bucket)
			{
				string path = "/deployment/components/" + parts.Count().ToString();
				string data = Describe(comp, "replacement_prefab=m_sReplacementPrefab|surface_observation=m_eSurfaceObservationBehaviour|delete_on_deployment=m_bDeleteThisPartOnDeployment", path);
				BaseContainerList variants = comp.GetObjectArray("m_aVariants");
				if (variants)
				{
					array<string> entries = {};
					for (int v = 0; v < variants.Count(); v++)
					{
						BaseContainer variant = variants.Get(v);
						if (!variant) { entries.Insert("null"); continue; }
						string variantPath = path + "/variants/" + v.ToString();
						string entry = Describe(variant, "variant_id=m_iVariantId|replacement_prefab=m_sReplacementPrefab|use_part_transform=m_bUsePartRotationAndPosition", variantPath);
						entry = TBD_EquipmentExportJson.Member(entry, "additional_parts", TBD_EquipmentExportJson.ObjectArray(variant, "m_aAdditionalPrefabs", "prefab=m_sPrefab|quantity=m_iNumberOfPrefabs", variantPath + "/additional_parts"));
						string requiredFields = "prefab=m_sPrefab|quantity=m_iNumberOfRequiredPrefabs|delete_on_deployment=m_bDeletePartsOnDeployment|detach_magazines=m_bDetachMagazinesWhenUsed";
						entry = TBD_EquipmentExportJson.Member(entry, "required_parts", TBD_EquipmentExportJson.ObjectArray(variant, "m_aRequiredElements", requiredFields, variantPath + "/required_parts"));
						entries.Insert(entry);
					}
					data = TBD_EquipmentExportJson.Member(data, "variants", "[" + TBD_EquipmentExportJson.Join(entries) + "]");
				}
				parts.Insert(data);
			}
		}
		info.m_bIsDeployable = !parts.IsEmpty();
		if (info.m_bIsDeployable) info.m_sDeploymentJson = "{\"components\":[" + TBD_EquipmentExportJson.Join(parts) + "]}";
		string support = TBD_WeaponExtractor.ExtractDeployment(comps, "/deployment/weapon_support");
		if (!support.IsEmpty())
		{
			if (info.m_sDeploymentJson.IsEmpty()) info.m_sDeploymentJson = "{}";
			info.m_sDeploymentJson = TBD_EquipmentExportJson.Member(info.m_sDeploymentJson, "weapon_support", support);
		}
	}

	//! Read inventory and body mass independently, preserving source-authored zero.
	protected static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		info.m_sInventoryJson = TBD_ItemInventoryExtractor.Inventory(comps);
		info.m_sPhysicsJson = TBD_ItemInventoryExtractor.Physics(comps);
		info.m_sStorageJson = TBD_ItemInventoryExtractor.Storage(comps);
	}

	//! Preserve an authored faction affiliation without inferring one from the resource path.
	protected static void ExtractFaction(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "FactionAffiliationComponent")) continue;
			foreach (BaseContainer affiliation : bucket)
			{
				if (affiliation.GetVarIndex("faction affiliation") < 0) continue;
				info.m_sFactionJson = TBD_EquipmentExportJson.Field(affiliation, "faction affiliation", "/faction");
				return;
			}
		}
	}

	//! Traverse only HUD display ownership to reach authored ballistic configurations.
	protected static void ExtractBallisticConfigurations(BaseContainer owner, array<string> entries, array<BaseContainer> active, int depth)
	{
		if (!owner) return;
		if (depth > 64 || active.Find(owner) >= 0)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, "/ballistic_configurations", "Ballistic display cycle or traversal limit");
			return;
		}
		active.Insert(owner);
		if (TBD_EquipmentComponentGraph.IsA(owner.GetClassName(), "SCR_BallisticTableDisplay"))
		{
			BaseContainerList configurations = owner.GetObjectArray("m_aBallisticConfigs");
			if (configurations)
			{
				for (int c = 0; c < configurations.Count(); c++)
				{
					BaseContainer config = configurations.Get(c);
					if (!config) { entries.Insert("null"); continue; }
					string path = "/ballistic_configurations/" + entries.Count().ToString();
					string fields = "projectile_prefab=m_sProjectilePrefab|displayed_text=m_sDisplayedText|range_step=m_iRangeStep";
					fields += "|projectile_initial_speed_coefficient=m_fProjectileInitSpeedCoef|standard_dispersion=m_fStandardDispersion";
					fields += "|unit_type=m_sUnitType|direct_fire=m_bDirectFireMode|minimum_fire_angle=m_fMinFireAngle|maximum_fire_angle=m_fMaxFireAngle";
					string json = Describe(config, fields, path);
					BaseContainer ancestor = config.GetAncestor();
					if (ancestor) json = TBD_EquipmentExportJson.Member(json, "parent_configuration", TBD_EquipmentExportJson.Quote(ancestor.GetResourceName()));
					entries.Insert(json);
				}
			}
		}
		BaseContainerList displays = owner.GetObjectArray("InfoDisplays");
		if (displays)
			for (int d = 0; d < displays.Count(); d++) ExtractBallisticConfigurations(displays.Get(d), entries, active, depth + 1);
		active.Remove(active.Count() - 1);
	}

	//! Read point transforms as native numeric arrays.
	protected static string Point(BaseContainer source, string path)
	{
		if (!source) return "null";
		return Describe(source, "pivot_id=PivotID|offset=Offset|angles=Angles", path);
	}

	//! Keep one source context per domain object, with ordinary values beside it.
	protected static string Describe(BaseContainer source, string fields, string path)
	{
		return TBD_EquipmentExportJson.Member(TBD_EquipmentExportJson.Fields(source, fields, path), "source", TBD_EquipmentExportJson.Context(source));
	}
}
