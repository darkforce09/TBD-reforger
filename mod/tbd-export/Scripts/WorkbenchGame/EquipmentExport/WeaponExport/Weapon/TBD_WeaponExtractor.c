//------------------------------------------------------------------------------------------------
// TBD_WeaponExtractor.c
//
// Reads a weapon's inventory and physics measurements, native support-point configuration,
// visual references and sight instances without synthesizing capability flags or measurements.
//
// Classification, muzzles, attachment slots and naming each have their own extractor beside this
// one. The scanner calls all five and assembles a single TBD_WeaponInfo from the results.
//------------------------------------------------------------------------------------------------

class TBD_WeaponExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract separate inventory, physics, deployment and visual sections from this installation.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_WeaponPhysicalInfo outPhys)
	{
		outPhys.m_sInventoryJson = TBD_ItemInventoryExtractor.Inventory(comps);
		outPhys.m_sPhysicsJson = TBD_ItemInventoryExtractor.Physics(comps);
		outPhys.m_sDeploymentJson = ExtractDeployment(comps);
		outPhys.m_sVisualsJson = TBD_ItemExtractor.ExtractVisuals(comps);
		outPhys.m_sMountingJson = TBD_AttachmentMountingExtractor.ExtractWeaponMounting(comps);
	}

	//! Preserve each inventory-owned aiming/deployment configuration without inferring a bipod capability.
	static string ExtractDeployment(map<string, ref array<BaseContainer>> comps, string outputPath = "/deployment")
	{
		array<string> configurations = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "InventoryItemComponent")) continue;
			foreach (BaseContainer inventory : bucket)
			{
				BaseContainer attributes = inventory.GetObject("Attributes");
				BaseContainerList custom = TBD_EquipmentDisplayAttributes.GetCustomAttributes(attributes);
				if (!custom) continue;
				for (int i = 0; i < custom.Count(); i++)
				{
					BaseContainer aiming = custom.Get(i);
					if (!aiming || !TBD_EquipmentComponentGraph.IsA(aiming.GetClassName(), "AimingModifierAttributes")) continue;
					string path = outputPath + "/configurations/" + configurations.Count().ToString();
					string json = "{\"source\":" + TBD_EquipmentExportJson.Context(aiming) + "}";
					json = TBD_EquipmentExportJson.Member(json, "owning_component", TBD_EquipmentExportJson.Context(inventory));
					json = TBD_EquipmentExportJson.Member(json, "installation_path", TBD_EquipmentExportJson.Quote(TBD_EquipmentComponentGraph.StructuralPath(inventory)));
					json = TBD_EquipmentExportJson.Member(json, "points", ReadDeploymentPoints(aiming, path + "/points"));
					configurations.Insert(json);
				}
			}
		}
		if (configurations.IsEmpty()) return string.Empty;
		return "{\"configurations\":[" + TBD_EquipmentExportJson.Join(configurations) + "]}";
	}

	//! Keep native point identity, system identifiers, support size and ordered stabilization points.
	protected static string ReadDeploymentPoints(BaseContainer aiming, string path)
	{
		BaseContainerList points = ReadDeploymentArray(aiming, "DeploymentPoints", path);
		if (!points) return "null";
		array<string> entries = {};
		for (int i = 0; i < points.Count(); i++)
		{
			BaseContainer point = points.Get(i);
			if (!point) { entries.Insert("null"); continue; }
			string pointPath = path + "/" + i.ToString();
			string json = TBD_EquipmentExportJson.Fields(point, "system_identifier=SystemIdentifier|stabilization_size=StabilizationSize", pointPath);
			json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(point));
			BaseContainer position = point.GetObject("Point info");
			int variable = point.GetVarIndex("Point info");
			string status = "not_present";
			string nativeType;
			if (variable >= 0)
			{
				status = "present";
				nativeType = typename.EnumToString(DataVarType, point.GetDataVarType(variable));
			}
			TBD_EquipmentExportJson.RecordNativeField(point, pointPath + "/point", status, "BaseContainer.GetObject", "Point info", string.Empty, nativeType);
			json = TBD_EquipmentExportJson.Member(json, "point", SerializeDeploymentPoint(position, pointPath + "/point"));
			json = TBD_EquipmentExportJson.Member(json, "stabilizers", ReadStabilizationPoints(point, pointPath + "/stabilizers"));
			entries.Insert(json);
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Preserve separate support points and null entries instead of collapsing equal configurations.
	protected static string ReadStabilizationPoints(BaseContainer point, string path)
	{
		BaseContainerList stabilizers = ReadDeploymentArray(point, "StabilizationPoints", path);
		if (!stabilizers) return "null";
		array<string> entries = {};
		for (int i = 0; i < stabilizers.Count(); i++)
			entries.Insert(SerializeDeploymentPoint(stabilizers.Get(i), path + "/" + i.ToString()));
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Read native pivot geometry without converting offsets or source-authored empty values.
	protected static string SerializeDeploymentPoint(BaseContainer point, string path)
	{
		if (!point) return "null";
		string json = TBD_EquipmentExportJson.Fields(point, "pivot_id=PivotID|offset=Offset", path);
		return TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(point));
	}

	//! Record array availability separately from explicit empty configuration and failed reads.
	protected static BaseContainerList ReadDeploymentArray(BaseContainer source, string property, string path)
	{
		int variable = source.GetVarIndex(property);
		if (variable < 0)
		{
			TBD_EquipmentExportJson.RecordNativeField(source, path, "not_present", "BaseContainer.GetObjectArray", property, string.Empty, string.Empty);
			return null;
		}
		string nativeType = typename.EnumToString(DataVarType, source.GetDataVarType(variable));
		BaseContainerList entries = source.GetObjectArray(property);
		string status = "present";
		if (!entries)
		{
			status = "error";
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Cannot read " + property);
		}
		TBD_EquipmentExportJson.RecordNativeField(source, path, status, "BaseContainer.GetObjectArray", property, string.Empty, nativeType);
		return entries;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract sights, zeroing distances, default zeroing index, and sight alignment pivots.
	static void ExtractSights(map<string, ref array<BaseContainer>> comps, TBD_WeaponSightsInfo outSights)
	{
		TBD_OpticSightsInfo sights = new TBD_OpticSightsInfo();
		TBD_OpticSightsExtractor.ExtractSights(comps, sights);
		foreach (TBD_OpticSightInstance sight : sights.m_aInstances)
			outSights.m_aInstances.Insert(sight.SerializeJson(""));
	}

	//------------------------------------------------------------------------------------------------
	//! Extract ballistics & dispersion metrics from muzzles.
	static void ExtractBallistics(map<string, ref array<BaseContainer>> comps, TBD_WeaponBallisticsInfo outBallistics)
	{
		// Ballistic values are read and serialized for each muzzle by ExtractMuzzles.
	}
}
