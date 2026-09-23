/**
 * TBD_StaticWeaponModel.c
 *
 * Data model for static and crew-served weapons (mortars, tripod-mounted HMGs/GPMGs,
 * and standalone tripod mount frames).
 *
 * Introspects turret limits, optical sights, compartment crew positions, mounted armament,
 * and multi-part dismantle linkages.
 */

class TBD_StaticWeaponInfo
{
	// Identity
	string m_sResourceName;
	string m_sId;
	string m_sDisplayName;
	string m_sDescription;
	string m_sCategory;     // "mortars", "tripods", "mounts"
	string m_sFamily;       // "2B14", "M252", "M2HB", "NSV", "PKM", "M60"
	string m_sFaction;      // "US", "USSR", "FIA"
	string m_sFilePath;
	string m_sAddonId;
	bool   m_bIsAbstract;
	string m_sVariantOf;

	// Turret Specs
	bool   m_bHasTurret;
	float  m_fTraverseMin;
	float  m_fTraverseMax;
	float  m_fElevationMin;
	float  m_fElevationMax;
	float  m_fAimingMaxSpeed;
	string m_sSightsType;
	float  m_fMagnification;
	string m_sReticleTexture;
	bool   m_bHasIllumination;

	// Crew & Compartment
	bool   m_bHasCrewSlot;
	string m_sCompartmentSlot;
	vector m_vPassengerOffset;
	string m_sDefaultOccupantPrefab;

	// Armament
	bool   m_bHasArmament;
	bool   m_bIsIntegral;
	string m_sMountedWeaponTemplate;
	string m_sCaliber;
	string m_sMagazineWell;
	ref array<string> m_aSupportedAmmo = {};
	ref array<string> m_aInitialMagazines = {};

	// Deployment & Multi-part linkages
	bool   m_bIsDeployable;
	string m_sDismantleAction;
	string m_sReplacementBase;
	string m_sBarrelPrefab;
	string m_sBipodPrefab;

	// Physical
	float  m_fMassKg;
	string m_sDebrisModel;

