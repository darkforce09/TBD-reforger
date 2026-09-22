//------------------------------------------------------------------------------------------------
// TBD_AttachmentFamilyExtractor.c
//
// Decides which family an attachment belongs to, then reads the properties only that family has:
// suppression and muzzle-velocity change for a muzzle device, melee damage for a bayonet, beam
// colour and range for an illuminator, and so on.
//
// Categorization keys off components and attachment type first and falls back to the prefab path,
// because several families are distinguishable only by where the prefab lives. The handguard,
// mount, stock and bipod families declare nothing beyond what mounting already reads, so their
// readers are deliberately empty rather than absent - the catalog still carries the family.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentFamilyExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Assign canonical category based 100% on component types and native attachment type reflection.
	static string CategorizeAttachment(map<string, ref array<BaseContainer>> comps, string filePath, string attachmentType)
	{
		typename t;
		if (!attachmentType.IsEmpty())
			t = attachmentType.ToType();

		// 1. Muzzles & Suppressors
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SuppressorComponent")
			|| (t && (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentSuppressor || t.IsInherited(AttachmentSuppressor)
				|| t == AttachmentFlashHider || t.IsInherited(AttachmentFlashHider))))
		{
			return "muzzles";
		}

		// 2. Bayonets
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_BayonetComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_BayonetEffectComponent")
			|| (t && (t == AttachmentBayonet || t.IsInherited(AttachmentBayonet))))
		{
			return "bayonets";
		}

		// 3. Tactical Lights & Lasers
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_FlashlightComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "FlashlightComponent"))
		{
			return "illuminators";
		}

		// 4. Stocks
		if (t && (t == AttachmentStock || t.IsInherited(AttachmentStock)))
		{
			return "stocks";
		}

		// 5. Handguards
		if (t && (t == AttachmentHandGuard || t.IsInherited(AttachmentHandGuard)))
		{
			return "handguards";
		}

		// 6. Camouflage Wraps
		if (t && (t == AttachmentCamouflage || t == AttachmentCamouflageOptics || t.IsInherited(AttachmentCamouflage) || t.IsInherited(AttachmentCamouflageOptics)))
		{
			return "camouflage";
		}

		// 7. Mounts (Adapters providing attachment slots)
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "AttachmentSlotComponent") && (filePath.Contains("/mount/") || filePath.Contains("/mounts/") || (t && t.IsInherited(AttachmentOptics))))
		{
			return "mounts";
		}

		// 8. Bipods & Foregrips
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "BipodComponent") || (t && (t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel))))
		{
			return "bipods";
		}

		// Fallback by folder structure if attachment type was untyped base
		string lp = filePath;
		lp.ToLower();
		if (lp.Contains("/muzzle/") || lp.Contains("/muzzles/"))
			return "muzzles";
		if (lp.Contains("/bayonet/") || lp.Contains("/bayonets/"))
			return "bayonets";
		if (lp.Contains("/stock/") || lp.Contains("/stocks/"))
			return "stocks";
		if (lp.Contains("/handguard/") || lp.Contains("/handguards/"))
			return "handguards";
		if (lp.Contains("/flashlight/") || lp.Contains("/flashlights/") || lp.Contains("/laser/"))
			return "illuminators";
		if (lp.Contains("/mount/") || lp.Contains("/mounts/"))
			return "mounts";
		if (lp.Contains("/camouflage/"))
			return "camouflage";
		if (lp.Contains("/bipod/") || lp.Contains("/bipods/"))
			return "bipods";

		return "";
	}

	//------------------------------------------------------------------------------------------------
	//! Extract genuine muzzle and suppressor properties from SCR_WeaponAttachmentSuppressorAttributes.
	static void ExtractMuzzleData(map<string, ref array<BaseContainer>> comps, TBD_MuzzleAttachmentInfo outMuzzle, string filePath, string attachmentType)
	{
		typename attT;
		if (!attachmentType.IsEmpty())
			attT = attachmentType.ToType();

		string lowerMuzzlePath = filePath;
		lowerMuzzlePath.ToLower();
		bool isSuppressorType = (attT && (attT == AttachmentSuppressor || attT.IsInherited(AttachmentSuppressor) || attT.ToString().Contains("Suppressor")));
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SuppressorComponent") || isSuppressorType || lowerMuzzlePath.Contains("suppressor") || lowerMuzzlePath.Contains("silencer"))
			outMuzzle.m_bIsSuppressed = true;

		// Introspect SCR_WeaponAttachmentSuppressorAttributes from CustomAttributes
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainerList attrList = TBD_AttachmentCustomAttributes.GetCustomAttributes(attrs);
						if (attrList)
						{
							for (int a = 0, an = attrList.Count(); a < an; a++)
							{
								BaseContainer attr = attrList.Get(a);
								if (!attr)
									continue;

								string attrCls = attr.GetClassName();
								if (attrCls.Contains("SuppressorAttributes") || attrCls.Contains("WeaponAttachmentSuppressor"))
								{
									outMuzzle.m_bIsSuppressed = true;
									outMuzzle.m_bHasSuppressorAttributes = true;

									float speedCoef;
									if (attr.Get("m_fMuzzleSpeedCoefficient", speedCoef))
										outMuzzle.m_fMuzzleSpeedCoefficient = speedCoef;

									float dispFactor;
									if (attr.Get("m_fMuzzleDispersionFactor", dispFactor))
										outMuzzle.m_fMuzzleDispersionFactor = dispFactor;

									float extraLen;
									if (attr.Get("m_fExtraObstructionLength", extraLen))
										outMuzzle.m_fExtraObstructionLength = extraLen;

									bool ovrEffects;
									if (attr.Get("m_bOverrideMuzzleEffects", ovrEffects))
										outMuzzle.m_bOverrideMuzzleEffects = ovrEffects;

									bool ovrShot;
									if (attr.Get("m_bOverrideShot", ovrShot))
										outMuzzle.m_bOverrideShot = ovrShot;

									vector ang;
									if (attr.Get("m_vAngularFactors", ang) && ang != "0 0 0")
									{
										outMuzzle.m_vAngularFactors = ang;
										outMuzzle.m_bHasAngularFactors = true;
									}

									vector lin;
									if (attr.Get("m_vLinearFactors", lin) && lin != "0 0 0")
									{
										outMuzzle.m_vLinearFactors = lin;
										outMuzzle.m_bHasLinearFactors = true;
									}

									vector turn;
									if (attr.Get("m_vTurnFactors", turn) && turn != "0 0 0")
									{
										outMuzzle.m_vTurnFactors = turn;
										outMuzzle.m_bHasTurnFactors = true;
									}

									return;
								}
							}
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract genuine bayonet properties from SCR_WeaponAttachmentBayonetAttributes.
	static void ExtractBayonetData(map<string, ref array<BaseContainer>> comps, TBD_BayonetAttachmentInfo outBayonet, string filePath, string attachmentType)
	{
		typename attT;
		if (!attachmentType.IsEmpty())
			attT = attachmentType.ToType();

		string lowerBayonetPath = filePath;
		lowerBayonetPath.ToLower();
		if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_BayonetComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_BayonetEffectComponent") || (attT && (attT == AttachmentBayonet || attT.IsInherited(AttachmentBayonet))) || lowerBayonetPath.Contains("bayonet"))
			outBayonet.m_bIsBayonet = true;

		// Introspect SCR_WeaponAttachmentBayonetAttributes from CustomAttributes
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainerList attrList = TBD_AttachmentCustomAttributes.GetCustomAttributes(attrs);
						if (attrList)
						{
							for (int a = 0, an = attrList.Count(); a < an; a++)
							{
								BaseContainer attr = attrList.Get(a);
								if (!attr)
									continue;

								string attrCls = attr.GetClassName();
								if (attrCls.Contains("BayonetAttributes") || attrCls.Contains("WeaponAttachmentBayonet"))
								{
									outBayonet.m_bHasBayonetAttributes = true;

									bool isBay;
									if (attr.Get("m_bIsBayonet", isBay))
										outBayonet.m_bIsBayonet = isBay;
									else
										outBayonet.m_bIsBayonet = true;

									float dmgFactor;
									if (attr.Get("m_fDamageModificationFactor", dmgFactor))
										outBayonet.m_fDamageModificationFactor = dmgFactor;

									float extraLen;
									if (attr.Get("m_fExtraObstructionLength", extraLen))
										outBayonet.m_fExtraObstructionLength = extraLen;

									float precFactor;
									if (attr.Get("m_fPrecisionModificationFactor", precFactor))
										outBayonet.m_fPrecisionModificationFactor = precFactor;

									float rangeFactor;
									if (attr.Get("m_fRangeModificationFactor", rangeFactor))
										outBayonet.m_fRangeModificationFactor = rangeFactor;

									return;
								}
							}
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract genuine tactical light properties from SCR_FlashlightComponent.
	static void ExtractIlluminatorData(map<string, ref array<BaseContainer>> comps, TBD_IlluminatorAttachmentInfo outIllum, string filePath, string attachmentType)
	{
		bool hasLight = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_FlashlightComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "FlashlightComponent");
		bool hasLaser = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "LaserComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_LaserComponent");

		outIllum.m_bHasLight = hasLight;
		outIllum.m_bHasLaser = hasLaser;

		// Introspect SCR_FlashlightComponent
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("FlashlightComponent"))
				continue;

			foreach (BaseContainer flComp : bucket)
			{
				BaseContainer cur = flComp;
				while (cur)
				{
					float emInt;
					if (outIllum.m_fEmissiveIntensity < 0 && cur.Get("m_fEmissiveIntensity", emInt) && emInt >= 0)
						outIllum.m_fEmissiveIntensity = emInt;

					vector adjOff;
					if (!outIllum.m_bHasAdjustOffset && cur.Get("m_vFlashlightAdjustOffset", adjOff) && adjOff != "0 0 0")
					{
						outIllum.m_vFlashlightAdjustOffset = adjOff;
						outIllum.m_bHasAdjustOffset = true;
					}

					float nearPlane;
					if (outIllum.m_fLightNearPlaneHand < 0 && cur.Get("m_fLightNearPlaneHand", nearPlane) && nearPlane >= 0)
						outIllum.m_fLightNearPlaneHand = nearPlane;

					// Extract lens color array
					if (outIllum.m_aLenses.Count() == 0)
					{
						BaseContainerList lenses = cur.GetObjectArray("m_aLenseArray");
						if (lenses)
						{
							for (int li = 0, ln = lenses.Count(); li < ln; li++)
							{
								BaseContainer lObj = lenses.Get(li);
								if (!lObj)
									continue;

								TBD_IlluminatorLensInfo lens = new TBD_IlluminatorLensInfo();
								lObj.Get("m_sDescription", lens.m_sDescription);
								lObj.Get("m_vLenseColor", lens.m_vLenseColor);
								lObj.Get("m_fLightValue", lens.m_fLightValue);
								outIllum.m_aLenses.Insert(lens);
							}
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract handguard nested child AttachmentSlotComponent slots.
	static void ExtractHandguardData(map<string, ref array<BaseContainer>> comps, TBD_HandguardAttachmentInfo outHg, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outHg.m_aNestedSlots);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract mount adapter nested child AttachmentSlotComponent slots.
	static void ExtractMountData(map<string, ref array<BaseContainer>> comps, TBD_MountAttachmentInfo outMount, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outMount.m_aNestedSlots);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract stock nested child AttachmentSlotComponent slots.
	static void ExtractStockData(map<string, ref array<BaseContainer>> comps, TBD_StockAttachmentInfo outStock, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outStock.m_aNestedSlots);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract bipod nested child AttachmentSlotComponent slots.
	static void ExtractBipodData(map<string, ref array<BaseContainer>> comps, TBD_BipodAttachmentInfo outBipod, string filePath, string attachmentType)
	{
		TBD_AttachmentMountingExtractor.ExtractNestedAttachmentSlots(comps, outBipod.m_aNestedSlots);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract camouflage wrap target type from genuine attachment type reflection.
	static void ExtractCamouflageData(map<string, ref array<BaseContainer>> comps, TBD_CamouflageAttachmentInfo outCamo, string filePath, string attachmentType = "")
	{
		typename t;
		if (!attachmentType.IsEmpty())
			t = attachmentType.ToType();

		if (t && (t == AttachmentCamouflageOptics || t.IsInherited(AttachmentCamouflageOptics)))
			outCamo.m_sTargetType = "optic";
		else
			outCamo.m_sTargetType = "weapon";
	}
}
