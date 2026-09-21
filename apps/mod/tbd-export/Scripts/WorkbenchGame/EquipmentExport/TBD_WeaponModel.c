//------------------------------------------------------------------------------------------------
// TBD_WeaponModel.c
//
// Strong data models representing weapon entities, ballistics, zeroing, muzzles, fire modes,
// and attachment slot specifications for the TBD Reforger Universal Weapon Export System.
//
// Pure intrinsic keys: exports pure foreign keys (magazine_wells, required_attachment_type,
// obstructed_attachment_types) without pre-computing candidate compatibility arrays.
//------------------------------------------------------------------------------------------------

class TBD_WeaponFireModeInfo
{
	string m_sName;
	string m_sFireModeType; // e.g. "Semiauto", "Auto", "Burst", "Safety", "Manual"
	int m_iBurstCount = 1; // 1 = semi, 0 = auto, 3 = burst
	int m_iRoundsPerMinute = 0;
	string m_sBurstType; // e.g. "Interruptable", "Uninterruptable", "InterruptableAndResetting"
}

class TBD_WeaponMuzzleInfo
{
	int m_iIndex = 0;
	string m_sMuzzleClass;
	ref array<string> m_aMagazineWells = {};
	string m_sDefaultMagazineTemplate;
	bool m_bIsDisposable = false;
	ref array<ref TBD_WeaponFireModeInfo> m_aFireModes = {};
}

class TBD_WeaponAttachmentSlotInfo
{
	string m_sSlotName;
	string m_sPivotId;
	string m_sRequiredAttachmentType;
	string m_sDefaultAttachedPrefab;
	ref array<string> m_aObstructedAttachmentTypes = {};
}

class TBD_WeaponSightsInfo
{
	string m_sSightsType;
	ref array<string> m_aZeroingDistances = {};
	int m_iDefaultZeroIndex = 0;
	string m_sRearPivot;
	string m_sFrontPivot;
}

class TBD_WeaponBallisticsInfo
{
	float m_fInitSpeedCoef = 1.0;
	float m_fDispersionDiameter = -1.0;
	float m_fDispersionRange = -1.0;
}

class TBD_WeaponPhysicalInfo
{
	float m_fWeightKg = -1.0;
	float m_fVolumeCm3 = -1.0;
	vector m_vDimensions = "0 0 0";
	string m_sInventorySize;
	bool m_bHasBipod = false;
	bool m_bIsDisposable = false;
}

class TBD_WeaponClassificationInfo
{
	string m_sWeaponType;
	string m_sWeaponSlotType;
}

class TBD_WeaponInfo
{
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sIcon;
	string m_sCategory; // "rifles", "machine_guns", "handguns", "launchers", "grenades", "explosives", "underbarrel"
	string m_sFamily;
	string m_sAddonId;
	string m_sFilePath;
	bool m_bIsAbstract = false;
	string m_sVariantOf;

	ref TBD_WeaponClassificationInfo m_Classification = new TBD_WeaponClassificationInfo();
	ref TBD_WeaponPhysicalInfo m_Physical = new TBD_WeaponPhysicalInfo();
	ref TBD_WeaponSightsInfo m_Sights = new TBD_WeaponSightsInfo();
	ref TBD_WeaponBallisticsInfo m_Ballistics = new TBD_WeaponBallisticsInfo();
	ref array<ref TBD_WeaponMuzzleInfo> m_aMuzzles = {};
	ref array<ref TBD_WeaponAttachmentSlotInfo> m_aAttachmentSlots = {};

