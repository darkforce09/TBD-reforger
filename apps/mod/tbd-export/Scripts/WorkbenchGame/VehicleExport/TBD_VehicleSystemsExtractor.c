/**
 * TBD_VehicleSystemsExtractor.c
 *
 * Dedicated extractor for electrical lighting, cockpit dashboard instruments, audio presets,
 * fuel systems, storage/cargo capacities, and communications radios.
 * Operates purely via Enfusion BaseContainer reflection without hardcoded vehicle tables.
 */

class TBD_VehicleSystemsExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Introspect electrical, instrumentation, audio, fuel, storage, and comms for a vehicle.
	static void Extract(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		ExtractElectrical(comps, varData);
		ExtractInstruments(comps, varData);
		ExtractAudio(comps, varData);
		ExtractFuel(comps, varData);
		ExtractStorage(comps, varData);
		ExtractComms(comps, varData);
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect BaseLightManagerComponent and light portals for lighting configuration.
	protected static void ExtractElectrical(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleElectricalData elec = new TBD_VehicleElectricalData();
		varData.m_Electrical = elec;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("LightManagerComponent")) continue;

			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer lm = bucket[b];
				if (!lm) continue;

				lm.Get("HighBeamTurnsOffHeadLights", elec.m_bHighBeamTurnsOffHeadlights);

				// Light slots
				BaseContainerList slots = lm.GetObjectArray("LightSlots");
				if (slots)
				{
					elec.m_iTotalLightSlots = slots.Count();
					for (int s = 0, sn = slots.Count(); s < sn; s++)
					{
						BaseContainer slot = slots.Get(s);
						if (!slot) continue;
						string slotName;
						if (slot.Get("m_sSlotName", slotName) && !slotName.IsEmpty())
							elec.m_aLightSlotNames.Insert(slotName);
					}
				}

				// Emissive surfaces (dash dials, backlight)
				BaseContainerList emissive = lm.GetObjectArray("EmissiveSurfaceSlots");
				if (emissive)
				{
					for (int e = 0, en = emissive.Count(); e < en; e++)
					{
						BaseContainer em = emissive.Get(e);
						if (!em) continue;
						string emName;
						if (em.Get("m_sSlotName", emName) && !emName.IsEmpty())
							elec.m_aEmissiveSurfaces.Insert(emName);
					}
				}
				if (elec.m_iTotalLightSlots > 0) break;
			}
			break;
		}
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
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("Fuel")) continue;

			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer fc = bucket[b];
				if (!fc) continue;

				float cap = 0;
				if (fc.Get("m_fFuelCapacity", cap) && cap > 0)
					varData.m_fFuelCapacityLiters = cap;
				else if (fc.Get("m_fMaxFuel", cap) && cap > 0)
					varData.m_fFuelCapacityLiters = cap;

				float cons = 0;
				if (fc.Get("m_fConsumptionRate", cons) && cons > 0)
					varData.m_fFuelConsumptionRate = cons;

				if (varData.m_fFuelCapacityLiters > 0) break;
			}
			if (varData.m_fFuelCapacityLiters > 0) break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect inventory storage volume, cargo weight limits, and supply resources.
	protected static void ExtractStorage(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleStorageData storageData = new TBD_VehicleStorageData();
		varData.m_Storage = storageData;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("InventoryStorage")) continue;

			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer sc = bucket[b];
				if (!sc) continue;

				float vol = 0;
				if (sc.Get("m_fMaxVolume", vol) && vol > 0)
					storageData.m_fVolumeLiters = vol;

				float maxWt = 0;
				if (sc.Get("m_fMaxWeight", maxWt) && maxWt > 0)
					storageData.m_fMaxWeightKg = maxWt;

				if (storageData.m_fVolumeLiters > 0) break;
			}
			if (storageData.m_fVolumeLiters > 0) break;
		}

		// Supply capacity from SCR_ResourceComponent
		foreach (string rcls, array<BaseContainer> rbucket : comps)
		{
			if (!rcls.Contains("SCR_ResourceComponent")) continue;

			for (int rb = 0; rb < rbucket.Count(); rb++)
			{
				BaseContainer rc = rbucket[rb];
				if (!rc) continue;

				float supp = 0;
				if (rc.Get("m_fMaxResourceValue", supp) && supp > 0)
				{
					storageData.m_fSupplyCapacity = supp;
					break;
				}
			}
			if (storageData.m_fSupplyCapacity > 0) break;
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Introspect radio transceivers for transmitter power and operational communication range.
	protected static void ExtractComms(map<string, ref array<BaseContainer>> comps, TBD_VehicleDeepVariant varData)
	{
		TBD_VehicleCommsData comms = new TBD_VehicleCommsData();
		varData.m_Comms = comms;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("Radio")) continue;

			for (int b = 0; b < bucket.Count(); b++)
			{
				BaseContainer rc = bucket[b];
				if (!rc) continue;

				float pwr = 0;
				if (rc.Get("m_fTransmitterPower", pwr) && pwr > 0)
					comms.m_fTransmitterPowerW = pwr;

				float rng = 0;
				if (rc.Get("m_fRange", rng) && rng > 0)
					comms.m_fRangeKm = rng;

				if (comms.m_sRadioType.IsEmpty())
					comms.m_sRadioType = rc.GetClassName();

				if (comms.m_fRangeKm > 0) break;
			}
			if (comms.m_fRangeKm > 0) break;
		}

		if (comms.m_sRadioType.IsEmpty())
			comms.m_sRadioType = "Standard Vehicle Radio";
	}
}
