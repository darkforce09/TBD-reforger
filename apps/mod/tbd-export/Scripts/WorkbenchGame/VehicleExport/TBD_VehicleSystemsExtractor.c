/**
 * TBD_VehicleSystemsExtractor.c
 *
 * Source-backed vehicle fuel, storage, communications and electrical configuration.
 * Domain sections preserve individual installations and native measurements.
 * Operates purely via Enfusion BaseContainer reflection without hardcoded vehicle tables.
 */

class TBD_VehicleSystemsExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extracts gameplay systems; presentation-only instrumentation and audio are not catalog sections.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		ExtractElectrical(comps, varData);
		ExtractFuel(comps, varData);
		ExtractStorage(comps, varData);
		ExtractComms(comps, varData);
		ExtractOperationalSystems(comps, varData);
		TBD_ItemToolInfo support = new TBD_ItemToolInfo();
		TBD_ItemExtractor.ExtractTool(comps, support, varData.m_sFilePath, "/support");
		varData.m_sSupportJson = support.m_sJson;
		array<string> affiliations = {};
		foreach (string className, array<BaseContainer> instances : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(className, "FactionAffiliationComponent")) continue;
			foreach (BaseContainer instance : instances)
			{
				if (TBD_EquipmentComponentGraph.StructuralPath(instance).Contains("/children/")) continue;
				string path = "/affiliations/" + affiliations.Count().ToString();
				affiliations.Insert(SourceFields(instance, "faction=faction affiliation", path));
			}
		}
		if (!affiliations.IsEmpty()) varData.m_sAffiliationsJson = "[" + TBD_EquipmentExportJson.Join(affiliations) + "]";
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect BaseLightManagerComponent and light portals for lighting configuration.
	protected static void ExtractElectrical(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		array<string> managers = {};
		array<string> power = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_PowerComponent"))
			{
				foreach (BaseContainer supply : bucket)
				{
					string powerPath = "/electrical/power/" + power.Count().ToString();
					power.Insert(SourceFields(supply, "enabled=Enabled|apply_no_power_penalty=m_bShouldApplyNoPowerPenalty|no_power_multiplier=m_fNoPowerMultiplier", powerPath));
				}
			}
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseLightManagerComponent")) continue;
			foreach (BaseContainer manager : bucket)
			{
				string path = "/electrical/light_managers/" + managers.Count().ToString();
				string entry = SourceFields(manager, "high_beam_turns_off_headlights=HighBeamTurnsOffHeadLights", path);
				string slots = SourceArray(manager, "LightSlots", "light_type=LightType|light_state=LightState|light_side=LightSide|is_presence_light=IsPresenceLight", path + "/light_slots");
				entry = TBD_EquipmentExportJson.Member(entry, "light_slots", slots);
				managers.Insert(entry);
			}
		}
		if (!managers.IsEmpty() || !power.IsEmpty())
			varData.m_sElectricalJson = "{\"light_managers\":[" + TBD_EquipmentExportJson.Join(managers) + "],\"power\":[" + TBD_EquipmentExportJson.Join(power) + "]}";
	}

	//! Preserve configured access, AI detection and weapon-selection behavior per component instance.
	protected static void ExtractOperationalSystems(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		array<string> locks = {};
		array<string> perception = {};
		array<string> weapons = {};
		array<string> protection = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer component : bucket)
			{
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_BaseLockComponent"))
					locks.Insert(SourceFields(component, "enabled=Enabled|initially_locked=m_bIsLocked", "/operational_systems/locks/" + locks.Count().ToString()));
				if (TBD_EquipmentComponentGraph.IsA(cls, "VehiclePerceivableComponent"))
				{
					string fields = "illumination_per_light=IlluminationLvPerLight|unit_type=UnitType|sound_power_rpm_multiplier=SoundPowerRpmMultiplierDb|horn_sound_power=HornSoundPowerDb";
					string path = "/operational_systems/perception/" + perception.Count().ToString();
					string entry = SourceFields(component, fields, path);
					perception.Insert(TBD_EquipmentExportJson.Member(entry, "aim_points", ReadAimPoints(component, path + "/aim_points")));
				}
				if (TBD_EquipmentComponentGraph.IsA(cls, "BaseWeaponManagerComponent"))
					weapons.Insert(SourceFields(component, "default_weapon_index=DefaultWeaponIndex", "/operational_systems/weapon_managers/" + weapons.Count().ToString()));
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_VehicleSpawnProtectionComponent"))
					protection.Insert(SourceFields(component, "enabled=Enabled|reason_text=m_sReasonText", "/operational_systems/spawn_protection/" + protection.Count().ToString()));
			}
		}
		string json = "{\"locks\":[" + TBD_EquipmentExportJson.Join(locks) + "],\"perception\":[" + TBD_EquipmentExportJson.Join(perception) + "]}";
		json = TBD_EquipmentExportJson.Member(json, "weapon_managers", "[" + TBD_EquipmentExportJson.Join(weapons) + "]");
		json = TBD_EquipmentExportJson.Member(json, "ai", TBD_VehicleAIConfigurationExtractor.Extract(comps));
		varData.m_sOperationalSystemsJson = TBD_EquipmentExportJson.Member(json, "spawn_protection", "[" + TBD_EquipmentExportJson.Join(protection) + "]");
	}

	//! Preserve AI target selection, visibility checks and authored attachment positions in source order.
	protected static string ReadAimPoints(BaseContainer owner, string path)
	{
		BaseContainerList points = owner.GetObjectArray("Additional aim points");
		if (!points) return "null";
		array<string> entries = {};
		for (int i = 0; i < points.Count(); i++)
		{
			BaseContainer point = points.Get(i);
			if (!point) { entries.Insert("null"); continue; }
			string pointPath = path + "/" + i.ToString();
			string json = SourceFields(point, "aimpoint_checked=AimpointChecked|visibility_checked=VsibilityChecked|dimension=Dimension|aim_point_type=AimPointType", pointPath);
			BaseContainer position = point.GetObject("AimPointPosition");
			string coordinates = SourceFields(position, "bone=Bone|offset=Offset|angles=Angles", pointPath + "/position");
			entries.Insert(TBD_EquipmentExportJson.Member(json, "position", coordinates));
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect CarProcAnimComponent parameters and anim graphs for cockpit instruments.
	protected static void ExtractInstruments(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleInstrumentationData inst = new TBD_VehicleInstrumentationData();
		varData.m_Instruments = inst;

		// 1. Procedural animation parameters (speedometer, tachometer, fuel needles)
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("CarProcAnimComponent") && !cls.Contains("ProcAnimComponent")) continue;

			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer pac = bucket[b];
				if (!pac) continue;

				BaseContainerList params = pac.GetObjectArray("Parameters");
				if (params && params.Count() > 0)
				{
					inst.m_iInstrumentParameterCount = params.Count();
					for (int p = 0, pn = params.Count(); p < pn; p++)
					{
						BaseContainer param = params.Get(p);
						if (!param) continue;
						string pName;
						if (param.Get("Name", pName) && !pName.IsEmpty())
							inst.m_aInstrumentParameters.Insert(pName);
					}
					break;
				}
			}
			break;
		}

		// 2. Vehicle animation workspaces
		foreach (string acls, array<BaseContainer> abucket : comps)
		{
			if (!acls.Contains("VehicleAnimationComponent")) continue;

			for (int ab = 0; ab < abucket.Count(); ab++)
			{
				BaseContainer vac = abucket[ab];
				if (!vac) continue;

				if (inst.m_sVehicleAnimGraph.IsEmpty())
					vac.Get("AnimGraph", inst.m_sVehicleAnimGraph);

				string animInj;
				if (inst.m_sPlayerAnimGraph.IsEmpty() && vac.Get("AnimInjection", animInj) && !animInj.IsEmpty())
				{
					int graphIdx = animInj.IndexOf("AnimGraph \"");
					if (graphIdx != -1)
					{
						int endQ = animInj.IndexOfFrom(graphIdx + 11, "\"");
						if (endQ != -1)
							inst.m_sPlayerAnimGraph = animInj.Substring(graphIdx + 11, endQ - graphIdx - 11);
					}
				}
				if (!inst.m_sVehicleAnimGraph.IsEmpty()) break;
			}
			break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect SCR_VehicleSoundComponent for sound point positions and engine audio signals.
	protected static void ExtractAudio(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleAudioData audio = new TBD_VehicleAudioData();
		varData.m_Audio = audio;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("SoundComponent")) continue;

			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer sc = bucket[b];
				if (!sc) continue;

				BaseContainerList spList = sc.GetObjectArray("SoundPoints");
				if (spList && spList.Count() > 0)
				{
					audio.m_iSoundPointsCount = spList.Count();
					for (int p = 0, pn = spList.Count(); p < pn; p++)
					{
						BaseContainer sp = spList.Get(p);
						if (!sp) continue;
						string spName;
						if (sp.Get("PointName", spName) && !spName.IsEmpty())
							audio.m_aSoundPoints.Insert(spName);
					}
				}

				BaseContainerList sigList = sc.GetObjectArray("m_aNormalizeSignalData");
				if (sigList)
				{
					for (int s = 0, sn = sigList.Count(); s < sn; s++)
					{
						BaseContainer sig = sigList.Get(s);
						if (!sig) continue;
						string sigName;
						if (sig.Get("m_sSignalName", sigName) && !sigName.IsEmpty())
							audio.m_aSoundSignals.Insert(sigName);
					}
				}
				if (audio.m_iSoundPointsCount > 0) break;
			}
			break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect fuel node and tank components for capacity and operational consumption.
	protected static void ExtractFuel(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		array<string> managers = {};
		array<string> consumers = {};
		array<BaseContainer> tanks = {};
		array<BaseContainer> consumptionComponents = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			foreach (BaseContainer component : bucket)
			{
				if (!component) continue;
				if (TBD_EquipmentComponentGraph.IsA(cls, "SCR_FuelConsumptionComponent"))
				{
					string consumptionPath = "/fuel/consumption/" + consumers.Count().ToString();
					consumers.Insert(SourceFields(component, "fuel_consumption=m_fFuelConsumption|idle_fuel_consumption=m_fFuelConsumptionIdle", consumptionPath));
					consumptionComponents.Insert(component);
				}
				if (!TBD_EquipmentComponentGraph.IsA(cls, "FuelManagerComponent")) continue;
				string managerPath = "/fuel/managers/" + managers.Count().ToString();
				string entry = "{\"source\":" + TBD_EquipmentExportJson.Context(component) + "}";
				entry = TBD_EquipmentExportJson.Member(entry, "tanks", SourceArray(component, "FuelNodes", "max_fuel=MaxFuel|initial_fuel=m_fInitialFuelTankState", managerPath + "/tanks"));
				managers.Insert(entry);
				BaseContainerList nodes = component.GetObjectArray("FuelNodes");
				if (!nodes) continue;
				for (int i = 0; i < nodes.Count(); i++)
				{
					BaseContainer node = nodes.Get(i);
					if (node) tanks.Insert(node);
				}
			}
		}
		if (!managers.IsEmpty() || !consumers.IsEmpty())
		{
			varData.m_sFuelJson = "{\"managers\":[" + TBD_EquipmentExportJson.Join(managers) + "]";
			varData.m_sFuelJson += ",\"consumption\":[" + TBD_EquipmentExportJson.Join(consumers) + "]}";
		}
		// Scalar summaries are populated only when one source supplies the value.
		if (tanks.Count() == 1) tanks[0].Get("MaxFuel", varData.m_fFuelCapacityLiters);
		if (consumptionComponents.Count() == 1) consumptionComponents[0].Get("m_fFuelConsumption", varData.m_fFuelConsumptionRate);
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect inventory storage volume, cargo weight limits, and supply resources.
	protected static void ExtractStorage(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		varData.m_sStorageJson = TBD_ItemInventoryExtractor.Storage(comps);
		TBD_VehicleStorageData storageData = new TBD_VehicleStorageData();
		varData.m_Storage = storageData;
		array<BaseContainer> storages = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseInventoryStorageComponent")) continue;
			foreach (BaseContainer storage : bucket)
			{
				if (storage && storage.GetVarIndex("MaxCumulativeVolume") >= 0)
					storages.Insert(storage);
			}
		}
		if (storages.Count() == 1)
		{
			storages[0].Get("MaxCumulativeVolume", storageData.m_fVolumeLiters);
			storages[0].Get("m_fMaxWeight", storageData.m_fMaxWeightKg);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Preserves radios and individual transceiver frequency/range settings without unit conversion.
	protected static void ExtractComms(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleCommsData comms = new TBD_VehicleCommsData();
		varData.m_Comms = comms;
		array<string> radios = {};
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!TBD_EquipmentComponentGraph.IsA(cls, "BaseRadioComponent")) continue;
			foreach (BaseContainer radio : bucket)
			{
				if (!radio) continue;
				if (comms.m_sRadioType.IsEmpty()) comms.m_sRadioType = radio.GetClassName();
				string path = "/communications/radios/" + radios.Count().ToString();
				string entry = SourceFields(radio, "encryption_key=Encryption key|turned_on=Turned on", path);
				string bindings = "configured_frequency=ChannelFrequency|minimum_frequency=Min tunable frequency|maximum_frequency=Max tunable frequency";
				bindings += "|frequency_step=Frequency resolution|transmission_range=Transmitting Range";
				entry = TBD_EquipmentExportJson.Member(entry, "transceivers", SourceArray(radio, "Transceivers", bindings, path + "/transceivers"));
				radios.Insert(entry);
			}
		}
		if (!radios.IsEmpty()) varData.m_sCommunicationsJson = "{\"radios\":[" + TBD_EquipmentExportJson.Join(radios) + "]}";
	}

	//! Adds native instance context to explicitly selected source properties.
	protected static string SourceFields(BaseContainer source, string bindings, string path)
	{
		if (!source) return "null";
		string value = TBD_EquipmentExportJson.Fields(source, bindings, path);
		return TBD_EquipmentExportJson.Member(value, "source", TBD_EquipmentExportJson.Context(source));
	}

	//! Keeps ordered nested configurations and their identities without fabricating entries.
	protected static string SourceArray(BaseContainer source, string property, string bindings, string path)
	{
		if (!source || source.GetVarIndex(property) < 0) return "null";
		BaseContainerList objects = source.GetObjectArray(property);
		if (!objects)
		{
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "Cannot read object array " + property);
			return "null";
		}
		array<string> entries = {};
		for (int i = 0; i < objects.Count(); i++)
			entries.Insert(SourceFields(objects.Get(i), bindings, path + "/" + i.ToString()));
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}
}
