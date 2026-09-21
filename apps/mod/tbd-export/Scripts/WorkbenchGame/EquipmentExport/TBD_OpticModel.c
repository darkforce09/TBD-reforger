//------------------------------------------------------------------------------------------------
// TBD_OpticModel.c
//
// Strong data models representing optical sights and scopes (collimators, reflex sights,
// holographic sights, telescopic scopes, variable power optics, launcher sights, and backup sights)
// for the TBD Reforger Universal Equipment Export System.
//
// Pure ground-truth extraction:
//   - ZERO synthetic defaults: omitted values serialize as JSON null.
//   - ZERO token mutation: raw engine localization strings (e.g. #AR-Item_Optic_ARTII_Name)
//     are preserved verbatim without stripping # or humanizing.
//   - ZERO hardcoded data: mounting keys and optical attributes reflect only what is genuinely
//     declared on the BaseContainer and its container ancestry.
//   - NO synthetic fields: id, family, category, and addon are eliminated.
//------------------------------------------------------------------------------------------------

class TBD_OpticMountingInfo
{
	string m_sAttachmentType;
	ref array<string> m_aCompatibleAttachmentTypes = {};
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_OpticSightsInfo
{
	string m_sSightsClass;
	bool m_bIsMagnified = false;

	float m_fMagnificationMin = -1.0;
	bool m_bHasMagnificationMin = false;
	float m_fMagnificationMax = -1.0;
	bool m_bHasMagnificationMax = false;
	ref array<float> m_aMagnificationSteps = {};
	bool m_bHasMagnificationSteps = false;

	float m_fFovMinDegrees = -1.0;
	bool m_bHasFovMin = false;
	float m_fFovMaxDegrees = -1.0;
	bool m_bHasFovMax = false;

	float m_fEyeReliefMeters = -1.0;
	bool m_bHasEyeRelief = false;

	float m_fObjectiveDiameterMm = -1.0;
	bool m_bHasObjectiveDiameter = false;

	ref array<string> m_aZeroingDistances = {};
	bool m_bHasZeroingDistances = false;

	string m_sReticleTexture;

	ref array<float> m_aReticleColor = {};
	bool m_bHasReticleColor = false;

	ref array<float> m_aGlowColor = {};
	bool m_bHasGlowColor = false;

	bool m_bHasRangefinder = false;
	bool m_bHasRangefinderSet = false;
}

class TBD_OpticPhysicalInfo
{
	float m_fWeightKg = -1.0;
	bool m_bHasWeight = false;
	float m_fVolumeCm3 = -1.0;
	bool m_bHasVolume = false;
	vector m_vDimensions = "0 0 0";
	bool m_bHasDimensions = false;
	string m_sInventorySize;
}

class TBD_OpticVisualsInfo
{
	string m_sModelMesh;
}

class TBD_OpticInfo
{
	string m_sResourceName;
	string m_sFilePath;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_OpticMountingInfo m_Mounting = new TBD_OpticMountingInfo();
	ref TBD_OpticSightsInfo m_Sights = new TBD_OpticSightsInfo();
	ref TBD_OpticPhysicalInfo m_Physical = new TBD_OpticPhysicalInfo();
	ref TBD_OpticVisualsInfo m_Visuals = new TBD_OpticVisualsInfo();

