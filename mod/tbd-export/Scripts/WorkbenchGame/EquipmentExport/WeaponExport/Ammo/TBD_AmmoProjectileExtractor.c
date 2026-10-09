/**
 * Reads native projectile motion, penetration, trigger and ordered damage
 * configurations into the standard ammunition catalog.
 */

//! Preserves projectile systems and their source relationships without universal damage ratings.
class TBD_AmmoProjectileExtractor
{
	protected static ref array<BaseContainer> m_ActiveEffects = {};
	protected static ref array<string> m_ActiveResources = {};

	//! Read effective first-motion measurements for existing scanner consumers.
	static void ExtractBallistics(map<string, ref array<BaseContainer>> comps, TBD_ProjectileBallisticsInfo outBallistics)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMotion(cls)) continue;
			foreach (BaseContainer motion : bucket)
			{
				motion.Get("InitSpeed", outBallistics.m_fInitSpeedMps);
				motion.Get("InitSpeedVariation", outBallistics.m_fInitSpeedVariation);
				motion.Get("AirDrag", outBallistics.m_fAirDrag);
				motion.Get("Mass", outBallistics.m_fMassKg);
				motion.Get("BallisticTableConfig", outBallistics.m_sBallisticTableConfig);
				return;
			}
		}
	}

	//! Preserve the configured safety distance without replacing damage structures with a scalar.
	static void ExtractWarhead(map<string, ref array<BaseContainer>> comps, TBD_ProjectileWarheadInfo outWarhead)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseTriggerComponent")) continue;
			foreach (BaseContainer trigger : bucket)
				if (trigger.Get("SafetyDistance", outWarhead.m_fSafetyDistanceMeters)) return;
		}
	}

	//! Preserve effective projectile and cartridge model references, including empty overrides.
	static void ExtractVisuals(BaseContainer root, TBD_ProjectileVisualsInfo outVisuals)
	{
		if (!root) return;
		root.Get("ProjectileModel", outVisuals.m_sProjectileModel);
		root.Get("CartridgeModel", outVisuals.m_sCartridgeModel);
	}

	//! Build domain sections from selected native fields and ordered gameplay effects.
	static string ExtractConfiguration(BaseContainer root, map<string, ref array<BaseContainer>> comps, string path = "/projectile")
	{
		m_ActiveEffects.Clear();
		m_ActiveResources.Clear();
		return ReadConfiguredSystems(root, comps, path, 0);
	}

	//! Read each configured system without resetting the active referenced-resource chain.
	protected static string ReadConfiguredSystems(BaseContainer root, map<string, ref array<BaseContainer>> comps, string path, int depth)
	{
		array<string> motions = {};
		array<string> triggers = {};
		array<string> charges = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer source : bucket)
			{
				if (IsMotion(cls))
					motions.Insert(ReadMotion(source, path + "/motion/" + motions.Count().ToString(), depth));
				if (TBD_EquipmentComponentGraph.IsA(cls, "BaseTriggerComponent"))
					triggers.Insert(ReadTrigger(source, path + "/triggers/" + triggers.Count().ToString(), depth));
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_MortarShellGadgetComponent"))
					charges.Insert(ReadChargeConfiguration(source, path + "/charge_configurations/" + charges.Count().ToString()));
			}
		}
		string result = "{\"spawn_as_cartridge\":" + TBD_EquipmentExportJson.Field(root, "SpawnAsCartridge", path + "/spawn_as_cartridge");
		result += ",\"motion\":[" + TBD_EquipmentExportJson.Join(motions) + "]";
		result += ",\"triggers\":[" + TBD_EquipmentExportJson.Join(triggers) + "]";
		result += ",\"charge_configurations\":[" + TBD_EquipmentExportJson.Join(charges) + "]";
		result += ",\"visuals\":" + TBD_EquipmentExportJson.Fields(root, "projectile_model=ProjectileModel|cartridge_model=CartridgeModel", path + "/visuals");
		return result + "}";
	}

	//! Preserve native charge-ring tuples and timed-fuze bounds without expanding or calculating them.
	protected static string ReadChargeConfiguration(BaseContainer source, string path)
	{
		string bindings = "charge_rings=m_aChargeRingConfig|uses_time_fuze=m_bIsUsingTimeFuze";
		bindings += "|minimum_fuze_time=m_fMinFuzeTime|maximum_fuze_time=m_fMaxFuzeTime";
		bindings += "|detonation_altitude=m_fDetonationAltitude|vertical_impact_time_offset=m_fVerticalImpactTimeOffset";
		string result = TBD_EquipmentExportJson.Fields(source, bindings, path);
		return TBD_EquipmentExportJson.Member(result, "source", TBD_EquipmentExportJson.Context(source));
	}

	//! Preserve one motion instance and its source-defined aerodynamic and penetration settings.
	protected static string ReadMotion(BaseContainer source, string path, int depth)
	{
		string motion = "initial_speed=InitSpeed|initial_speed_variation=InitSpeedVariation|dispersion_multiplier=DispersionMultiplier";
		motion += "|mass=Mass|air_drag=AirDrag|time_to_live=TimeToLive|trigger_when_timeout=TriggerWhenTimeout";
		motion += "|gravity_scale=GravityScale|gravity=Gravity|wind_influence=WindInfluence|water_drag=WaterDrag";
		motion += "|ballistic_table=BallisticTableConfig|wind_table=ProjectileWindTableConfig";
		motion += "|parent_speed_multiplier=ParentSpeedMultiplier|wind_influence_multiplier=WindInfluenceMultiplier";
		motion += "|delete_when_stopped=DeleteWhenStop|side_air_drag_scale=SideAirDragScale";
		string result = SerializeProjectileSourceIdentity(source);
		result += ",\"ballistics\":" + TBD_EquipmentExportJson.Fields(source, motion, path + "/ballistics");
		string penetration = "diameter=Diameter|length=Length|depth=PenetrationDepth|speed=PenetrationSpeed|density=PenetrationDensity";
		penetration += "|mushrooming_damage_multiplier=MushroomingDamageMultiplier|tumbling_damage_multiplier=TumblingDamageMultiplier";
		penetration += "|deflection_angle=DeflectionAngle|deflection_angle_variation=DeflectionAngleVariation";
		penetration += "|deflection_critical_angle=DeflectionCriticalAngle|penetrator_type=PenetratorType";
		penetration += "|direction_distribution=PenetrationDirDistribution";
		result += ",\"penetration\":" + TBD_EquipmentExportJson.Fields(source, penetration, path + "/penetration");
		string propulsion = "thrust_initial_time=ThrustInitTime|thrust_time=ThrustTime|thrust_force=ThrustForce";
		propulsion += "|forward_air_friction=ForwardAirFriction|side_air_friction=SideAirFriction|gravity_enable_distance=DistanceEnableGravitation";
		propulsion += "|guidance_type=GuidanceType|max_turn_rate=MaxTurnRate|seeker_field_of_view=SeekerFOV|align_torque=AlignTorque";
		result += ",\"propulsion_and_guidance\":" + TBD_EquipmentExportJson.Fields(source, propulsion, path + "/propulsion_and_guidance");
		result += ",\"effects\":" + ReadEffects(source, "ProjectileEffects", path + "/effects", depth);
		return result + "}";
	}

	//! Read one item trigger with the same ordered gameplay effects and referenced warheads as ammunition.
	static string ExtractTriggerConfiguration(BaseContainer source, string path)
	{
		if (!source || !TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "BaseTriggerComponent"))
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Expected a native projectile trigger component");
			return "null";
		}
		m_ActiveEffects.Clear();
		m_ActiveResources.Clear();
		return ReadTrigger(source, path, 0);
	}

	//! Preserve safety, timing and ordered trigger effects independently of motion effects.
	protected static string ReadTrigger(BaseContainer source, string path, int depth)
	{
		string bindings = "safety_distance=SafetyDistance|arming_time=ArmingTime|trigger_delay=TriggerDelay";
		bindings += "|trigger_offset=Trigger Offset|delete_on_trigger=DELETE_ON_TRIGGER|trigger_alive=TRIGGER_ALIVE";
		bindings += "|enabled=Enabled|timer=TIMER|has_valid_direction=HAS_VALID_DIRECTION";
		if (TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_BaseTriggerComponent"))
			bindings += "|live_by_default=m_bLive";
		if (TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SCR_PressureTriggerComponent"))
			bindings += "|minimum_weight=m_fMinWeight";
		string result = SerializeProjectileSourceIdentity(source);
		result += ",\"settings\":" + TBD_EquipmentExportJson.Fields(source, bindings, path + "/settings");
		result += ",\"effects\":" + ReadEffects(source, "PROJECTILE_EFFECTS", path + "/effects", depth);
		return result + "}";
	}

	//! Keep configured effect ordering; cosmetic and AI presentation effects stay outside gameplay data.
	protected static string ReadEffects(BaseContainer source, string property, string path, int depth)
	{
		if (!source || source.GetVarIndex(property) < 0) return "null";
		BaseContainerList effects = source.GetObjectArray(property);
		if (!effects) return "null";
		array<string> values = {};
		for (int i = 0; i < effects.Count(); i++)
		{
			BaseContainer effect = effects.Get(i);
			if (!effect) { values.Insert("null"); continue; }
			if (ExcludedEffect(effect.GetClassName())) continue;
			string entry = ReadEffect(effect, path + "/" + values.Count().ToString(), depth + 1);
			if (entry != "null")
				entry = "{\"source_index\":" + i.ToString() + "," + TBD_EquipmentExportJson.PreserveSubstring(entry, 1, entry.Length() - 1);
			values.Insert(entry);
		}
		return "[" + TBD_EquipmentExportJson.Join(values) + "]";
	}

	//! Preserve each damage, fragmentation and propagation effect with its own native parameters.
	protected static string ReadEffect(BaseContainer source, string path, int depth)
	{
		if (depth > 64 || m_ActiveEffects.Find(source) >= 0)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Damage effect cycle or traversal limit");
			return "null";
		}
		m_ActiveEffects.Insert(source);
		string bindings = "damage_value=DamageValue|damage_type=DamageType|damage_distance=DamageDistance";
		bindings += "|damage_falloff_curve=DamageFalloffCurve|effect_speed=ExplosionEffectSpeed|trigger_once=TriggerOnce";
		bindings += "|charge_weight=ChargeWeight|fragment_count=DamageFragmentCount|case_weight=CaseWeight";
		bindings += "|fragment_mass_scale=FragMassScale|fragment_range_scale=FragRangeScale";
		bindings += "|duration=m_fDuration|damage_multiplier=m_fDamageMultiplier|damage_threshold=m_fDamageThreshold";
		bindings += "|impulse=Impulse|impulse_distance=ImpulseDistance|impulse_falloff_curve=ImpulseFalloffCurve";
		bindings += "|enabled=Enabled|tnt_equivalent=TntEquivalent|gurney_constant=GurneyConstant|gurney_shape=GurneyShape";
		bindings += "|damage_distance_curve=DamageDistanceCurve|explosion_damage_power=ExplosionDamagePower";
		bindings += "|explosion_impulse_multiplier=ExplosionImpulseMultiplier|perform_traces=PerformTraces";
		bindings += "|resilience_damage_multiplier=m_fResilienceDamageValueMultiplier";
		string result = SerializeProjectileSourceIdentity(source);
		result += ",\"parameters\":" + TBD_EquipmentExportJson.Fields(source, bindings, path + "/parameters");
		result += ",\"damage_effects\":" + ReadEffects(source, "DamageEffects", path + "/damage_effects", depth);
		result += ",\"damage_effect\":" + ReadEffects(source, "DamageEffect", path + "/damage_effect", depth);
		result += ",\"explosion_effects\":" + ReadEffects(source, "ExplosionEffects", path + "/explosion_effects", depth);
		if (source.GetVarIndex("EffectPrefab") >= 0)
		{
			result += ",\"effect_prefab\":" + TBD_EquipmentExportJson.Field(source, "EffectPrefab", path + "/effect_prefab");
			string prefab;
			if (source.Get("EffectPrefab", prefab) && !prefab.IsEmpty())
				result += ",\"warhead\":" + ReadWarhead(prefab, path + "/warhead", depth + 1);
		}
		if (TBD_EquipmentComponentGraph.IsA(source.GetClassName(), "SubmunitionEffect"))
		{
			string submunition = "prefab=Prefab|count=Count|dispersion=Dispersion|initial_speed_coefficient=InitSpeedCoef";
			result += ",\"submunition\":" + TBD_EquipmentExportJson.Fields(source, submunition, path + "/submunition");
			string submunitionPrefab;
			if (source.Get("Prefab", submunitionPrefab) && !submunitionPrefab.IsEmpty())
				result += ",\"submunition_configuration\":" + ReadWarhead(submunitionPrefab, path + "/submunition_configuration", depth + 1);
		}
		m_ActiveEffects.Remove(m_ActiveEffects.Count() - 1);
		return result + "}";
	}

	//! Follow gameplay motion and trigger systems while excluding presentation components of referenced prefabs.
	protected static string ReadWarhead(string resourceName, string path, int depth)
	{
		if (depth > 64 || m_ActiveResources.Find(resourceName) >= 0)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Warhead reference cycle or traversal limit");
			return "null";
		}
		Resource resource = Resource.Load(resourceName);
		if (!resource || !resource.IsValid() || !resource.GetResource())
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Cannot load required warhead " + resourceName);
			return "null";
		}
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Warhead is not a container " + resourceName);
			return "null";
		}
		m_ActiveResources.Insert(resourceName);
		TBD_EquipmentComponentContext context = TBD_EquipmentComponentGraph.SaveContext();
		map<string, ref array<BaseContainer>> components = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, components, 128);
		TBD_EquipmentComponentGraph.m_CurrentResource = context.m_sResource;
		string configuration = ReadConfiguredSystems(root, components, path, depth);
		TBD_EquipmentComponentGraph.RestoreContext(context);
		m_ActiveResources.Remove(m_ActiveResources.Count() - 1);
		return TBD_EquipmentExportJson.Member(configuration, "resource_name", TBD_EquipmentExportJson.Quote(resourceName));
	}

	//! Native motion classes identify configured flight behavior without filename matching.
	protected static bool IsMotion(string className)
	{
		return TBD_EquipmentComponentGraph.IsA(className, "ProjectileMoveComponent");
	}

	//! Exclude explicitly cosmetic or AI-notification effects from gameplay values.
	protected static bool ExcludedEffect(string className)
	{
		array<string> excluded = {"AIHitEffect", "AIExplosionEffect", "AIActivateEffect", "HitSoundEffect", "SpawnDecalEffect", "SpawnParticleEffect", "SpawnDistanceParticleEffect"};
		foreach (string baseName : excluded)
			if (TBD_EquipmentComponentGraph.IsA(className, baseName)) return true;
		return false;
	}

	//! Component identity remains separate from parameter values.
	protected static string SerializeProjectileSourceIdentity(BaseContainer source)
	{
		string value = "{\"native_class\":" + TBD_EquipmentExportJson.Quote(source.GetClassName());
		value += ",\"instance_id\":" + TBD_EquipmentExportJson.Quote(TBD_EquipmentComponentGraph.InstanceId(source));
		return value;
	}

	//! Classify only configurations with an explicit native role; others remain available as unclassified.
	static string CategorizeProjectile(string filePath, string caliber, bool isExplosive)
	{
		Resource resource = Resource.Load(filePath);
		if (!resource || !resource.IsValid() || !resource.GetResource()) return "projectiles_other";
		BaseContainer root = resource.GetResource().ToBaseContainer();
		if (!root) return "projectiles_other";
		BaseContainerList components = root.GetObjectArray("components");
		if (!components) return "projectiles_other";
		bool missile;
		for (int i = 0; i < components.Count(); i++)
		{
			BaseContainer component = components.Get(i);
			if (!component) continue;
			string cls = component.GetClassName();
			if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_MortarShellGadgetComponent")) return "projectiles_mortar";
			if (TBD_EquipmentComponentGraph.IsA(cls, "MissileMoveComponent")) missile = true;
		}
		if (missile) return "projectiles_rockets";
		return "projectiles_other";
	}
}
