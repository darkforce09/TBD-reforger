/**
 * TBD_VehicleArmorExtractor.c
 *
 * Dedicated extractor for vehicle armor hit zones, health budgets, armor plate thickness,
 * damage multipliers, collision thresholds, and secondary explosion configurations.
 * Pure dynamic reflection over SCR_WheeledDamageManagerComponent, SCR_ArmorDamageManagerComponent, etc.
 */

class TBD_VehicleArmorExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspect hit zones, armor thickness, and damage thresholds for a vehicle variant.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		array<string> managers = {};
		BaseContainer physicsOwnerSource;
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "DamageManagerComponent")) continue;
			foreach (BaseContainer manager : bucket)
			{
				if (!physicsOwnerSource) physicsOwnerSource = TBD_EquipmentComponentGraph.OwningEntitySource(manager);
				string path = "/damage/managers/" + managers.Count().ToString();
				string json = TBD_EquipmentExportJson.Context(manager);
				json = TBD_EquipmentExportJson.Member(json, "default_hit_zone", SerializeConfiguredHitZone(manager.GetObject("DefaultHitZone"), path + "/default_hit_zone"));
				array<string> zones = {};
				BaseContainerList additional = manager.GetObjectArray("Additional hit zones");
				if (additional)
					for (int i = 0; i < additional.Count(); i++)
						zones.Insert(SerializeConfiguredHitZone(additional.Get(i), path + "/hit_zones/" + i.ToString()));
				json = TBD_EquipmentExportJson.Member(json, "hit_zones", "[" + TBD_EquipmentExportJson.Join(zones) + "]");
				string bindings = "collision_velocity_threshold=CollisionVelocityThreshold|heavy_damage_threshold=Heavy damage threshold";
				bindings += "|occupants_damage_speed_threshold=m_fOccupantsDamageSpeedThreshold|occupants_speed_death=m_fOccupantsSpeedDeath";
				bindings += "|vehicle_destroy_damage=m_fVehicleDestroyDamage|vehicle_damage_speed_threshold=m_fVehicleDamageSpeedThreshold";
				bindings += "|vehicle_speed_destroy=m_fVehicleSpeedDestroy|engine_malfunctioning_threshold=m_fEngineMalfunctioningThreshold";
				json = TBD_EquipmentExportJson.Member(json, "thresholds", TBD_EquipmentExportJson.Fields(manager, bindings, path + "/thresholds"));
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_VehicleDamageManagerComponent"))
				{
					string collision = "maximum_shared_damage_distance=m_fMaxSharedDamageDistance|frontal_impact_position=m_vFrontalImpact";
					collision += "|front_multiplier=m_fFrontMultiplier|bottom_multiplier=m_fBottomMultiplier|rear_multiplier=m_fRearMultiplier";
					collision += "|left_multiplier=m_fLeftMultiplier|right_multiplier=m_fRightMultiplier|top_multiplier=m_fTopMultiplier";
					json = TBD_EquipmentExportJson.Member(json, "collision", TBD_EquipmentExportJson.Fields(manager, collision, path + "/collision"));
					string ejection = "minimum_explosion_damage=m_iMinExplosionEjectionDamageThreshold|explosion_chance=m_iExplosionDamageEjectionChance";
					ejection += "|minimum_collision_damage=m_iMinCollisionEjectionDamageThreshold|collision_chance=m_iCollisionDamageEjectionChance";
					json = TBD_EquipmentExportJson.Member(json, "occupant_ejection", TBD_EquipmentExportJson.Fields(manager, ejection, path + "/occupant_ejection"));
					string fire = "fuel_tank_damage_rate=m_fFuelTankFireDamageRate|supplies_damage_rate=m_fSuppliesFireDamageRate|damage_delay=m_fSecondaryFireDamageDelay";
					json = TBD_EquipmentExportJson.Member(json, "secondary_fire", TBD_EquipmentExportJson.Fields(manager, fire, path + "/secondary_fire"));
				}
				managers.Insert(json);
			}
		}
		if (!managers.IsEmpty())
		{
			varData.m_sDamageJson = "{\"managers\":[" + TBD_EquipmentExportJson.Join(managers) + "]}";
			varData.m_sDamageJson = TBD_EquipmentExportJson.Member(varData.m_sDamageJson, "geometry", ReadGeometryReferences(comps));
			varData.m_sDamageJson = TBD_EquipmentExportJson.Member(varData.m_sDamageJson, "collision_materials", ReadCollisionMaterials(TBD_EquipmentComponentGraph.m_PrefabResource, "/damage/collision_materials", physicsOwnerSource));
		}
	}

	//! Inspect the initialized native collision surfaces; material thickness is never labeled vehicle armor thickness.
	static string ReadCollisionMaterials(string resourceName, string path, BaseContainer ownerSource)
	{
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		Resource resource = Resource.Load(resourceName);
		if (!editor || !editor.GetApi() || !GetGame() || !resource || !resource.IsValid())
		{
			TBD_EquipmentExportJson.ExtractionError(resourceName, path, "Cannot initialize material inspection");
			return "null";
		}
		IEntity entity = GetGame().SpawnEntityPrefab(resource, editor.GetApi().GetWorld());
		if (!entity)
		{
			TBD_EquipmentExportJson.ExtractionError(resourceName, path, "Cannot spawn material inspection prefab");
			return "null";
		}
		array<string> geometries = {};
		array<string> materials = {};
		array<SurfaceProperties> seenMaterials = {};
		IEntity physicsOwner = entity;
		if (ownerSource && ownerSource != resource.GetResource().ToBaseContainer())
		{
			array<IEntity> matchingOwners = {};
			FindPhysicsOwners(entity, ownerSource, matchingOwners);
			physicsOwner = null;
			if (matchingOwners.Count() == 1) physicsOwner = matchingOwners[0];
			else TBD_EquipmentExportJson.ExtractionError(resourceName, path, "Cannot uniquely match configured collision-body owner");
		}
		Physics physics;
		if (physicsOwner) physics = physicsOwner.GetPhysics();
		if (physics)
		{
			for (int geometryIndex = 0; geometryIndex < physics.GetNumGeoms(); geometryIndex++)
			{
				array<SurfaceProperties> surfaces = {};
				physics.GetGeomSurfaces(geometryIndex, surfaces);
				array<string> indices = {};
				foreach (SurfaceProperties surface : surfaces)
				{
					if (!surface) { indices.Insert("null"); continue; }
					int materialIndex = seenMaterials.Find(surface);
					if (materialIndex < 0)
					{
						materialIndex = seenMaterials.Count();
						seenMaterials.Insert(surface);
						materials.Insert(ReadMaterialSurface(surface, path + "/materials/" + materialIndex.ToString()));
					}
					indices.Insert(materialIndex.ToString());
				}
				string geometry = "{\"native_index\":" + geometryIndex.ToString();
				geometry += ",\"collider_name\":" + TBD_EquipmentExportJson.Quote(physics.GetGeomName(geometryIndex));
				geometry += ",\"materials\":[" + TBD_EquipmentExportJson.Join(indices) + "]}";
				geometries.Insert(geometry);
			}
		}
		SCR_EntityHelper.DeleteEntityAndChildren(entity);
		return "{\"geometries\":[" + TBD_EquipmentExportJson.Join(geometries) + "],\"materials\":[" + TBD_EquipmentExportJson.Join(materials) + "]}";
	}

	//! Match preview ownership to the actual source entity, never to an arbitrary child index.
	protected static void FindPhysicsOwners(IEntity entity, BaseContainer ownerSource, array<IEntity> matches)
	{
		if (!entity) return;
		EntityPrefabData prefab = entity.GetPrefabData();
		if (prefab && prefab.GetPrefab())
		{
			BaseContainer source = prefab.GetPrefab();
			if (source == ownerSource || source.GetResourceName() == ownerSource.GetResourceName()) matches.Insert(entity);
		}
		IEntity child = entity.GetChildren();
		while (child)
		{
			FindPhysicsOwners(child, ownerSource, matches);
			child = child.GetSibling();
		}
	}

	//! Surface resource identity and native ballistic getters retain their original values and unspecified units.
	protected static string ReadMaterialSurface(SurfaceProperties surface, string path)
	{
		string json = TBD_EquipmentExportJson.Context(surface);
		BaseContainer configuration = surface.GetObject("BallisticInfo");
		if (configuration)
		{
			string bindings = "thickness_override=ThicknessOverride|stops_explosion_trace=StopExplosionTrace";
			bindings += "|kinetic_penetration_resistance=KineticPenetrationResistance|chemical_penetration_resistance=ChemicalPenetrationResistance";
			bindings += "|penetration_direction_distribution=PenetrationDirDistribution|penetration_direction_combine=PenetrationDirCombine";
			bindings += "|cavitation_damage_multiplier=CavitationDamageMultiplier|mushrooming_damage_multiplier=MushroomingDamageMultiplier";
			bindings += "|tumbling_damage_multiplier=TumblingDamageMultiplier|glass_surface=GlassSurface|penetration_damage_model=PenetrationDamageModel";
			json = TBD_EquipmentExportJson.Member(json, "configuration", TBD_EquipmentExportJson.Fields(configuration, bindings, path + "/configuration"));
		}
		GameMaterial material = surface;
		if (!material || !material.GetBallisticInfo())
			return TBD_EquipmentExportJson.Member(json, "ballistics", "null");
		BallisticInfo ballistics = material.GetBallisticInfo();
		string values = "{\"density\":" + TBD_EquipmentExportJson.Number(ballistics.GetDensity());
		values += ",\"thickness\":" + TBD_EquipmentExportJson.Number(ballistics.GetThickness());
		values += ",\"maximum_thickness\":" + TBD_EquipmentExportJson.Number(ballistics.GetThicknessMax());
		string deflection = "false";
		if (ballistics.AllowsDeflection()) deflection = "true";
		string water = "false";
		if (ballistics.IsWaterSurface()) water = "true";
		values += ",\"allows_deflection\":" + deflection + ",\"water_surface\":" + water + "}";
		array<string> fields = {"density", "thickness", "maximum_thickness", "allows_deflection", "water_surface"};
		array<string> methods = {"GetDensity", "GetThickness", "GetThicknessMax", "AllowsDeflection", "IsWaterSurface"};
		for (int i = 0; i < fields.Count(); i++)
		{
			string nativeType = "SCALAR";
			if (i >= 3) nativeType = "BOOLEAN";
			TBD_EquipmentExportJson.RecordNativeField(surface, path + "/ballistics/" + fields[i], "present", "BallisticInfo." + methods[i], "Physics.GetGeomSurfaces -> GameMaterial.GetBallisticInfo on a temporary initialized prefab; no gameplay simulation", "", nativeType);
		}
		return TBD_EquipmentExportJson.Member(json, "ballistics", values);
	}

	//! Keep authored collision-model references associated with their source component instances.
	protected static string ReadGeometryReferences(map<string, ref array<BaseContainer>> comps)
	{
		array<string> geometry = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "MeshObject")) continue;
			foreach (BaseContainer component : bucket)
			{
				string path = "/damage/geometry/" + geometry.Count().ToString();
				string json = TBD_EquipmentExportJson.Fields(component, "model=Object", path);
				json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(component));
				geometry.Insert(json);
			}
		}
		return "[" + TBD_EquipmentExportJson.Join(geometry) + "]";
	}

	//! Preserve hit-zone health, collider identity and ordered damage propagation rules.
	static string SerializeConfiguredHitZone(BaseContainer source, string path)
	{
		if (!source) return "null";
		string bindings = "max_health=MaxHealth|colliders=ColliderNames|group=m_eHitZoneGroup|damage_state_thresholds=DamageState threshold";
		bindings += "|collision_multiplier=Collision multiplier|melee_multiplier=Melee multiplier|kinetic_multiplier=Kinetic multiplier";
		bindings += "|fragmentation_multiplier=Fragmentation multiplier|explosive_multiplier=Explosive multiplier|incendiary_multiplier=Incendiary multiplier";
		string json = TBD_EquipmentExportJson.Fields(source, bindings, path);
		json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(source));
		string rules = "damage_states=m_aDamageStates|source_damage_types=m_aSourceDamageTypes|output_damage_type=m_eOutputDamageType";
		rules += "|multiplier=m_fMultiplier|pass_to_default=m_bPassToDefaultHitZone|pass_to_root=m_bPassToRoot|pass_to_parent=m_bPassToParent";
		json = TBD_EquipmentExportJson.Member(json, "damage_pass_rules", TBD_EquipmentExportJson.ObjectArray(source, "m_aDamagePassRules", rules, path + "/damage_pass_rules"));
		return json;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract damage multipliers, collision limits, and secondary blast presets from DamageManager.
	protected static void ExtractThresholds(BaseContainer dm, TBD_VehicleDamageThresholds thresholds)
	{
		if (thresholds.m_fVehicleDestroyDamage <= 0)
		{
			dm.Get("m_fVehicleDestroyDamage", thresholds.m_fVehicleDestroyDamage);
			if (thresholds.m_fVehicleDestroyDamage <= 0)
				dm.Get("VehicleDestroyDamage", thresholds.m_fVehicleDestroyDamage);
		}

		if (thresholds.m_fCollisionVelocityThreshold <= 0)
		{
			dm.Get("CollisionVelocityThreshold", thresholds.m_fCollisionVelocityThreshold);
			if (thresholds.m_fCollisionVelocityThreshold <= 0)
				dm.Get("m_fCollisionVelocityThreshold", thresholds.m_fCollisionVelocityThreshold);
		}

		if (thresholds.m_fHeavyDamageThreshold <= 0)
		{
			dm.Get("Heavy damage threshold", thresholds.m_fHeavyDamageThreshold);
			if (thresholds.m_fHeavyDamageThreshold <= 0)
				dm.Get("HeavyDamageThreshold", thresholds.m_fHeavyDamageThreshold);
		}

		if (thresholds.m_fOccupantsDamageSpeedThreshold <= 0)
			dm.Get("m_fOccupantsDamageSpeedThreshold", thresholds.m_fOccupantsDamageSpeedThreshold);

		if (thresholds.m_fOccupantsSpeedDeath <= 0)
			dm.Get("m_fOccupantsSpeedDeath", thresholds.m_fOccupantsSpeedDeath);

		// Directional multipliers
		float fMult = 1, rMult = 1, lMult = 1, riMult = 1, tMult = 1, bMult = 1;
		if (dm.Get("m_fFrontMultiplier", fMult)) thresholds.m_fFrontMultiplier = fMult;
		if (dm.Get("m_fRearMultiplier", rMult)) thresholds.m_fRearMultiplier = rMult;
		if (dm.Get("m_fLeftMultiplier", lMult)) thresholds.m_fLeftMultiplier = lMult;
		if (dm.Get("m_fRightMultiplier", riMult)) thresholds.m_fRightMultiplier = riMult;
		if (dm.Get("m_fTopMultiplier", tMult)) thresholds.m_fTopMultiplier = tMult;
		if (dm.Get("m_fBottomMultiplier", bMult)) thresholds.m_fBottomMultiplier = bMult;

		// Secondary blast presets
		if (thresholds.m_sSecondaryExplosionsConf.IsEmpty())
		{
			string secExpl;
			if (dm.Get("m_SecondaryExplosions", secExpl) && !secExpl.IsEmpty())
				thresholds.m_sSecondaryExplosionsConf = ExtractConfPath(secExpl);
		}

		if (thresholds.m_sSecondaryFiresConf.IsEmpty())
		{
			string secFire;
			if (dm.Get("m_SecondaryFires", secFire) && !secFire.IsEmpty())
				thresholds.m_sSecondaryFiresConf = ExtractConfPath(secFire);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Iterate a container array and extract individual hit zones without duplicate registration.
	protected static void ExtractHitZoneList(BaseContainerList list, array<ref TBD_VehicleHitZoneData> outZones, array<string> recorded)
	{
		if (!list) return;

		for (int i = 0, n = list.Count(); i < n; i++)
		{
			ExtractSingleHitZone(list.Get(i), outZones, recorded);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract name, group category, max health, and armor thickness from a hit zone container.
	protected static void ExtractSingleHitZone(BaseContainer hz, array<ref TBD_VehicleHitZoneData> outZones, array<string> recorded)
	{
		if (!hz) return;

		string name;
		if (hz.Get("m_sHitZoneName", name) && !name.IsEmpty())
		{
		}
		else if (hz.Get("m_sName", name) && !name.IsEmpty())
		{
		}
		else if (hz.Get("HitZoneName", name) && !name.IsEmpty())
		{
		}
		else
		{
			name = hz.GetName();
		}

		if (recorded.Find(name) != -1)
			return;

		TBD_VehicleHitZoneData hd = new TBD_VehicleHitZoneData();
		hd.m_sName = name;
		recorded.Insert(name);

		// Group classification
		string group;
		if (hz.Get("m_eHitZoneGroup", group) && !group.IsEmpty())
			hd.m_sGroup = group;
		else if (hz.Get("HitZoneGroup", group) && !group.IsEmpty())
			hd.m_sGroup = group;
		else
			hd.m_sGroup = string.Empty;

		// Health budget
		float hp = 0;
		if (hz.Get("m_fMaxHealth", hp) && hp > 0)
			hd.m_fMaxHealth = hp;
		else if (hz.Get("MaxHealth", hp) && hp > 0)
			hd.m_fMaxHealth = hp;

		// Armor plate thickness (mm)
		float armor = 0;
		if (hz.Get("m_fArmorThickness", armor) && armor > 0)
			hd.m_fArmorThickness = armor;
		else if (hz.Get("ArmorThickness", armor) && armor > 0)
			hd.m_fArmorThickness = armor;

		// Damage multiplier
		float dmgMult = 1.0;
		if (hz.Get("m_fDamageMultiplier", dmgMult) && dmgMult > 0)
			hd.m_fDamageMultiplier = dmgMult;
		else if (hz.Get("DamageMultiplier", dmgMult) && dmgMult > 0)
			hd.m_fDamageMultiplier = dmgMult;

		outZones.Insert(hd);
	}

	//------------------------------------------------------------------------------------------------
	//! Derive tactical hit zone group category from name when not explicitly serialized.
	protected static string ClassifyGroupByStem(string name)
	{
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract configuration file path from Enfusion resource reference string.
	protected static string ExtractConfPath(string s)
	{
		int quote1 = s.IndexOf("\"");
		if (quote1 == -1) return s;
		int quote2 = s.IndexOfFrom(quote1 + 1, "\"");
		if (quote2 == -1) return s;
		return s.Substring(quote1 + 1, quote2 - quote1 - 1);
	}
}
