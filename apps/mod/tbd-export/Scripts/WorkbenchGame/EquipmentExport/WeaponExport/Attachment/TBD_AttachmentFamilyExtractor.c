/** Source-backed attachment family properties and native type categorization. */
class TBD_AttachmentFamilyExtractor
{
	//! Classify native attachment types and capabilities without deriving meaning from resource paths.
	static string CategorizeAttachment(map<string, ref array<BaseContainer>> comps, string filePath, string attachmentType)
	{
		if (IsType(attachmentType, "AttachmentMuzzle") || IsType(attachmentType, "AttachmentSuppressor") || IsType(attachmentType, "AttachmentFlashHider")) return "muzzles";
		if (IsType(attachmentType, "AttachmentBayonet") || Has(comps, "SCR_BayonetComponent") || Has(comps, "SCR_BayonetEffectComponent")) return "bayonets";
		if (Has(comps, "SCR_FlashlightComponent") || Has(comps, "FlashlightComponent") || Has(comps, "LaserComponent") || Has(comps, "SCR_LaserComponent")) return "illuminators";
		if (IsType(attachmentType, "AttachmentStock")) return "stocks";
		if (IsType(attachmentType, "AttachmentHandGuard")) return "handguards";
		if (IsType(attachmentType, "AttachmentCamouflage") || IsType(attachmentType, "AttachmentCamouflageOptics")) return "camouflage";
		if (Has(comps, "BipodComponent") || IsType(attachmentType, "AttachmentUnderBarrel")) return "bipods";
		if (Has(comps, "AttachmentSlotComponent")) return "mounts";
		if (Has(comps, "SuppressorComponent")) return "muzzles";
		return "attachments_other";
	}

	//! Preserve all effective suppressor attributes, including explicit zero and false modifiers.
	static void ExtractMuzzleData(map<string, ref array<BaseContainer>> comps, TBD_MuzzleAttachmentInfo outMuzzle, string filePath, string attachmentType)
	{
		outMuzzle.m_sSourceJson = "";
		array<BaseContainer> attributes = {};
		CollectAttributes(comps, "SCR_WeaponAttachmentSuppressorAttributes", attributes);
		array<string> entries = {};
		foreach (BaseContainer attribute : attributes)
		{
			string path = "/muzzle_data/attributes/" + entries.Count().ToString();
			string bindings = "muzzle_speed_coefficient=m_fMuzzleSpeedCoefficient|muzzle_dispersion_factor=m_fMuzzleDispersionFactor|extra_obstruction_length=m_fExtraObstructionLength";
			bindings += "|override_muzzle_effects=m_bOverrideMuzzleEffects|override_shot=m_bOverrideShot";
			bindings += "|recoil_angular_factors=m_vAngularFactors|recoil_linear_factors=m_vLinearFactors|turn_factors=m_vTurnFactors";
			entries.Insert(FieldsWithSource(attribute, bindings, path));
			outMuzzle.m_bHasSuppressorAttributes = true;
			attribute.Get("m_fMuzzleSpeedCoefficient", outMuzzle.m_fMuzzleSpeedCoefficient);
			attribute.Get("m_fMuzzleDispersionFactor", outMuzzle.m_fMuzzleDispersionFactor);
			attribute.Get("m_fExtraObstructionLength", outMuzzle.m_fExtraObstructionLength);
			attribute.Get("m_bOverrideMuzzleEffects", outMuzzle.m_bOverrideMuzzleEffects);
			attribute.Get("m_bOverrideShot", outMuzzle.m_bOverrideShot);
			outMuzzle.m_bHasAngularFactors = attribute.Get("m_vAngularFactors", outMuzzle.m_vAngularFactors);
			outMuzzle.m_bHasLinearFactors = attribute.Get("m_vLinearFactors", outMuzzle.m_vLinearFactors);
			outMuzzle.m_bHasTurnFactors = attribute.Get("m_vTurnFactors", outMuzzle.m_vTurnFactors);
		}
		if (!entries.IsEmpty()) outMuzzle.m_sSourceJson = "{\"attributes\":[" + TBD_EquipmentExportJson.Join(entries) + "]}";
	}

