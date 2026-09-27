/**
 * Item capability extraction reads effective native containers.
 * Each repeated component and ordered configuration entry retains its identity.
 * Field metadata records source types, origins and unavailable values separately.
 */
//! Extract the gameplay capabilities of inventory items without path-based specifications.
class TBD_ItemExtractor
{
	//! Read effective inventory attributes for callers using the standard physical model.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_ItemPhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "InventoryItemComponent")) continue;
			foreach (BaseContainer comp : bucket)
			{
				BaseContainer attrs = comp.GetObject("Attributes");
				if (!attrs) continue;
				int inventorySize;
				if (attrs.Get("m_Size", inventorySize)) outPhys.m_sInventorySize = inventorySize.ToString();
				BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
				if (!phys) continue;
				phys.Get("Weight", outPhys.m_fWeightKg);
				phys.Get("ItemVolume", outPhys.m_fVolumeCm3);
				phys.Get("ItemDimensions", outPhys.m_vDimensions);
				return;
			}
		}
	}

	//! Preserve separate amounts, rates, durations and the actual consumable effect class.
	static void ExtractMedical(map<string, ref array<BaseContainer>> comps, TBD_ItemMedicalInfo outMed, string filePath)
	{
		array<string> effects = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "SCR_ConsumableItemComponent")) continue;
			foreach (BaseContainer comp : bucket)
			{
				BaseContainer effect = comp.GetObject("m_ConsumableEffect");
				if (!effect || !TBD_EquipmentComponentGraph.IsA(effect.GetClassName(), "SCR_ConsumableEffectHealthItems")) continue;
				string path = "medical.effects[" + effects.Count().ToString() + "]";
				string bindings = "consumable_type=m_eConsumableType|amount=m_fItemAbsoluteRegenerationAmount|rate=m_fItemRegenerationSpeed";
				bindings += "|duration=m_fItemRegenerationDuration|apply_to_self_duration=m_fApplyToSelfDuration|apply_to_other_duration=m_fApplyToOtherDuration";
				bindings += "|delete_on_use=m_bDeleteOnUse|target_hit_zone_group=m_eTargetHZGroup|group_roles=m_aGroupRoles";
				bindings += "|character_labels=m_aCharacterLabels|role_speed_bonus=m_fRoleSpeedBonus|role_speed_penalty=m_fRoleSpeedPenalty";
				string data = Describe(effect, bindings, path);
				string damage = TypedArray(effect, "m_aDamageEffectsToLoad", "duration_per_hit_zone=m_fDurationPerHitZone", path + ".damage_effects");
				data = Extend(data, "damage_effects", damage);
				data = Extend(data, "owning_component", TBD_EquipmentExportJson.Quote(TBD_EquipmentComponentGraph.InstanceId(comp)));
				effects.Insert(data);
				outMed.m_sConsumableType = effect.GetClassName();
			}
		}
		outMed.m_bIsMedical = !effects.IsEmpty();
		if (outMed.m_bIsMedical) outMed.m_sJson = "{\"effects\":[" + TBD_EquipmentExportJson.Join(effects) + "]}";
	}

	//! Preserve every transceiver independently; range and frequency keep their native meanings.
	static void ExtractRadio(map<string, ref array<BaseContainer>> comps, TBD_ItemRadioInfo outRadio, string filePath)
	{
		array<string> radios = {};
		array<string> controls = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer comp : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "BaseRadioComponent"))
				{
					string path = "radio.components[" + radios.Count().ToString() + "]";
					string data = Describe(comp, "encryption_key=Encryption key|turned_on=Turned on", path);
					string bindings = "configured_frequency=ChannelFrequency|minimum_frequency=Min tunable frequency|maximum_frequency=Max tunable frequency";
					bindings += "|frequency_step=Frequency resolution|transmission_range=Transmitting Range";
					data = Extend(data, "transceivers", TypedArray(comp, "Transceivers", bindings, path + ".transceivers"));
					radios.Insert(data);
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_RadioComponent"))
				{
					string controlPath = "radio.controls[" + controls.Count().ToString() + "]";
					controls.Insert(Describe(comp, "category=m_iRadioCategory|type=m_iRadioType", controlPath));
				}
			}
		}
		outRadio.m_bIsRadio = !radios.IsEmpty() || !controls.IsEmpty();
		if (!outRadio.m_bIsRadio) return;
		outRadio.m_sJson = "{\"components\":[" + TBD_EquipmentExportJson.Join(radios) + "]";
		outRadio.m_sJson += ",\"controls\":[" + TBD_EquipmentExportJson.Join(controls) + "]}";
	}

	//! Read optical and handheld gadget configuration without converting FOV or inferring magnification.
	static void ExtractGadget(map<string, ref array<BaseContainer>> comps, TBD_ItemGadgetInfo outGadget, string filePath)
	{
		array<string> gadgets = {};
		array<string> optics = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer comp : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_GadgetComponent"))
				{
					string path = "gadget.components[" + gadgets.Count().ToString() + "]";
					string bindings = "use_mask=m_eUseMask|can_be_held=m_bCanBeHeld|weapon_no_fire_time=m_fWeaponNoFireTime";
					if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_DetonatorGadgetComponent")) bindings += "|maximum_detonation_range=m_fMaxDetonationRange";
					if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_CompassComponent")) bindings += "|compass_type=m_iCompassType";
					if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_SupportStationGadgetComponent")) bindings += "|support_station_types=m_aSupportStationTypes";
					gadgets.Insert(Describe(comp, bindings, path));
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_2DOpticsComponent"))
				{
					string opticalPath = "gadget.optics[" + optics.Count().ToString() + "]";
					string opticalBindings = "magnification=m_fMagnification|objective_fov=m_fObjectiveFov|reticle_angular_size=m_fReticleAngularSize";
					opticalBindings += "|reticle_base_zoom=m_fReticleBaseZoom|reticle_texture=m_sReticleTexture|reticle_glow_texture=m_sReticleGlowTexture";
					string data = Describe(comp, opticalBindings, opticalPath);
					BaseContainer fov = comp.GetObject("SightsFOVInfo");
					if (fov)
					{
						string fovBindings = "field_of_view=m_fFieldOfView|base_zoom=m_fBaseZoom|maximum_zoom=m_fZoomMax";
						fovBindings += "|zoom_step=m_fStepZoomSize|interpolation_speed=m_fInterpolationSpeed";
						data = Extend(data, "fov", Describe(fov, fovBindings, opticalPath + ".fov"));
					}
					optics.Insert(data);
				}
			}
		}
		outGadget.m_bIsGadget = !gadgets.IsEmpty() || !optics.IsEmpty();
		if (!outGadget.m_bIsGadget) return;
		outGadget.m_sJson = "{\"components\":[" + TBD_EquipmentExportJson.Join(gadgets) + "]";
		outGadget.m_sJson += ",\"optics\":[" + TBD_EquipmentExportJson.Join(optics) + "]}";
	}

	//! Read native trigger damage systems and preserve independent charge and fuze-slot configuration.
	static void ExtractExplosive(map<string, ref array<BaseContainer>> comps, TBD_ItemExplosiveInfo outExp, string filePath)
	{
		array<string> triggers = {};
		array<string> charges = {};
		array<string> fuzes = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer comp : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "BaseTriggerComponent"))
				{
					string path = "/explosive/triggers/" + triggers.Count().ToString();
					triggers.Insert(TBD_AmmoProjectileExtractor.ExtractTriggerConfiguration(comp, path));
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_ExplosiveChargeComponent"))
					charges.Insert(Describe(comp, "fuze_time=m_fFuzeTime|fuze_type=m_eUsedFuzeType", "explosive.charges[" + charges.Count().ToString() + "]"));
				if (TBD_EquipmentComponentGraph.IsA(cls, "SlotManagerComponent"))
				{
					BaseContainerList slots = comp.GetObjectArray("Slots");
					if (!slots) continue;
					for (int i = 0; i < slots.Count(); i++)
					{
						BaseContainer slot = slots.Get(i);
						if (!slot || !TBD_EquipmentComponentGraph.IsA(slot.GetClassName(), "FuzeSlotInfo")) continue;
						fuzes.Insert(Describe(slot, "prefab=Prefab|fuze_type=m_eFuzeType", "explosive.fuzes[" + fuzes.Count().ToString() + "]"));
					}
				}
			}
		}
		outExp.m_bIsExplosive = !triggers.IsEmpty() || !charges.IsEmpty() || !fuzes.IsEmpty();
		if (!outExp.m_bIsExplosive) return;
		outExp.m_sJson = "{\"triggers\":[" + TBD_EquipmentExportJson.Join(triggers) + "]";
		outExp.m_sJson += ",\"charges\":[" + TBD_EquipmentExportJson.Join(charges) + "]";
		outExp.m_sJson += ",\"fuzes\":[" + TBD_EquipmentExportJson.Join(fuzes) + "]}";
	}

	//! Tool capabilities come from configured construction and support components.
	static void ExtractTool(map<string, ref array<BaseContainer>> comps, TBD_ItemToolInfo outTool, string filePath, string outputPath = "tool")
	{
		array<string> components = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			string bindings;
			if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_CampaignBuildingGadgetToolComponent"))
				bindings = "construction_value=m_iConstructionValue|build_distance=m_fDistanceToBuildComposition";
			else if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_BaseSupportStationComponent"))
			{
				bindings = "range=m_fRange|enabled=m_bIsEnabled|base_supply_cost=m_iBaseSupplyCostOnUse|faction_usage=m_eFactionUsageCheck";
				bindings += "|allow_parent_faction=m_bAllowGetFactionFromParent|is_vehicle=m_bIsVehicle|ignore_self=m_bIgnoreSelf|offset=m_vOffset|use_bounding_box_range=m_bUseRangeBoundingBox";
				bindings += "|priority=m_eSupportStationPriority";
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_BaseDamageHealSupportStationComponent"))
				{
					bindings += "|damage_healed_each_execution=m_iDamageHealedEachExecution|supply_cost_damage_healed=m_iSupplyCostDamageHealed";
					bindings += "|maximum_heal_scaled=m_fMaxHealScaled|damage_over_time_types_healed=m_aDoTTypesHealed|supplies_per_damage_over_time_healed=m_iSuppliesPerDoTHealed";
					bindings += "|server_added_maximum_heal_scaled=m_fServerAddedMaxHealScaled|maximum_heal_reached_reason=m_eMaxHealDone";
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_RepairSupportStationComponent"))
					bindings += "|fire_rate_reduction=m_fFireRateReductionEachExecute|supply_cost_per_fire_rate_reduction=m_iSupplyCostPerFireRateReduction";
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_HealSupportStationComponent"))
					bindings += "|blood_healed_each_execution=m_iBloodHealedEachExecute|supply_cost_blood_healed=m_iSupplyCostBloodHealed|maximum_blood_scaled=m_fMaxBloodScaled";
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_ResupplySupportStationComponent"))
					bindings += "|supply_cost_type=m_eSupplyCostType|fallback_item_supply_cost=m_iFallbackItemSupplyCost";
			}
			if (bindings.IsEmpty()) continue;
			foreach (BaseContainer comp : bucket)
				components.Insert(Describe(comp, bindings, outputPath + ".components[" + components.Count().ToString() + "]"));
		}
		outTool.m_bIsTool = !components.IsEmpty();
		if (outTool.m_bIsTool) outTool.m_sJson = "{\"components\":[" + TBD_EquipmentExportJson.Join(components) + "]}";
	}

	//! Preserve deployable replacements, variant parts and source-defined respawn limits.
	static void ExtractSurvival(map<string, ref array<BaseContainer>> comps, TBD_ItemSurvivalInfo outSurv, string filePath)
	{
		array<string> deployables = {};
		array<string> fuel = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_FuelManagerComponent"))
			{
				foreach (BaseContainer manager : bucket)
				{
					string fuelPath = "survival.fuel[" + fuel.Count().ToString() + "]";
					string fuelData = Describe(manager, "", fuelPath);
					string fuelFields = "fuel_type=FuelType|maximum_fuel=MaxFuel|initial_fuel=m_fInitialFuelTankState";
					fuelFields += "|maximum_flow_out=m_MaxFlowCapacityOut|leak_speed=m_iFuelLeakSpeed|node_type=m_eFuelNodeType";
					fuel.Insert(Extend(fuelData, "nodes", TypedArray(manager, "FuelNodes", fuelFields, fuelPath + ".nodes")));
				}
			}
			if (!TBD_EquipmentComponentGraph.IsA(cls, "SCR_BaseDeployableInventoryItemComponent")) continue;
			foreach (BaseContainer comp : bucket)
			{
				string path = "survival.deployables[" + deployables.Count().ToString() + "]";
				string data = Describe(comp, "replacement_prefab=m_sReplacementPrefab|delete_when_destroyed=m_bDeleteWhenDestroyed", path);
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_RestrictedDeployableSpawnPointComponent"))
				{
					string limits = "spawn_point_prefab=m_sSpawnPointPrefab|supplies_spawn_point_prefab=m_sSpawnPointPrefabSupplies|faction=m_FactionKey";
					limits += "|budget_type=m_eRespawnBudgetType|supplies_value=m_fSuppliesValue|maximum_respawns=m_iMaxRespawns";
					limits += "|respawn_generation_time=m_iRespawnGenerationTime|respawn_generation_amount=m_iRespawnGenerationAmount";
					data = Extend(data, "spawn_point", TBD_EquipmentExportJson.Fields(comp, limits, path + ".spawn_point"));
				}
				BaseContainerList variants = comp.GetObjectArray("m_aVariants");
				if (variants)
				{
					array<string> entries = {};
					for (int v = 0; v < variants.Count(); v++)
					{
						BaseContainer variant = variants.Get(v);
						string variantPath = path + ".variants[" + v.ToString() + "]";
						if (!variant) { entries.Insert("null"); continue; }
						string entry = Describe(variant, "variant_id=m_iVariantId|replacement_prefab=m_sReplacementPrefab|preview_prefab=m_sPreviewObject", variantPath);
						string required = "prefab=m_sPrefab|quantity=m_iNumberOfRequiredPrefabs|name=m_sPartName|delete_on_deployment=m_bDeletePartsOnDeployment|detach_magazines=m_bDetachMagazinesWhenUsed";
						entry = Extend(entry, "required_parts", TypedArray(variant, "m_aRequiredElements", required, variantPath + ".required_parts"));
						entry = Extend(entry, "additional_parts", TypedArray(variant, "m_aAdditionalPrefabs", "prefab=m_sPrefab|quantity=m_iNumberOfPrefabs", variantPath + ".additional_parts"));
						entries.Insert(entry);
					}
					data = Extend(data, "variants", "[" + TBD_EquipmentExportJson.Join(entries) + "]");
				}
				deployables.Insert(data);
			}
		}
		outSurv.m_bIsSurvival = !deployables.IsEmpty() || !fuel.IsEmpty();
		if (!outSurv.m_bIsSurvival) return;
		outSurv.m_sJson = "{\"deployables\":[" + TBD_EquipmentExportJson.Join(deployables) + "]";
		outSurv.m_sJson += ",\"fuel\":[" + TBD_EquipmentExportJson.Join(fuel) + "]}";
	}

	//! Preserve separate native icon, item model and preview references with their owning instances.
	static string ExtractVisuals(map<string, ref array<BaseContainer>> comps)
	{
		BaseContainer display = TBD_EquipmentDisplayAttributes.DisplayContainer(comps);
		string visual = TBD_EquipmentExportJson.Fields(display, "icon=Icon", "/visuals");
		array<string> models = {};
		array<string> previews = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer component : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "MeshObject"))
				{
					string modelPath = "/visuals/models/" + models.Count().ToString();
					string model = TBD_EquipmentExportJson.Fields(component, "item_model=Object", modelPath);
					models.Insert(Extend(model, "source", TBD_EquipmentExportJson.Context(component)));
				}
				if (!TBD_EquipmentComponentGraph.IsA(cls, "InventoryItemComponent")) continue;
				BaseContainer attributes = component.GetObject("Attributes");
				BaseContainerList custom = TBD_EquipmentDisplayAttributes.GetCustomAttributes(attributes);
				if (!custom) continue;
				for (int i = 0; i < custom.Count(); i++)
				{
					BaseContainer entry = custom.Get(i);
					if (!entry || !TBD_EquipmentComponentGraph.IsA(entry.GetClassName(), "PreviewRenderAttributes")) continue;
					string previewPath = "/visuals/previews/" + previews.Count().ToString();
					string fields = "preview_model=PreviewModel|preview_prefab=PreviewPrefab|preview_worn_model=PreviewWornModel";
					string preview = TBD_EquipmentExportJson.Fields(entry, fields, previewPath);
					preview = Extend(preview, "source", TBD_EquipmentExportJson.Context(entry));
					preview = Extend(preview, "owning_component", TBD_EquipmentExportJson.Quote(TBD_EquipmentComponentGraph.InstanceId(component)));
					previews.Insert(preview);
				}
			}
		}
		visual = Extend(visual, "models", "[" + TBD_EquipmentExportJson.Join(models) + "]");
		return Extend(visual, "previews", "[" + TBD_EquipmentExportJson.Join(previews) + "]");
	}

	//! Add native container identity to explicitly selected domain fields.
	protected static string Describe(BaseContainer source, string bindings, string path)
	{
		string data = TBD_EquipmentExportJson.Fields(source, bindings, path);
		data = Extend(data, "native_class", TBD_EquipmentExportJson.Quote(source.GetClassName()));
		return Extend(data, "instance_id", TBD_EquipmentExportJson.Quote(TBD_EquipmentComponentGraph.InstanceId(source)));
	}

	//! Preserve ordered entries and explicit null objects without deduplicating instances.
	protected static string TypedArray(BaseContainer source, string property, string bindings, string path)
	{
		BaseContainerList values = source.GetObjectArray(property);
		if (!values) return "null";
		array<string> entries = {};
		for (int i = 0; i < values.Count(); i++)
		{
			BaseContainer value = values.Get(i);
			if (!value) entries.Insert("null");
			else entries.Insert(Describe(value, bindings, path + "[" + i.ToString() + "]"));
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Append one member to a JSON object produced by the standard field writer.
	protected static string Extend(string objectJson, string name, string value)
	{
		string separator;
		if (objectJson.Length() > 2) separator = ",";
		return TBD_EquipmentExportJson.PreserveSubstring(objectJson, 0, objectJson.Length() - 1) + separator + TBD_EquipmentExportJson.Quote(name) + ":" + value + "}";
	}
}