	//------------------------------------------------------------------------------------------------
	//! Serialize optic entry to clean, indented JSON string strictly adhering to Target Data Schema
	string SerializeJson(string indent = "    ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";

		// 1. Identity & Raw Engine Strings
		json += sub + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";

		if (!m_sDisplayName.IsEmpty())
			json += sub + "\"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";
		else
			json += sub + "\"display_name\": null,\n";

		if (!m_sDescription.IsEmpty())
			json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		else
			json += sub + "\"description\": null,\n";

		if (!m_sIcon.IsEmpty())
			json += sub + "\"icon\": \"" + TBD_EquipmentExportJson.Escape(m_sIcon) + "\",\n";
		else
			json += sub + "\"icon\": null,\n";

		string isAbsStr = "false";
		if (m_bIsAbstract) isAbsStr = "true";
		json += sub + "\"is_abstract\": " + isAbsStr + ",\n";

		if (!m_sVariantOf.IsEmpty())
			json += sub + "\"variant_of\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			json += sub + "\"variant_of\": null,\n";

		// 2. Mounting & Relational Keys
		json += sub + "\"mounting\": {\n";
		if (!m_Mounting.m_sAttachmentType.IsEmpty())
			json += sub2 + "\"attachment_type\": \"" + TBD_EquipmentExportJson.Escape(m_Mounting.m_sAttachmentType) + "\",\n";
		else
			json += sub2 + "\"attachment_type\": null,\n";

		json += sub2 + "\"compatible_attachment_types\": [";
		for (int c = 0; c < m_Mounting.m_aCompatibleAttachmentTypes.Count(); c++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_Mounting.m_aCompatibleAttachmentTypes[c]) + "\"";
			if (c < m_Mounting.m_aCompatibleAttachmentTypes.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		json += sub2 + "\"obstructed_attachment_types\": [";
		for (int o = 0; o < m_Mounting.m_aObstructedAttachmentTypes.Count(); o++)
		{
			json += "\"" + TBD_EquipmentExportJson.Escape(m_Mounting.m_aObstructedAttachmentTypes[o]) + "\"";
			if (o < m_Mounting.m_aObstructedAttachmentTypes.Count() - 1)
				json += ", ";
		}
		json += "]\n";
		json += sub + "},\n";

		// 3. Optics & Sights Introspection
		json += sub + "\"sights\": {\n";
		if (!m_Sights.m_sSightsClass.IsEmpty())
			json += sub2 + "\"sights_class\": \"" + TBD_EquipmentExportJson.Escape(m_Sights.m_sSightsClass) + "\",\n";
		else
			json += sub2 + "\"sights_class\": null,\n";

		string isMagStr = "false";
		if (m_Sights.m_bIsMagnified) isMagStr = "true";
		json += sub2 + "\"is_magnified\": " + isMagStr + ",\n";

		if (m_Sights.m_bHasMagnificationMin)
			json += sub2 + "\"magnification_min\": " + m_Sights.m_fMagnificationMin.ToString() + ",\n";
		else
			json += sub2 + "\"magnification_min\": null,\n";

		if (m_Sights.m_bHasMagnificationMax)
			json += sub2 + "\"magnification_max\": " + m_Sights.m_fMagnificationMax.ToString() + ",\n";
		else
			json += sub2 + "\"magnification_max\": null,\n";

		if (m_Sights.m_bHasMagnificationSteps && m_Sights.m_aMagnificationSteps.Count() > 0)
		{
			json += sub2 + "\"magnification_steps\": [";
			for (int ms = 0; ms < m_Sights.m_aMagnificationSteps.Count(); ms++)
			{
				json += m_Sights.m_aMagnificationSteps[ms].ToString();
				if (ms < m_Sights.m_aMagnificationSteps.Count() - 1)
					json += ", ";
			}
			json += "],\n";
		}
		else
		{
			json += sub2 + "\"magnification_steps\": null,\n";
		}

		if (m_Sights.m_bHasFovMin)
			json += sub2 + "\"fov_min_degrees\": " + m_Sights.m_fFovMinDegrees.ToString() + ",\n";
		else
			json += sub2 + "\"fov_min_degrees\": null,\n";

		if (m_Sights.m_bHasFovMax)
			json += sub2 + "\"fov_max_degrees\": " + m_Sights.m_fFovMaxDegrees.ToString() + ",\n";
		else
			json += sub2 + "\"fov_max_degrees\": null,\n";

		if (m_Sights.m_bHasEyeRelief)
			json += sub2 + "\"eye_relief_meters\": " + m_Sights.m_fEyeReliefMeters.ToString() + ",\n";
		else
			json += sub2 + "\"eye_relief_meters\": null,\n";

		if (m_Sights.m_bHasObjectiveDiameter)
			json += sub2 + "\"objective_diameter_mm\": " + m_Sights.m_fObjectiveDiameterMm.ToString() + ",\n";
		else
			json += sub2 + "\"objective_diameter_mm\": null,\n";

		if (m_Sights.m_bHasZeroingDistances && m_Sights.m_aZeroingDistances.Count() > 0)
		{
			json += sub2 + "\"zeroing_distances\": [";
			for (int zd = 0; zd < m_Sights.m_aZeroingDistances.Count(); zd++)
			{
				json += "\"" + TBD_EquipmentExportJson.Escape(m_Sights.m_aZeroingDistances[zd]) + "\"";
				if (zd < m_Sights.m_aZeroingDistances.Count() - 1)
					json += ", ";
			}
			json += "],\n";
		}
		else
		{
			json += sub2 + "\"zeroing_distances\": null,\n";
		}

		if (!m_Sights.m_sReticleTexture.IsEmpty())
			json += sub2 + "\"reticle_texture\": \"" + TBD_EquipmentExportJson.Escape(m_Sights.m_sReticleTexture) + "\",\n";
		else
			json += sub2 + "\"reticle_texture\": null,\n";

		if (m_Sights.m_bHasReticleColor && m_Sights.m_aReticleColor.Count() > 0)
		{
			json += sub2 + "\"reticle_color\": [";
			for (int rc = 0; rc < m_Sights.m_aReticleColor.Count(); rc++)
			{
				json += m_Sights.m_aReticleColor[rc].ToString();
				if (rc < m_Sights.m_aReticleColor.Count() - 1)
					json += ", ";
			}
			json += "],\n";
		}
		else
		{
			json += sub2 + "\"reticle_color\": null,\n";
		}

		if (m_Sights.m_bHasGlowColor && m_Sights.m_aGlowColor.Count() > 0)
		{
			json += sub2 + "\"glow_color\": [";
			for (int gc = 0; gc < m_Sights.m_aGlowColor.Count(); gc++)
			{
				json += m_Sights.m_aGlowColor[gc].ToString();
				if (gc < m_Sights.m_aGlowColor.Count() - 1)
					json += ", ";
			}
			json += "],\n";
		}
		else
		{
			json += sub2 + "\"glow_color\": null,\n";
		}

		if (m_Sights.m_bHasRangefinderSet)
		{
			string rfStr = "false";
			if (m_Sights.m_bHasRangefinder) rfStr = "true";
			json += sub2 + "\"has_rangefinder\": " + rfStr + "\n";
		}
		else
		{
			json += sub2 + "\"has_rangefinder\": null\n";
		}
		json += sub + "},\n";

		// 4. Physical
		json += sub + "\"physical\": {\n";
		if (m_Physical.m_bHasWeight)
			json += sub2 + "\"weight_kg\": " + m_Physical.m_fWeightKg.ToString() + ",\n";
		else
			json += sub2 + "\"weight_kg\": null,\n";

		if (m_Physical.m_bHasVolume)
			json += sub2 + "\"volume_cm3\": " + m_Physical.m_fVolumeCm3.ToString() + ",\n";
		else
			json += sub2 + "\"volume_cm3\": null,\n";

		if (m_Physical.m_bHasDimensions)
			json += sub2 + "\"dimensions\": [" + m_Physical.m_vDimensions[0].ToString() + ", " + m_Physical.m_vDimensions[1].ToString() + ", " + m_Physical.m_vDimensions[2].ToString() + "],\n";
		else
			json += sub2 + "\"dimensions\": null,\n";

		if (!m_Physical.m_sInventorySize.IsEmpty())
			json += sub2 + "\"inventory_size\": \"" + TBD_EquipmentExportJson.Escape(m_Physical.m_sInventorySize) + "\"\n";
		else
			json += sub2 + "\"inventory_size\": null\n";
		json += sub + "},\n";

		// 5. Visuals (3D Mesh)
		json += sub + "\"visuals\": {\n";
		if (!m_Visuals.m_sModelMesh.IsEmpty())
			json += sub2 + "\"model_mesh\": \"" + TBD_EquipmentExportJson.Escape(m_Visuals.m_sModelMesh) + "\"\n";
		else
			json += sub2 + "\"model_mesh\": null\n";
		json += sub + "}\n";

		json += indent + "}";
		return json;
	}
}
