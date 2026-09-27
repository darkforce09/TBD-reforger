/**
 * TBD_VehicleDeepSerializer.c
 *
 * Dedicated JSON serializer for the deep vehicle engineering platform.
 * Formats platforms, variants, drivetrain, armor hit zones, turrets, and reflection dumps
 * into clean, schema-compliant, indented JSON with zero mock data.
 */

class TBD_VehicleDeepSerializer
{
	//------------------------------------------------------------------------------------------------
	//! Serialize an entire platform and all of its deep variants to JSON.
	static string SerializePlatformToJson(TBD_VehicleDeepPlatform plat)
	{
		array<string> variants = {};
		foreach (TBD_VehicleDeepVariant variant : plat.m_aVariants) variants.Insert(SerializeVariantToJson(variant));
		string json = TBD_EquipmentExportJson.Envelope(variants.Count());
		json = TBD_EquipmentExportJson.Member(json, "platform", TBD_EquipmentExportJson.Quote(plat.m_sPlatformId));
		json = TBD_EquipmentExportJson.Member(json, "variants", "[" + TBD_EquipmentExportJson.Join(variants) + "]");
		return json;
	}

	static string SerializeAllToJson(array<ref TBD_VehicleDeepPlatform> platforms, int totalVariants)
	{
		array<string> entries = {};
		foreach (TBD_VehicleDeepPlatform platform : platforms) entries.Insert(SerializePlatformToJson(platform));
		return TBD_EquipmentExportJson.Member(TBD_EquipmentExportJson.Envelope(totalVariants), "platforms", "[\n" + TBD_EquipmentExportJson.Join(entries, ",\n") + "\n]");
	}

