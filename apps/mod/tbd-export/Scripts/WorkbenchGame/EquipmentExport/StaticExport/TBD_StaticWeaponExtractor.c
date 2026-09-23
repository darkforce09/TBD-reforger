/**
 * TBD_StaticWeaponExtractor.c
 *
 * Introspects static and crew-served weapon prefabs for:
 *   - Turret limits (horizontal traverse, vertical elevation) & aiming speed
 *   - Optical sights (2D sights magnification, reticles, illumination)
 *   - Gunner crew compartment slots and passenger offsets
 *   - Armament (integral mortar muzzle or mounted weapon template)
 *   - Dismantle / multi-part deployment linkages (baseplate, barrel, bipod)
 *   - Physical attributes (mass, destruction debris model)
 */

class TBD_StaticWeaponExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract all static weapon specs into the info object.
	static void ExtractSpecs(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		ExtractTurret(comps, info);
		ExtractCrew(comps, info);
		ExtractArmament(comps, info);
		ExtractDeployment(comps, info);
		ExtractPhysical(comps, info);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract turret limits and optical sights.
	protected static void ExtractTurret(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		info.m_bHasTurret = false;
		info.m_fTraverseMin = 0;
		info.m_fTraverseMax = 0;
		info.m_fElevationMin = 0;
		info.m_fElevationMax = 0;
		info.m_fAimingMaxSpeed = 0;
		info.m_sSightsType = string.Empty;
		info.m_fMagnification = 1.0;
		info.m_sReticleTexture = string.Empty;
		info.m_bHasIllumination = false;

		// 1. Inspect TurretComponent
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("TurretComponent"))
				continue;

			foreach (BaseContainer tc : bucket)
			{
				info.m_bHasTurret = true;

				vector limH;
				if (tc.Get("LimitsHoriz", limH))
				{
					info.m_fTraverseMin = limH[0];
					info.m_fTraverseMax = limH[1];
				}

				vector limV;
				if (tc.Get("LimitsVert", limV))
				{
					info.m_fElevationMin = limV[0];
					info.m_fElevationMax = limV[1];
				}

				vector aimSpd;
				if (tc.Get("AimingMaxSpeed", aimSpd))
				{
					info.m_fAimingMaxSpeed = aimSpd[0];
				}

				// Check nested sights in TurretComponent
				BaseContainerList nestedComps = tc.GetObjectArray("components");
				if (nestedComps)
				{
					for (int i = 0; i < nestedComps.Count(); i++)
					{
						BaseContainer sc = nestedComps.Get(i);
						if (!sc) continue;
						ExtractSightsComponent(sc, info);
					}
				}
			}
		}

		// 2. Check SCR_TurretControllerComponent if limits were not on TurretComponent
		if (info.m_fTraverseMin == 0 && info.m_fTraverseMax == 0)
		{
			foreach (string ccls, array<BaseContainer> cbucket : comps)
			{
				if (!ccls.EndsWith("TurretControllerComponent"))
					continue;

				foreach (BaseContainer tcc : cbucket)
				{
					vector cLimH;
					if (tcc.Get("LimitsHoriz", cLimH))
					{
						info.m_fTraverseMin = cLimH[0];
						info.m_fTraverseMax = cLimH[1];
					}
					vector cLimV;
					if (tcc.Get("LimitsVert", cLimV))
					{
						info.m_fElevationMin = cLimV[0];
						info.m_fElevationMax = cLimV[1];
					}
				}
			}
		}

		// 3. Fallback: inspect any top-level sights component
		if (info.m_sSightsType.IsEmpty())
		{
			foreach (string scls, array<BaseContainer> sbucket : comps)
			{
				if (!scls.Contains("SightsComponent"))
					continue;

				foreach (BaseContainer topSc : sbucket)
				{
					ExtractSightsComponent(topSc, info);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Helper for extracting optics & reticle from a sights component container.
	protected static void ExtractSightsComponent(BaseContainer sc, TBD_StaticWeaponInfo info)
	{
		info.m_sSightsType = sc.GetClassName();

		float mag = 1.0;
		if (sc.Get("m_fMagnification", mag) && mag > 0)
			info.m_fMagnification = mag;

		string ret;
		if (sc.Get("m_sReticleTexture", ret) && !ret.IsEmpty())
			info.m_sReticleTexture = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(ret);

		bool illum = false;
		if (sc.Get("m_bHasIllumination", illum))
			info.m_bHasIllumination = illum;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract crew compartment and gunner seating offset.
	protected static void ExtractCrew(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		info.m_bHasCrewSlot = false;
		info.m_sCompartmentSlot = string.Empty;
		info.m_vPassengerOffset = vector.Zero;
		info.m_sDefaultOccupantPrefab = string.Empty;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("CompartmentManager"))
				continue;

			foreach (BaseContainer cmc : bucket)
			{
				BaseContainerList slots = cmc.GetObjectArray("CompartmentSlots");
				if (!slots)
					continue;

				for (int i = 0; i < slots.Count(); i++)
				{
					BaseContainer slot = slots.Get(i);
					if (!slot) continue;

					info.m_bHasCrewSlot = true;
					info.m_sCompartmentSlot = slot.GetClassName();

					BaseContainer passPos = slot.GetObject("PassengerPositionInfo");
					if (passPos)
					{
						vector off;
						if (passPos.Get("Offset", off))
							info.m_vPassengerOffset = off;
					}

					string occ;
					if (slot.Get("DefaultOccupantPrefab", occ) && !occ.IsEmpty())
						info.m_sDefaultOccupantPrefab = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(occ);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract mounted weapon template, caliber, or integral mortar muzzle.
	protected static void ExtractArmament(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		info.m_bHasArmament = false;
		info.m_bIsIntegral = false;
		info.m_sMountedWeaponTemplate = string.Empty;
		info.m_sCaliber = string.Empty;
		info.m_sMagazineWell = string.Empty;
		info.m_aSupportedAmmo.Clear();
		info.m_aInitialMagazines.Clear();

		// Check WeaponSlotComponent (used on Tripods with mounted heavy weapons)
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("WeaponSlotComponent"))
				continue;

			foreach (BaseContainer wsc : bucket)
			{
				string tmpl;
				if (wsc.Get("WeaponTemplate", tmpl) && !tmpl.IsEmpty())
				{
					// If the template is the generic Turret_Base fallback pintle and we are scanning a tripod/mortar, ignore it!
					if (tmpl.Contains("HMG_M2HB_pintle"))
						continue;

					info.m_bHasArmament = true;
					info.m_bIsIntegral = false;
					info.m_sMountedWeaponTemplate = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(tmpl);

					// Infer caliber from template
					if (tmpl.Contains("M2HB")) info.m_sCaliber = "12.7x99mm";
					else if (tmpl.Contains("NSV")) info.m_sCaliber = "12.7x108mm";
					else if (tmpl.Contains("PKM") || tmpl.Contains("PK_")) info.m_sCaliber = "7.62x54mm";
					else if (tmpl.Contains("M60")) info.m_sCaliber = "7.62x51mm";
					break;
				}
			}
		}

		// Check integral WeaponComponent / SCR_MortarMuzzleComponent (used on Mortars)
		foreach (string wcls, array<BaseContainer> wbucket : comps)
		{
			if (!wcls.EndsWith("WeaponComponent"))
				continue;

			foreach (BaseContainer wc : wbucket)
			{
				BaseContainerList nested = wc.GetObjectArray("components");
				if (!nested) continue;

				for (int i = 0; i < nested.Count(); i++)
				{
					BaseContainer mc = nested.Get(i);
					if (!mc) continue;

					if (mc.GetClassName().Contains("MortarMuzzle"))
					{
						info.m_bHasArmament = true;
						info.m_bIsIntegral = true;

						BaseContainer magWell = mc.GetObject("MagazineWell");
						if (magWell)
							info.m_sMagazineWell = magWell.GetClassName();

						string ammoTmpl;
						if (mc.Get("AmmoTemplate", ammoTmpl) && !ammoTmpl.IsEmpty())
						{
							string canAmmo = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(ammoTmpl);
							info.m_aSupportedAmmo.Insert(canAmmo);
						}

						if (info.m_sMagazineWell.Contains("82mm") || info.m_sFilePath.Contains("2B14"))
							info.m_sCaliber = "82mm";
						else if (info.m_sMagazineWell.Contains("81mm") || info.m_sFilePath.Contains("M252"))
							info.m_sCaliber = "81mm";
					}
				}
			}
		}

		// Check initial inventory magazines (e.g. M2 ammo boxes)
		foreach (string icls, array<BaseContainer> ibucket : comps)
		{
			if (!icls.Contains("UniversalInventoryStorage"))
				continue;

			foreach (BaseContainer uisc : ibucket)
			{
				BaseContainerList initSlots = uisc.GetObjectArray("InitialStorageSlots");
				if (!initSlots) continue;

				for (int s = 0; s < initSlots.Count(); s++)
				{
					BaseContainer islot = initSlots.Get(s);
					if (!islot) continue;
					string p;
					if (islot.Get("Prefab", p) && !p.IsEmpty())
					{
						info.m_aInitialMagazines.Insert(TBD_EquipmentResourceNames.ResolveCanonicalResourceName(p));
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract dismantle and multi-part deployment components.
	protected static void ExtractDeployment(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		info.m_bIsDeployable = false;
		info.m_sDismantleAction = string.Empty;
		info.m_sReplacementBase = string.Empty;
		info.m_sBarrelPrefab = string.Empty;
		info.m_sBipodPrefab = string.Empty;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.Contains("MultiPartDeployableItem"))
				continue;

			foreach (BaseContainer mpc : bucket)
			{
				info.m_bIsDeployable = true;

				BaseContainerList variants = mpc.GetObjectArray("m_aVariants");
				if (!variants) continue;

				for (int v = 0; v < variants.Count(); v++)
				{
					BaseContainer var = variants.Get(v);
					if (!var) continue;

					string repl;
					if (var.Get("m_sReplacementPrefab", repl) && !repl.IsEmpty())
						info.m_sReplacementBase = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(repl);

					BaseContainerList addParts = var.GetObjectArray("m_aAdditionalPrefabs");
					if (addParts)
					{
						for (int p = 0; p < addParts.Count(); p++)
						{
							BaseContainer part = addParts.Get(p);
							if (!part) continue;
							string partPf;
							if (part.Get("m_sPrefab", partPf) && !partPf.IsEmpty())
							{
								string canPart = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(partPf);
								if (canPart.Contains("Barrel") || canPart.Contains("Cannon"))
									info.m_sBarrelPrefab = canPart;
								else if (canPart.Contains("Bipod") || canPart.Contains("Mount"))
									info.m_sBipodPrefab = canPart;
							}
						}
					}
				}
			}
		}

		// Dismantle action name
		foreach (string acls, array<BaseContainer> abucket : comps)
		{
			if (!acls.Contains("ActionsManager"))
				continue;

			foreach (BaseContainer amc : abucket)
			{
				BaseContainerList actions = amc.GetObjectArray("additionalActions");
				if (!actions) continue;

				for (int a = 0; a < actions.Count(); a++)
				{
					BaseContainer act = actions.Get(a);
					if (!act) continue;
					if (act.GetClassName().Contains("DeployMultiPart"))
						info.m_sDismantleAction = act.GetClassName();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract mass and destroyed debris model.
	protected static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_StaticWeaponInfo info)
	{
		info.m_fMassKg = 0;
		info.m_sDebrisModel = string.Empty;

		// RigidBodyComponent / Mass
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("RigidBodyComponent"))
				continue;

			foreach (BaseContainer rbc : bucket)
			{
				float mass;
				if (rbc.Get("Mass", mass) && mass > 0)
					info.m_fMassKg = mass;
			}
		}

		// Multiphase destruction debris
		foreach (string dcls, array<BaseContainer> dbucket : comps)
		{
			if (!dcls.Contains("DestructionMultiPhase"))
				continue;

			foreach (BaseContainer dmc : dbucket)
			{
				BaseContainerList spawns = dmc.GetObjectArray("m_DestroySpawnObjects");
				if (!spawns) continue;

				for (int s = 0; s < spawns.Count(); s++)
				{
					BaseContainer sp = spawns.Get(s);
					if (!sp) continue;

					float dMass;
					if (sp.Get("m_fMass", dMass) && dMass > 0 && info.m_fMassKg <= 0)
						info.m_fMassKg = dMass;

					BaseContainerList models = sp.GetObjectArray("m_ModelPrefabs");
					if (models && models.Count() > 0)
					{
						string m0;
						if (models.Get(0).Get("m_sPrefab", m0) && !m0.IsEmpty())
							info.m_sDebrisModel = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(m0);
					}
				}
			}
		}

		// Fallback mass by family if mass was 0
		if (info.m_fMassKg <= 0)
		{
			if (info.m_sFamily == "2B14") info.m_fMassKg = 42.0;
			else if (info.m_sFamily == "M252") info.m_fMassKg = 41.3;
			else if (info.m_sFamily == "M2HB") info.m_fMassKg = 38.1;
			else if (info.m_sFamily == "NSV") info.m_fMassKg = 41.0;
			else if (info.m_sFamily == "PKM") info.m_fMassKg = 16.5;
			else if (info.m_sFamily == "M60") info.m_fMassKg = 18.5;
			else if (info.m_sFamily == "M3") info.m_fMassKg = 20.0;
			else if (info.m_sFamily == "M122") info.m_fMassKg = 7.5;
			else if (info.m_sFamily == "6T5") info.m_fMassKg = 8.0;
			else if (info.m_sFamily == "6T7") info.m_fMassKg = 16.0;
		}
	}
}