	//! Keep authored bayonet identity and each gameplay modifier separate.
	static void ExtractBayonetData(map<string, ref array<BaseContainer>> comps, TBD_BayonetAttachmentInfo outBayonet, string filePath, string attachmentType)
	{
		outBayonet.m_sSourceJson = "";
		array<BaseContainer> attributes = {};
		CollectAttributes(comps, "SCR_WeaponAttachmentBayonetAttributes", attributes);
		array<string> entries = {};
		foreach (BaseContainer attribute : attributes)
		{
			string path = "/bayonet_data/attributes/" + entries.Count().ToString();
			string bindings = "is_bayonet=m_bIsBayonet|damage_modification_factor=m_fDamageModificationFactor|extra_obstruction_length=m_fExtraObstructionLength";
			bindings += "|precision_modification_factor=m_fPrecisionModificationFactor|range_modification_factor=m_fRangeModificationFactor";
			entries.Insert(FieldsWithSource(attribute, bindings, path));
			outBayonet.m_bHasBayonetAttributes = true;
			attribute.Get("m_bIsBayonet", outBayonet.m_bIsBayonet);
			attribute.Get("m_fDamageModificationFactor", outBayonet.m_fDamageModificationFactor);
			attribute.Get("m_fExtraObstructionLength", outBayonet.m_fExtraObstructionLength);
			attribute.Get("m_fPrecisionModificationFactor", outBayonet.m_fPrecisionModificationFactor);
			attribute.Get("m_fRangeModificationFactor", outBayonet.m_fRangeModificationFactor);
		}
		if (!entries.IsEmpty()) outBayonet.m_sSourceJson = "{\"attributes\":[" + TBD_EquipmentExportJson.Join(entries) + "]}";
	}

	//! Preserve individual light configurations and the ordered source lens array.
	static void ExtractIlluminatorData(map<string, ref array<BaseContainer>> comps, TBD_IlluminatorAttachmentInfo outIllum, string filePath, string attachmentType)
	{
		outIllum.m_sSourceJson = "";
		array<string> entries = {};
		foreach (string className, array<BaseContainer> instances : comps)
		{
			bool flashlight = IsType(className, "SCR_FlashlightComponent");
			bool laser = IsType(className, "LaserComponent") || IsType(className, "SCR_LaserComponent");
			if (!flashlight && !laser && !IsType(className, "FlashlightComponent")) continue;
			foreach (BaseContainer component : instances)
			{
				string path = "/illuminator_data/components/" + entries.Count().ToString();
				string entry;
				if (flashlight)
				{
					string bindings = "enabled=Enabled|emissive_intensity=m_fEmissiveIntensity|flashlight_adjust_offset=m_vFlashlightAdjustOffset";
					bindings += "|light_near_plane_hand=m_fLightNearPlaneHand|light_near_plane_strapped=m_fLightNearPlaneStrapped|light_near_plane_vehicle=m_fLightNearPlaneVehicle";
					entry = FieldsWithSource(component, bindings, path);
					string lenses = TBD_EquipmentExportJson.ObjectArray(component, "m_aLenseArray", "description=m_sDescription|color=m_vLenseColor|light_value=m_fLightValue", path + "/lenses");
					entry = TBD_EquipmentExportJson.Member(entry, "lenses", lenses);
					outIllum.m_bHasLight = true;
					component.Get("m_fEmissiveIntensity", outIllum.m_fEmissiveIntensity);
					outIllum.m_bHasAdjustOffset = component.Get("m_vFlashlightAdjustOffset", outIllum.m_vFlashlightAdjustOffset);
					component.Get("m_fLightNearPlaneHand", outIllum.m_fLightNearPlaneHand);
				}
				else entry = FieldsWithSource(component, "enabled=Enabled", path);
				if (laser) outIllum.m_bHasLaser = true;
				entries.Insert(entry);
			}
		}
		if (!entries.IsEmpty()) outIllum.m_sSourceJson = "{\"components\":[" + TBD_EquipmentExportJson.Join(entries) + "]}";
	}