	//------------------------------------------------------------------------------------------------
	//! Serialize a single variant's complete engineering telemetry to JSON.
	static string SerializeVariantToJson(TBD_VehicleDeepVariant v)
	{
		TBD_EquipmentExportJson.IncludeResource(v.m_sResourceName);
		if (!v.m_sSerializedJson.IsEmpty()) return v.m_sSerializedJson;
		string json = "{}";
		json = TBD_EquipmentExportJson.Member(json, "resource_name", TBD_EquipmentExportJson.Quote(v.m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "resource_guid", TBD_EquipmentResourceNames.GuidJson(v.m_sResourceName));
		json = TBD_EquipmentExportJson.Member(json, "id", TBD_EquipmentExportJson.Quote(v.m_sId));
		json = TBD_EquipmentExportJson.Member(json, "parent_prefab", TBD_EquipmentExportJson.Quote(v.m_sParentPrefab));
		json = TBD_EquipmentExportJson.Member(json, "platform", TBD_EquipmentExportJson.Quote(v.m_sPlatform));
		json = TBD_EquipmentExportJson.Member(json, "addon", TBD_EquipmentExportJson.Quote(v.m_sAddonId));
		if (!v.m_sNamesJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "names", v.m_sNamesJson);
		if (!v.m_sInventoryJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "inventory", v.m_sInventoryJson);
		if (!v.m_sPhysicsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "physics", v.m_sPhysicsJson);
		if (!v.m_sDrivetrainJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "drivetrain", v.m_sDrivetrainJson);
		if (!v.m_sWaterJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "water_interaction", v.m_sWaterJson);
		if (!v.m_sFuelJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "fuel", v.m_sFuelJson);
		if (!v.m_sStorageJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "storage", v.m_sStorageJson);
		if (!v.m_sSupportJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "support", v.m_sSupportJson);
		if (!v.m_sOperationalSystemsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "operational_systems", v.m_sOperationalSystemsJson);
		if (!v.m_sAffiliationsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "affiliations", v.m_sAffiliationsJson);
		if (!v.m_sCommunicationsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "communications", v.m_sCommunicationsJson);
		if (!v.m_sElectricalJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "electrical", v.m_sElectricalJson);
		if (!v.m_sCrewJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "crew_stations", v.m_sCrewJson);
		if (!v.m_sTurretsJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "installations", v.m_sTurretsJson);
		if (!v.m_sDamageJson.IsEmpty()) json = TBD_EquipmentExportJson.Member(json, "damage", v.m_sDamageJson);
		v.m_sSerializedJson = TBD_EquipmentExportJson.Pretty(json, "    ");
		return v.m_sSerializedJson;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeDrivetrainBlock(TBD_VehicleDrivetrainData drivetrainData)
	{
		if (!drivetrainData) return "      \"drivetrain\": null,\n";

		string json = "      \"drivetrain\": {\n";
		json += "        \"simulationClass\": \"" + drivetrainData.m_sSimulationClass + "\",\n";
		json += "        \"driveConfiguration\": \"" + drivetrainData.m_sDriveConfiguration + "\",\n";
		json += "        \"wheelCount\": " + drivetrainData.m_iWheelCount.ToString() + ",\n";
		json += "        \"axleCount\": " + drivetrainData.m_iAxleCount.ToString() + ",\n";
		json += "        \"engine\": {\n";
		json += "          \"config\": \"" + drivetrainData.m_sEngineConfig + "\",\n";
		json += "          \"peakPowerKw\": " + drivetrainData.m_fEnginePeakPowerKw.ToString() + ",\n";
		json += "          \"peakPowerHp\": " + drivetrainData.m_fEnginePeakPowerHp.ToString() + ",\n";
		json += "          \"peakTorqueNm\": " + drivetrainData.m_fEnginePeakTorqueNm.ToString() + ",\n";
		json += "          \"maxRpm\": " + drivetrainData.m_fEngineMaxRpm.ToString() + ",\n";
		json += "          \"idleRpm\": " + drivetrainData.m_fEngineIdleRpm.ToString() + "\n";
		json += "        },\n";
		json += "        \"transmission\": {\n";
		json += "          \"forwardGears\": " + drivetrainData.m_iGearboxForwardGears.ToString() + ",\n";
		json += "          \"reverseGears\": " + drivetrainData.m_iGearboxReverseGears.ToString() + ",\n";
		json += "          \"gearboxEfficiency\": " + drivetrainData.m_fGearboxEfficiency.ToString() + ",\n";
		json += "          \"finalDriveRatio\": " + drivetrainData.m_fFinalDriveRatio.ToString() + ",\n";
		json += "          \"gearRatios\": [";
		for (int g = 0; g < drivetrainData.m_aGearRatios.Count(); g++)
		{
			json += drivetrainData.m_aGearRatios[g].ToString();
			if (g < drivetrainData.m_aGearRatios.Count() - 1) json += ", ";
		}
		json += "]\n";
		json += "        },\n";
		json += "        \"steering\": {\n";
		json += "          \"maxSteeringAngleDeg\": " + drivetrainData.m_fMaxSteeringAngleDeg.ToString() + ",\n";
		json += "          \"steeredAxles\": [";
		for (int a = 0; a < drivetrainData.m_aSteeredAxles.Count(); a++)
		{
			json += drivetrainData.m_aSteeredAxles[a].ToString();
			if (a < drivetrainData.m_aSteeredAxles.Count() - 1) json += ", ";
		}
		json += "]\n";
		json += "        }\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeSeatsBlock(array<ref TBD_VehicleSeatData> seats)
	{
		string json = "      \"seats\": [\n";
		for (int s = 0, sn = seats.Count(); s < sn; s++)
		{
			TBD_VehicleSeatData seat = seats[s];
			json += "        {\n";
			json += "          \"slotId\": " + seat.m_iSlotId.ToString() + ",\n";
			json += "          \"name\": \"" + seat.m_sName + "\",\n";
			json += "          \"type\": \"" + seat.m_sType + "\",\n";
			json += "          \"uiName\": \"" + TBD_VehicleExportJson.Escape(seat.m_sUIName) + "\",\n";
			json += "          \"door\": \"" + seat.m_sDoor + "\",\n";
			json += "          \"canTurnOut\": " + seat.m_bCanTurnOut.ToString() + "\n";
			json += "        }";
			if (s < sn - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeTurretsBlock(array<ref TBD_VehicleTurretData> turrets)
	{
		string json = "      \"turrets\": [\n";
		for (int t = 0, tn = turrets.Count(); t < tn; t++)
		{
			TBD_VehicleTurretData tur = turrets[t];
			json += "        {\n";
			json += "          \"slotName\": \"" + tur.m_sSlotName + "\",\n";
			json += "          \"turretPrefab\": \"" + tur.m_sTurretPrefab + "\",\n";
			json += "          \"display_name\": \"" + TBD_VehicleExportJson.Escape(tur.m_sDisplayName) + "\",\n";
			json += "          \"limits\": {\n";
			json += "            \"yawMin\": " + tur.m_fYawMin.ToString() + ",\n";
			json += "            \"yawMax\": " + tur.m_fYawMax.ToString() + ",\n";
			json += "            \"pitchMin\": " + tur.m_fPitchMin.ToString() + ",\n";
			json += "            \"pitchMax\": " + tur.m_fPitchMax.ToString() + ",\n";
			json += "            \"yawSpeedDegPerSec\": " + tur.m_fYawSpeed.ToString() + ",\n";
			json += "            \"pitchSpeedDegPerSec\": " + tur.m_fPitchSpeed.ToString() + "\n";
			json += "          },\n";
			json += "          \"optics\": [";
			for (int op = 0; op < tur.m_aOptics.Count(); op++)
			{
				json += "\"" + TBD_VehicleExportJson.Escape(tur.m_aOptics[op]) + "\"";
				if (op < tur.m_aOptics.Count() - 1) json += ", ";
			}
			json += "],\n";
			json += "          \"weapons\": [\n";
			for (int w = 0, wn = tur.m_aWeapons.Count(); w < wn; w++)
			{
				TBD_VehicleMountedWeaponData wep = tur.m_aWeapons[w];
				json += "            {\n";
				json += "              \"slotName\": \"" + wep.m_sSlotName + "\",\n";
				json += "              \"display_name\": \"" + TBD_VehicleExportJson.Escape(wep.m_sDisplayName) + "\",\n";
				json += "              \"weaponPrefab\": \"" + wep.m_sWeaponPrefab + "\",\n";
				json += "              \"weaponType\": \"" + wep.m_sWeaponType + "\",\n";
				json += "              \"defaultMagazine\": {\n";
				json += "                \"prefab\": \"" + wep.m_sDefaultMagazinePrefab + "\",\n";
				json += "                \"display_name\": \"" + TBD_VehicleExportJson.Escape(wep.m_sDefaultMagazineDisplayName) + "\",\n";
				json += "                \"capacity\": " + wep.m_iMagazineCapacity.ToString() + "\n";
				json += "              },\n";
				json += "              \"fireModes\": [";
				for (int fm = 0; fm < wep.m_aFireModes.Count(); fm++)
				{
					json += "\"" + wep.m_aFireModes[fm] + "\"";
					if (fm < wep.m_aFireModes.Count() - 1) json += ", ";
				}
				json += "],\n";
				json += "              \"zeroingDistances\": [";
				for (int zd = 0; zd < wep.m_aZeroingDistances.Count(); zd++)
				{
					json += "\"" + wep.m_aZeroingDistances[zd] + "\"";
					if (zd < wep.m_aZeroingDistances.Count() - 1) json += ", ";
				}
				json += "]\n";
				json += "            }";
				if (w < wn - 1) json += ",";
				json += "\n";
			}
			json += "          ]\n";
			json += "        }";
			if (t < tn - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeHitZonesBlock(array<ref TBD_VehicleHitZoneData> hitZones)
	{
		string json = "      \"hitZones\": [\n";
		for (int hz = 0, hzn = hitZones.Count(); hz < hzn; hz++)
		{
			TBD_VehicleHitZoneData hzd = hitZones[hz];
			json += "        {\n";
			json += "          \"name\": \"" + hzd.m_sName + "\",\n";
			json += "          \"group\": \"" + hzd.m_sGroup + "\",\n";
			json += "          \"maxHealth\": " + hzd.m_fMaxHealth.ToString() + ",\n";
			json += "          \"armorThicknessMm\": " + hzd.m_fArmorThickness.ToString() + "\n";
			json += "        }";
			if (hz < hzn - 1) json += ",";
			json += "\n";
		}
		json += "      ],\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeThresholdsBlock(TBD_VehicleDamageThresholds thresholdsData)
	{
		if (!thresholdsData) return "      \"damageThresholds\": null,\n";
		string json = "      \"damageThresholds\": {\n";
		json += "        \"destroyDamage\": " + thresholdsData.m_fVehicleDestroyDamage.ToString() + ",\n";
		json += "        \"collisionVelocity\": " + thresholdsData.m_fCollisionVelocityThreshold.ToString() + ",\n";
		json += "        \"heavyDamage\": " + thresholdsData.m_fHeavyDamageThreshold.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeStorageBlock(TBD_VehicleStorageData storageData)
	{
		if (!storageData) return "      \"storage\": null,\n";
		string json = "      \"storage\": {\n";
		json += "        \"volumeLiters\": " + storageData.m_fVolumeLiters.ToString() + ",\n";
		json += "        \"maxWeightKg\": " + storageData.m_fMaxWeightKg.ToString() + ",\n";
		json += "        \"supplyCapacity\": " + storageData.m_fSupplyCapacity.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeCommsBlock(TBD_VehicleCommsData commsData)
	{
		if (!commsData) return "      \"comms\": null,\n";
		string json = "      \"comms\": {\n";
		json += "        \"radio\": \"" + commsData.m_sRadioType + "\",\n";
		json += "        \"powerW\": " + commsData.m_fTransmitterPowerW.ToString() + ",\n";
		json += "        \"rangeKm\": " + commsData.m_fRangeKm.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeElectricalBlock(TBD_VehicleElectricalData electricalData)
	{
		if (!electricalData) return "      \"electrical\": null,\n";
		string json = "      \"electrical\": {\n";
		json += "        \"totalLightSlots\": " + electricalData.m_iTotalLightSlots.ToString() + ",\n";
		json += "        \"highBeamTurnsOffHeadlights\": " + electricalData.m_bHighBeamTurnsOffHeadlights.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeInstrumentsBlock(TBD_VehicleInstrumentationData instrumentsData)
	{
		if (!instrumentsData) return "      \"instrumentation\": null,\n";
		string json = "      \"instrumentation\": {\n";
		json += "        \"parameterCount\": " + instrumentsData.m_iInstrumentParameterCount.ToString() + ",\n";
		json += "        \"vehicleAnimGraph\": \"" + instrumentsData.m_sVehicleAnimGraph + "\"\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeAudioBlock(TBD_VehicleAudioData audioData)
	{
		if (!audioData) return "      \"audio\": null,\n";
		string json = "      \"audio\": {\n";
		json += "        \"soundPointsCount\": " + audioData.m_iSoundPointsCount.ToString() + "\n";
		json += "      },\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	protected static string SerializeRawVariablesBlock(map<string, ref array<string>> rawVars)
	{
		string json = "      \"rawComponentsDump\": {\n";
		int cIdx = 0;
		int total = rawVars.Count();
		foreach (string cName, array<string> cLines : rawVars)
		{
			json += "        \"" + cName + "\": [\n";
			for (int l = 0, ln = cLines.Count(); l < ln; l++)
			{
				json += "          \"" + TBD_VehicleExportJson.Escape(cLines[l]) + "\"";
				if (l < ln - 1) json += ",";
				json += "\n";
			}
			json += "        ]";
			if (cIdx < total - 1) json += ",";
			json += "\n";
			cIdx++;
		}
		json += "      }\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	//! Serialize summary matrix index to JSON.
	static string SerializeSummaryToJson(array<ref TBD_VehicleDeepPlatform> plats, int totalVariants)
	{
		array<string> entries = {};
		foreach (TBD_VehicleDeepPlatform platform : plats)
		{
			array<string> resources = {};
			foreach (TBD_VehicleDeepVariant variant : platform.m_aVariants) resources.Insert(variant.m_sResourceName);
			string json = "{}";
			json = TBD_EquipmentExportJson.Member(json, "platform", TBD_EquipmentExportJson.Quote(platform.m_sPlatformId));
			json = TBD_EquipmentExportJson.Member(json, "variant_count", resources.Count().ToString());
			json = TBD_EquipmentExportJson.Member(json, "resources", TBD_EquipmentExportJson.Strings(resources));
			entries.Insert(json);
		}
		return TBD_EquipmentExportJson.Pretty(TBD_EquipmentExportJson.Member(TBD_EquipmentExportJson.Envelope(totalVariants), "summary", "[" + TBD_EquipmentExportJson.Join(entries) + "]"));
	}
}