	//------------------------------------------------------------------------------------------------
	//! Serialize weapon entry to clean, indented JSON string
	string SerializeJson(string indent = "    ")
	{
		string json = indent + "{\n";
		string sub = indent + "  ";
		string sub2 = sub + "  ";
		string sub3 = sub2 + "  ";

		// Identity
		json += sub + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		json += sub + "\"id\": \"" + TBD_EquipmentExportJson.Escape(m_sId) + "\",\n";
		json += sub + "\"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";

		if (!m_sDescription.IsEmpty())
			json += sub + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		else
			json += sub + "\"description\": null,\n";

		if (!m_sIcon.IsEmpty())
			json += sub + "\"icon\": \"" + TBD_EquipmentExportJson.Escape(m_sIcon) + "\",\n";
		else
			json += sub + "\"icon\": null,\n";

		json += sub + "\"category\": \"" + TBD_EquipmentExportJson.Escape(m_sCategory) + "\",\n";
		json += sub + "\"family\": \"" + TBD_EquipmentExportJson.Escape(m_sFamily) + "\",\n";
		json += sub + "\"addon\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		json += sub + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";

		string isAbsStr = "false";
		if (m_bIsAbstract) isAbsStr = "true";
		json += sub + "\"is_abstract\": " + isAbsStr + ",\n";

		if (!m_sVariantOf.IsEmpty())
			json += sub + "\"variant_of\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			json += sub + "\"variant_of\": null,\n";

		// Classification
		json += sub + "\"classification\": {\n";
		json += sub2 + "\"weapon_type\": \"" + TBD_EquipmentExportJson.Escape(m_Classification.m_sWeaponType) + "\",\n";
		json += sub2 + "\"weapon_slot_type\": \"" + TBD_EquipmentExportJson.Escape(m_Classification.m_sWeaponSlotType) + "\"\n";
		json += sub + "},\n";

		// Physical
		json += sub + "\"physical\": {\n";
		if (m_Physical.m_fWeightKg >= 0)
			json += sub2 + "\"weight_kg\": " + m_Physical.m_fWeightKg.ToString() + ",\n";
		else
			json += sub2 + "\"weight_kg\": null,\n";

		if (m_Physical.m_fVolumeCm3 >= 0)
			json += sub2 + "\"volume_cm3\": " + m_Physical.m_fVolumeCm3.ToString() + ",\n";
		else
			json += sub2 + "\"volume_cm3\": null,\n";

		json += sub2 + "\"dimensions\": [" + m_Physical.m_vDimensions[0].ToString() + ", " + m_Physical.m_vDimensions[1].ToString() + ", " + m_Physical.m_vDimensions[2].ToString() + "],\n";

		if (!m_Physical.m_sInventorySize.IsEmpty())
			json += sub2 + "\"inventory_size\": \"" + TBD_EquipmentExportJson.Escape(m_Physical.m_sInventorySize) + "\",\n";
		else
			json += sub2 + "\"inventory_size\": null,\n";

		string hasBipodStr = "false";
		if (m_Physical.m_bHasBipod) hasBipodStr = "true";
		json += sub2 + "\"has_bipod\": " + hasBipodStr + ",\n";

		string isDispStr = "false";
		if (m_Physical.m_bIsDisposable) isDispStr = "true";
		json += sub2 + "\"is_disposable\": " + isDispStr + "\n";
		json += sub + "},\n";

		// Sights
		json += sub + "\"sights\": {\n";
		if (!m_Sights.m_sSightsType.IsEmpty())
			json += sub2 + "\"sights_type\": \"" + TBD_EquipmentExportJson.Escape(m_Sights.m_sSightsType) + "\",\n";
		else
			json += sub2 + "\"sights_type\": null,\n";

		json += sub2 + "\"zeroing_distances\": [";
		for (int zd = 0; zd < m_Sights.m_aZeroingDistances.Count(); zd++)
		{
			json += "\"" + m_Sights.m_aZeroingDistances[zd] + "\"";
			if (zd < m_Sights.m_aZeroingDistances.Count() - 1)
				json += ", ";
		}
		json += "],\n";

		json += sub2 + "\"default_zero_index\": " + m_Sights.m_iDefaultZeroIndex.ToString() + ",\n";

		if (!m_Sights.m_sRearPivot.IsEmpty())
			json += sub2 + "\"rear_pivot\": \"" + TBD_EquipmentExportJson.Escape(m_Sights.m_sRearPivot) + "\",\n";
		else
			json += sub2 + "\"rear_pivot\": null,\n";

		if (!m_Sights.m_sFrontPivot.IsEmpty())
			json += sub2 + "\"front_pivot\": \"" + TBD_EquipmentExportJson.Escape(m_Sights.m_sFrontPivot) + "\"\n";
		else
			json += sub2 + "\"front_pivot\": null\n";

		json += sub + "},\n";

		// Ballistics
		json += sub + "\"ballistics\": {\n";
		json += sub2 + "\"init_speed_coef\": " + m_Ballistics.m_fInitSpeedCoef.ToString() + ",\n";
		if (m_Ballistics.m_fDispersionDiameter >= 0)
			json += sub2 + "\"dispersion_diameter\": " + m_Ballistics.m_fDispersionDiameter.ToString() + ",\n";
		else
			json += sub2 + "\"dispersion_diameter\": null,\n";

		if (m_Ballistics.m_fDispersionRange >= 0)
			json += sub2 + "\"dispersion_range\": " + m_Ballistics.m_fDispersionRange.ToString() + "\n";
		else
			json += sub2 + "\"dispersion_range\": null\n";
		json += sub + "},\n";

		// Muzzles
		json += sub + "\"muzzles\": [\n";
		for (int m = 0; m < m_aMuzzles.Count(); m++)
		{
			TBD_WeaponMuzzleInfo muz = m_aMuzzles[m];
			json += sub2 + "{\n";
			json += sub3 + "\"index\": " + muz.m_iIndex.ToString() + ",\n";
			json += sub3 + "\"muzzle_class\": \"" + muz.m_sMuzzleClass + "\",\n";

			// Magazine wells
			json += sub3 + "\"magazine_wells\": [";
			for (int mw = 0; mw < muz.m_aMagazineWells.Count(); mw++)
			{
				json += "\"" + muz.m_aMagazineWells[mw] + "\"";
				if (mw < muz.m_aMagazineWells.Count() - 1)
					json += ", ";
			}
			json += "],\n";

			if (!muz.m_sDefaultMagazineTemplate.IsEmpty())
				json += sub3 + "\"default_magazine\": \"" + TBD_EquipmentExportJson.Escape(muz.m_sDefaultMagazineTemplate) + "\",\n";
			else
				json += sub3 + "\"default_magazine\": null,\n";

			string muzDispStr = "false";
			if (muz.m_bIsDisposable) muzDispStr = "true";
			json += sub3 + "\"is_disposable\": " + muzDispStr + ",\n";

			// Fire modes
			json += sub3 + "\"fire_modes\": [\n";
			for (int f = 0; f < muz.m_aFireModes.Count(); f++)
			{
				TBD_WeaponFireModeInfo fmi = muz.m_aFireModes[f];
				json += sub3 + "  {\n";
				json += sub3 + "    \"name\": \"" + TBD_EquipmentExportJson.Escape(fmi.m_sName) + "\",\n";
				json += sub3 + "    \"fire_mode_type\": \"" + TBD_EquipmentExportJson.Escape(fmi.m_sFireModeType) + "\",\n";
				json += sub3 + "    \"burst\": " + fmi.m_iBurstCount.ToString() + ",\n";
				json += sub3 + "    \"rpm\": " + fmi.m_iRoundsPerMinute.ToString() + ",\n";
				json += sub3 + "    \"burst_type\": \"" + TBD_EquipmentExportJson.Escape(fmi.m_sBurstType) + "\"\n";
				json += sub3 + "  }";
				if (f < muz.m_aFireModes.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += sub3 + "]\n";

			json += sub2 + "}";
			if (m < m_aMuzzles.Count() - 1)
				json += ",";
			json += "\n";
		}
		json += sub + "],\n";

		// Attachment slots
		json += sub + "\"attachment_slots\": [\n";
		for (int s = 0; s < m_aAttachmentSlots.Count(); s++)
		{
			TBD_WeaponAttachmentSlotInfo slot = m_aAttachmentSlots[s];
			json += sub2 + "{\n";
			json += sub3 + "\"slot_name\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sSlotName) + "\",\n";
			json += sub3 + "\"pivot_id\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sPivotId) + "\",\n";
			json += sub3 + "\"required_attachment_type\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sRequiredAttachmentType) + "\",\n";

			if (!slot.m_sDefaultAttachedPrefab.IsEmpty())
				json += sub3 + "\"default_attachment\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sDefaultAttachedPrefab) + "\",\n";
			else
				json += sub3 + "\"default_attachment\": null,\n";

			json += sub3 + "\"obstructed_attachment_types\": [";
			for (int ob = 0; ob < slot.m_aObstructedAttachmentTypes.Count(); ob++)
			{
				json += "\"" + slot.m_aObstructedAttachmentTypes[ob] + "\"";
				if (ob < slot.m_aObstructedAttachmentTypes.Count() - 1)
					json += ", ";
			}
			json += "]\n";

			json += sub2 + "}";
			if (s < m_aAttachmentSlots.Count() - 1)
				json += ",";
			json += "\n";
		}
		json += sub + "]\n";

		json += indent + "}";
		return json;
	}
}
