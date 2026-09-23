/**
 * TBD_ItemExtractor.c
 *
 * Introspects component graphs across prefab ancestry to extract:
 * 1. Physical weight, volume, dimensions, and inventory UI size (InventoryItemComponent).
 * 2. Medical consumable type and healing effect values (SCR_ConsumableItemComponent).
 * 3. Radio transceivers, frequency ranges, and encryption (BaseRadioComponent).
 * 4. Gadget classification, optics magnification, and FOV (SCR_GadgetComponent, SCR_BinocularsComponent).
 * 5. Explosives, mines, pressure triggers, and blasting detonators (SCR_MineComponent, SCR_ExplosiveChargeComponent).
 * 6. Engineering, demining, and maintenance tools (SCR_ToolComponent, SCR_MineFlagComponent).
 * 7. Survival gear, canteens, rations, and field containers.
 */

class TBD_ItemExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: weight, volume, dimensions, and inventory UI size.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_ItemPhysicalInfo outPhys)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("StorageComponent"))
				continue;

			foreach (BaseContainer comp : bucket)
			{
				BaseContainer curComp = comp;
				while (curComp)
				{
					BaseContainer attrs = curComp.GetObject("Attributes");
					if (attrs)
					{
						BaseContainer curAttrs = attrs;
						while (curAttrs)
						{
							if (outPhys.m_sInventorySize.IsEmpty())
							{
								string sz;
								if (curAttrs.Get("m_Size", sz) && !sz.IsEmpty())
									outPhys.m_sInventorySize = sz;
							}

							BaseContainer phys = curAttrs.GetObject("ItemPhysAttributes");
							if (phys)
							{
								BaseContainer curPhys = phys;
								while (curPhys)
								{
									if (outPhys.m_fWeightKg < 0)
									{
										float w;
										if (curPhys.Get("Weight", w) && w >= 0)
											outPhys.m_fWeightKg = w;
									}

									if (outPhys.m_fVolumeCm3 < 0)
									{
										float v;
										if (curPhys.Get("ItemVolume", v) && v >= 0)
											outPhys.m_fVolumeCm3 = v;
									}

									if (outPhys.m_vDimensions == "0 0 0")
									{
										vector dims;
										if (curPhys.Get("ItemDimensions", dims) && dims != "0 0 0")
											outPhys.m_vDimensions = dims;
									}

									curPhys = curPhys.GetAncestor();
								}
							}

							curAttrs = curAttrs.GetAncestor();
						}
					}

					curComp = curComp.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract medical properties from consumable components and effects.
	static void ExtractMedical(map<string, ref array<BaseContainer>> comps, TBD_ItemMedicalInfo outMed, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("ConsumableItemComponent"))
				continue;

			outMed.m_bIsMedical = true;

			foreach (BaseContainer comp : bucket)
			{
				BaseContainer cur = comp;
				while (cur)
				{
					BaseContainer effectObj = cur.GetObject("m_ConsumableEffect");
					if (effectObj)
					{
						string effectClass = effectObj.GetClassName();
						if (effectClass == "SCR_ConsumableTourniquet")
							outMed.m_sConsumableType = "TOURNIQUET";
						else if (effectClass == "SCR_ConsumableBandage")
							outMed.m_sConsumableType = "BANDAGE";
						else if (effectClass == "SCR_ConsumableMorphine")
							outMed.m_sConsumableType = "MORPHINE";
						else if (effectClass == "SCR_ConsumableSalineBag")
							outMed.m_sConsumableType = "SALINE";

						float regenSpeed = 0, regenTotal = 0;
						if (outMed.m_fEffectValue < 0)
						{
							if (effectObj.Get("m_fItemAbsoluteRegenerationAmount", regenTotal) && regenTotal > 0)
								outMed.m_fEffectValue = regenTotal;
							else if (effectObj.Get("m_fItemRegenerationSpeed", regenSpeed) && regenSpeed > 0)
								outMed.m_fEffectValue = regenSpeed;
						}
					}

					BaseContainer consumableTypeObj = cur.GetObject("m_ConsumableType");
					if (consumableTypeObj)
					{
						int typeInt = -1;
						if (outMed.m_sConsumableType.IsEmpty() && consumableTypeObj.Get("m_eConsumableType", typeInt) && typeInt >= 0)
							outMed.m_sConsumableType = ConsumableTypeToString(typeInt);

						float val;
						if (outMed.m_fEffectValue < 0 && consumableTypeObj.Get("m_fValue", val) && val >= 0)
							outMed.m_fEffectValue = val;
					}

					cur = cur.GetAncestor();
				}
			}
		}

		if (outMed.m_sConsumableType.IsEmpty())
		{
			if (filePath.Contains("Tourniquet")) outMed.m_sConsumableType = "TOURNIQUET";
			else if (filePath.Contains("FieldDressing") || filePath.Contains("Gauze") || filePath.Contains("Bandage")) outMed.m_sConsumableType = "BANDAGE";
			else if (filePath.Contains("Morphine")) outMed.m_sConsumableType = "MORPHINE";
			else if (filePath.Contains("Saline")) outMed.m_sConsumableType = "SALINE";
			else if (filePath.Contains("MedicalKit")) outMed.m_sConsumableType = "MEDKIT";
			else if (filePath.Contains("Aspirin")) outMed.m_sConsumableType = "PAINKILLER";
			else if (outMed.m_bIsMedical) outMed.m_sConsumableType = "MEDICAL_SUPPLY";
		}

		if (!outMed.m_bIsMedical && !outMed.m_sConsumableType.IsEmpty())
			outMed.m_bIsMedical = true;
	}

	//------------------------------------------------------------------------------------------------
	protected static string ConsumableTypeToString(int type)
	{
		switch (type)
		{
			case 0: return "BANDAGE";
			case 1: return "HEALTH";
			case 2: return "TOURNIQUET";
			case 3: return "SALINE";
			case 4: return "MORPHINE";
		}
		return "CONSUMABLE";
	}

	//------------------------------------------------------------------------------------------------
	//! Extract radio transceiver frequency bounds and power.
	static void ExtractRadio(map<string, ref array<BaseContainer>> comps, TBD_ItemRadioInfo outRadio, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("RadioComponent"))
				continue;

			outRadio.m_bIsRadio = true;

			foreach (BaseContainer comp : bucket)
			{
				BaseContainer cur = comp;
				while (cur)
				{
					string encKey;
					if (cur.Get("Encryption key", encKey) && !encKey.IsEmpty())
						outRadio.m_bHasEncryption = true;

					BaseContainerList transceivers = cur.GetObjectArray("Transceivers");
					if (transceivers && transceivers.Count() > 0)
					{
						outRadio.m_iChannelCount = transceivers.Count();
						for (int i = 0; i < transceivers.Count(); i++)
						{
							BaseContainer trx = transceivers.Get(i);
							if (!trx)
								continue;

							int minFi = 0, maxFi = 0;
							float minFf = 0, maxFf = 0, pwr = 0;

							if (outRadio.m_fMinFrequency < 0)
							{
								if (trx.Get("Min tunable frequency", minFi) && minFi > 0)
									outRadio.m_fMinFrequency = minFi;
								else if (trx.Get("Min tunable frequency", minFf) && minFf > 0)
									outRadio.m_fMinFrequency = minFf;
								else if (trx.Get("m_fMinFrequency", minFf) && minFf > 0)
									outRadio.m_fMinFrequency = minFf;
							}

							if (outRadio.m_fMaxFrequency < 0)
							{
								if (trx.Get("Max tunable frequency", maxFi) && maxFi > 0)
									outRadio.m_fMaxFrequency = maxFi;
								else if (trx.Get("Max tunable frequency", maxFf) && maxFf > 0)
									outRadio.m_fMaxFrequency = maxFf;
								else if (trx.Get("m_fMaxFrequency", maxFf) && maxFf > 0)
									outRadio.m_fMaxFrequency = maxFf;
							}

							if (outRadio.m_fTransmissionPower < 0)
							{
								if (trx.Get("Transmitting Range", pwr) && pwr > 0)
									outRadio.m_fTransmissionPower = pwr;
								else if (trx.Get("Range", pwr) && pwr > 0)
									outRadio.m_fTransmissionPower = pwr;
								else if (trx.Get("m_fTransmissionPower", pwr) && pwr > 0)
									outRadio.m_fTransmissionPower = pwr;
							}
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}

		if (outRadio.m_bIsRadio)
		{
			if (filePath.Contains("ANPRC68") || filePath.Contains("RF10") || filePath.Contains("R148"))
				outRadio.m_sRadioType = "VHF_HANDHELD";
			else if (filePath.Contains("ANPRC77") || filePath.Contains("R107M"))
				outRadio.m_sRadioType = "VHF_MANPACK";
			else
				outRadio.m_sRadioType = "TRANSCEIVER";
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract gadget attributes: optical magnification, FOV, or gadget type.
	static void ExtractGadget(map<string, ref array<BaseContainer>> comps, TBD_ItemGadgetInfo outGadget, string filePath)
	{
		bool hasBino = false;
		bool hasFlashlight = false;
		bool hasCompass = false;
		bool hasMap = false;
		bool hasWatch = false;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith("BinocularsComponent")) hasBino = true;
			if (cls.EndsWith("FlashlightComponent")) hasFlashlight = true;
			if (cls.EndsWith("CompassComponent")) hasCompass = true;
			if (cls.EndsWith("MapGadgetComponent")) hasMap = true;
			if (cls.EndsWith("WatchGadgetComponent")) hasWatch = true;

			if (hasBino)
			{
				foreach (BaseContainer comp : bucket)
				{
					BaseContainer cur = comp;
					while (cur)
					{
						float mag, fov;
						if (outGadget.m_fMagnification < 0 && cur.Get("m_fMagnification", mag) && mag > 0)
							outGadget.m_fMagnification = mag;
						if (outGadget.m_fFieldOfView < 0 && cur.Get("m_fFieldOfView", fov) && fov > 0)
							outGadget.m_fFieldOfView = fov;
						cur = cur.GetAncestor();
					}
				}
			}
		}

		if (hasBino || filePath.Contains("/Binoculars/"))
		{
			outGadget.m_bIsGadget = true;
			outGadget.m_sGadgetType = "BINOCULARS";
		}
		else if (hasFlashlight || filePath.Contains("/Flashlights/"))
		{
			outGadget.m_bIsGadget = true;
			outGadget.m_sGadgetType = "FLASHLIGHT";
		}
		else if (hasCompass || filePath.Contains("/Compass/"))
		{
			outGadget.m_bIsGadget = true;
			outGadget.m_sGadgetType = "COMPASS";
		}
		else if (hasMap || filePath.Contains("/Maps/"))
		{
			outGadget.m_bIsGadget = true;
			outGadget.m_sGadgetType = "MAP";
		}
		else if (hasWatch || filePath.Contains("/Watches/"))
		{
			outGadget.m_bIsGadget = true;
			outGadget.m_sGadgetType = "WATCH";
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract explosive attributes: mine triggers, arming delays, and demolition charges.
	static void ExtractExplosive(map<string, ref array<BaseContainer>> comps, TBD_ItemExplosiveInfo outExp, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith("MineComponent") || cls.EndsWith("MineWeaponComponent"))
			{
				outExp.m_bIsExplosive = true;
				if (filePath.Contains("TM62M") || filePath.Contains("M15AT"))
					outExp.m_sExplosiveType = "MINE_ANTITANK";
				else
					outExp.m_sExplosiveType = "MINE_ANTIPERSONNEL";

				foreach (BaseContainer comp : bucket)
				{
					BaseContainer cur = comp;
					while (cur)
					{
						float press;
						if (outExp.m_fTriggerPressureKg < 0 && cur.Get("m_fTriggerPressure", press) && press >= 0)
							outExp.m_fTriggerPressureKg = press;
						cur = cur.GetAncestor();
					}
				}
			}
			else if (cls.EndsWith("ExplosiveChargeComponent") || cls.EndsWith("ExplosiveChargeInventoryItemComponent"))
			{
				outExp.m_bIsExplosive = true;
				outExp.m_sExplosiveType = "DEMOLITION_BLOCK";
			}
			else if (cls.EndsWith("ExplosiveTriggerComponent") || cls.EndsWith("DetonatorComponent") || filePath.Contains("/Detonators/") || filePath.Contains("BlastingMachine"))
			{
				outExp.m_bIsExplosive = true;
				outExp.m_sExplosiveType = "DETONATOR";
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract tool properties: entrenching tools, demining tools, and repair kits.
	static void ExtractTool(map<string, ref array<BaseContainer>> comps, TBD_ItemToolInfo outTool, string filePath)
	{
		if (filePath.Contains("ETool") || filePath.Contains("Shovel"))
		{
			outTool.m_bIsTool = true;
			outTool.m_sToolType = "ENTRENCHING_TOOL";
			outTool.m_sActionType = "Digging / Fortification";
		}
		else if (filePath.Contains("MineFlag") || filePath.Contains("/Demining/"))
		{
			outTool.m_bIsTool = true;
			outTool.m_sToolType = "DEMINING_TOOL";
			outTool.m_sActionType = "Mine Marking";
		}
		else if (filePath.Contains("RepairKit") || filePath.Contains("wrench"))
		{
			outTool.m_bIsTool = true;
			outTool.m_sToolType = "REPAIR_TOOL";
			outTool.m_sActionType = "Vehicle / Equipment Maintenance";
		}
		else if (filePath.Contains("RearmingKit"))
		{
			outTool.m_bIsTool = true;
			outTool.m_sToolType = "REARMING_TOOL";
			outTool.m_sActionType = "Ammunition Service";
		}
		else if (filePath.Contains("BarbedTape") || filePath.Contains("Barbed_Tape"))
		{
			outTool.m_bIsTool = true;
			outTool.m_sToolType = "OBSTACLE_TOOL";
			outTool.m_sActionType = "Wire Obstacle Construction";
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract survival properties: canteens, food rations, tents, and jerrycans.
	static void ExtractSurvival(map<string, ref array<BaseContainer>> comps, TBD_ItemSurvivalInfo outSurv, string filePath)
	{
		if (filePath.Contains("/Canteens/") || filePath.Contains("Canteen"))
		{
			outSurv.m_bIsSurvival = true;
			outSurv.m_sSurvivalType = "WATER_CANTEEN";
			outSurv.m_fCapacity = 1.0;
		}
		else if (filePath.Contains("/Food/") || filePath.Contains("MRE") || filePath.Contains("ArmyCrackers") || filePath.Contains("Meat"))
		{
			outSurv.m_bIsSurvival = true;
			outSurv.m_sSurvivalType = "FOOD_RATION";
		}
		else if (filePath.Contains("/Tents/") || filePath.Contains("Tent"))
		{
			outSurv.m_bIsSurvival = true;
			outSurv.m_sSurvivalType = "DEPLOYABLE_TENT";
		}
		else if (filePath.Contains("Jerrycan") || filePath.Contains("Canister") || filePath.Contains("/Fuel/"))
		{
			outSurv.m_bIsSurvival = true;
			outSurv.m_sSurvivalType = "PORTABLE_FUEL_CONTAINER";
			outSurv.m_fCapacity = 20.0;
		}
		else if (filePath.Contains("SupplyPortableContainers") || filePath.Contains("SupplyCrate") || filePath.Contains("SupplyStack"))
		{
			outSurv.m_bIsSurvival = true;
			outSurv.m_sSurvivalType = "PORTABLE_SUPPLY_BOX";
		}
	}
}