	//! Preserve each effective handguard slot, including default installations and obstructions.
	static void ExtractHandguardData(map<string, ref array<BaseContainer>> comps, TBD_HandguardAttachmentInfo outHg, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outHg.m_aNestedSlots, "/handguard_data/nested_attachment_slots");
	}

	//! Preserve each effective adapter slot without expanding compatibility into equipment pairs.
	static void ExtractMountData(map<string, ref array<BaseContainer>> comps, TBD_MountAttachmentInfo outMount, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outMount.m_aNestedSlots, "/mount_data/nested_attachment_slots");
	}

	//! Preserve individual stock-provided slots.
	static void ExtractStockData(map<string, ref array<BaseContainer>> comps, TBD_StockAttachmentInfo outStock, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outStock.m_aNestedSlots, "/stock_data/nested_attachment_slots");
	}

	//! Preserve individual bipod-provided slots.
	static void ExtractBipodData(map<string, ref array<BaseContainer>> comps, TBD_BipodAttachmentInfo outBipod, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outBipod.m_aNestedSlots, "/bipod_data/nested_attachment_slots");
	}

	//! Retain native camouflage mount identity without inventing a target or protection rating.
	static void ExtractCamouflageData(map<string, ref array<BaseContainer>> comps, TBD_CamouflageAttachmentInfo outCamo, string filePath, string attachmentType = "")
	{
		outCamo.m_sSourceJson = "";
		array<BaseContainer> attributes = {};
		CollectAttributes(comps, "AttachmentAttributes", attributes);
		array<string> entries = {};
		foreach (BaseContainer attribute : attributes)
		{
			BaseContainer mount = attribute.GetObject("AttachmentType");
			if (!mount) continue;
			string nativeType = mount.GetClassName();
			if (!IsType(nativeType, "AttachmentCamouflage") && !IsType(nativeType, "AttachmentCamouflageOptics")) continue;
			string entry = "{\"attachment_type\":" + TBD_EquipmentExportJson.Quote(nativeType);
			entry += ",\"source\":" + TBD_EquipmentExportJson.Context(attribute);
			entry += ",\"type_source\":" + TBD_EquipmentExportJson.Context(mount) + "}";
			entries.Insert(entry);
		}
		if (!entries.IsEmpty()) outCamo.m_sSourceJson = "{\"attributes\":[" + TBD_EquipmentExportJson.Join(entries) + "]}";
	}

	//! Effective array reads preserve removals and empty overrides; no ancestor accumulation occurs.
	protected static void CollectAttributes(map<string, ref array<BaseContainer>> comps, string nativeClass, array<BaseContainer> output)
	{
		foreach (string className, array<BaseContainer> instances : comps)
		{
			if (!IsType(className, "InventoryItemComponent")) continue;
			foreach (BaseContainer component : instances)
			{
				BaseContainer attributes = component.GetObject("Attributes");
				if (!attributes) continue;
				BaseContainerList custom = attributes.GetObjectArray("CustomAttributes");
				if (!custom)
				{
					BaseContainer wrapper = attributes.GetObject("CustomAttributes");
					if (wrapper) custom = wrapper.GetObjectArray("m_aAttributes");
				}
				if (!custom) continue;
				for (int i = 0; i < custom.Count(); i++)
				{
					BaseContainer attribute = custom.Get(i);
					if (attribute && IsType(attribute.GetClassName(), nativeClass)) output.Insert(attribute);
				}
			}
		}
	}

	//! Values use common source metadata while installations retain their own contexts.
	protected static string FieldsWithSource(BaseContainer source, string bindings, string path)
	{
		string json = TBD_EquipmentExportJson.Fields(source, bindings, path);
		return TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(source));
	}

	//! Native inheritance includes addon-defined subclasses.
	protected static bool IsType(string className, string baseName)
	{
		return TBD_EquipmentComponentGraph.IsA(className, baseName);
	}

	//! Inspect component capability through native inheritance.
	protected static bool Has(map<string, ref array<BaseContainer>> comps, string className)
	{
		return TBD_EquipmentComponentGraph.HasCompSuffix(comps, className);
	}
}