	//------------------------------------------------------------------------------------------------
	string SerializeJson(string indent = "    ")
	{
		string s = indent + "{\n";
		string ind2 = indent + "  ";
		string ind3 = indent + "    ";

		// Identity
		s += ind2 + "\"resource_name\": \"" + TBD_EquipmentExportJson.Escape(m_sResourceName) + "\",\n";
		s += ind2 + "\"id\": \"" + TBD_EquipmentExportJson.Escape(m_sId) + "\",\n";
		s += ind2 + "\"display_name\": \"" + TBD_EquipmentExportJson.Escape(m_sDisplayName) + "\",\n";
		s += ind2 + "\"description\": \"" + TBD_EquipmentExportJson.Escape(m_sDescription) + "\",\n";
		s += ind2 + "\"category\": \"" + TBD_EquipmentExportJson.Escape(m_sCategory) + "\",\n";
		s += ind2 + "\"family\": \"" + TBD_EquipmentExportJson.Escape(m_sFamily) + "\",\n";
		s += ind2 + "\"faction\": \"" + TBD_EquipmentExportJson.Escape(m_sFaction) + "\",\n";
		s += ind2 + "\"addon\": \"" + TBD_EquipmentExportJson.Escape(m_sAddonId) + "\",\n";
		s += ind2 + "\"file_path\": \"" + TBD_EquipmentExportJson.Escape(m_sFilePath) + "\",\n";
		s += ind2 + "\"is_abstract\": " + m_bIsAbstract.ToString() + ",\n";
		if (!m_sVariantOf.IsEmpty())
			s += ind2 + "\"variant_of\": \"" + TBD_EquipmentExportJson.Escape(m_sVariantOf) + "\",\n";
		else
			s += ind2 + "\"variant_of\": null,\n";

		// Turret
		s += ind2 + "\"turret\": {\n";
		s += ind3 + "\"has_turret\": " + m_bHasTurret.ToString() + ",\n";
		s += ind3 + "\"traverse_limits\": [" + m_fTraverseMin.ToString() + ", " + m_fTraverseMax.ToString() + "],\n";
		s += ind3 + "\"elevation_limits\": [" + m_fElevationMin.ToString() + ", " + m_fElevationMax.ToString() + "],\n";
		s += ind3 + "\"aiming_max_speed\": " + m_fAimingMaxSpeed.ToString() + ",\n";
		s += ind3 + "\"sights\": {\n";
		s += ind3 + "  \"sights_type\": \"" + TBD_EquipmentExportJson.Escape(m_sSightsType) + "\",\n";
		s += ind3 + "  \"magnification\": " + m_fMagnification.ToString() + ",\n";
		s += ind3 + "  \"reticle_texture\": \"" + TBD_EquipmentExportJson.Escape(m_sReticleTexture) + "\",\n";
		s += ind3 + "  \"has_illumination\": " + m_bHasIllumination.ToString() + "\n";
		s += ind3 + "}\n";
		s += ind2 + "},\n";

		// Crew
		s += ind2 + "\"crew\": {\n";
		s += ind3 + "\"has_crew_slot\": " + m_bHasCrewSlot.ToString() + ",\n";
		s += ind3 + "\"compartment_slot\": \"" + TBD_EquipmentExportJson.Escape(m_sCompartmentSlot) + "\",\n";
		s += ind3 + "\"passenger_offset\": [" + m_vPassengerOffset[0].ToString() + ", " + m_vPassengerOffset[1].ToString() + ", " + m_vPassengerOffset[2].ToString() + "],\n";
		if (!m_sDefaultOccupantPrefab.IsEmpty())
			s += ind3 + "\"default_occupant_prefab\": \"" + TBD_EquipmentExportJson.Escape(m_sDefaultOccupantPrefab) + "\"\n";
		else
			s += ind3 + "\"default_occupant_prefab\": null\n";
		s += ind2 + "},\n";

		// Armament
		s += ind2 + "\"armament\": {\n";
		s += ind3 + "\"has_armament\": " + m_bHasArmament.ToString() + ",\n";
		s += ind3 + "\"is_integral\": " + m_bIsIntegral.ToString() + ",\n";
		if (!m_sMountedWeaponTemplate.IsEmpty())
			s += ind3 + "\"mounted_weapon_template\": \"" + TBD_EquipmentExportJson.Escape(m_sMountedWeaponTemplate) + "\",\n";
		else
			s += ind3 + "\"mounted_weapon_template\": null,\n";
		s += ind3 + "\"caliber\": \"" + TBD_EquipmentExportJson.Escape(m_sCaliber) + "\",\n";
		s += ind3 + "\"magazine_well\": \"" + TBD_EquipmentExportJson.Escape(m_sMagazineWell) + "\",\n";

		// Supported ammo
		s += ind3 + "\"supported_ammo\": [";
		for (int i = 0; i < m_aSupportedAmmo.Count(); i++)
		{
			s += "\"" + TBD_EquipmentExportJson.Escape(m_aSupportedAmmo[i]) + "\"";
			if (i < m_aSupportedAmmo.Count() - 1)
				s += ", ";
		}
		s += "],\n";

		// Initial magazines
		s += ind3 + "\"initial_magazines\": [";
		for (int j = 0; j < m_aInitialMagazines.Count(); j++)
		{
			s += "\"" + TBD_EquipmentExportJson.Escape(m_aInitialMagazines[j]) + "\"";
			if (j < m_aInitialMagazines.Count() - 1)
				s += ", ";
		}
		s += "]\n";
		s += ind2 + "},\n";

		// Deployment
		s += ind2 + "\"deployment\": {\n";
		s += ind3 + "\"is_deployable\": " + m_bIsDeployable.ToString() + ",\n";
		s += ind3 + "\"dismantle_action\": \"" + TBD_EquipmentExportJson.Escape(m_sDismantleAction) + "\",\n";
		s += ind3 + "\"parts\": {\n";
		if (!m_sReplacementBase.IsEmpty())
			s += ind3 + "  \"replacement_base\": \"" + TBD_EquipmentExportJson.Escape(m_sReplacementBase) + "\",\n";
		else
			s += ind3 + "  \"replacement_base\": null,\n";
		if (!m_sBarrelPrefab.IsEmpty())
			s += ind3 + "  \"barrel\": \"" + TBD_EquipmentExportJson.Escape(m_sBarrelPrefab) + "\",\n";
		else
			s += ind3 + "  \"barrel\": null,\n";
		if (!m_sBipodPrefab.IsEmpty())
			s += ind3 + "  \"bipod\": \"" + TBD_EquipmentExportJson.Escape(m_sBipodPrefab) + "\"\n";
		else
			s += ind3 + "  \"bipod\": null\n";
		s += ind3 + "}\n";
		s += ind2 + "},\n";

		// Physical
		s += ind2 + "\"physical\": {\n";
		s += ind3 + "\"mass_kg\": " + m_fMassKg.ToString() + ",\n";
		if (!m_sDebrisModel.IsEmpty())
			s += ind3 + "\"debris_model\": \"" + TBD_EquipmentExportJson.Escape(m_sDebrisModel) + "\"\n";
		else
			s += ind3 + "\"debris_model\": null\n";
		s += ind2 + "}\n";

		s += indent + "}";
		return s;
	}
}
