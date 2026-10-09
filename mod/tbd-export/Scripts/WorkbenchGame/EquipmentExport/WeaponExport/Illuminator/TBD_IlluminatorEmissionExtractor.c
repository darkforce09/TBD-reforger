//------------------------------------------------------------------------------------------------
// TBD_IlluminatorEmissionExtractor.c
//
// Reads what a tactical light or pointer emits: beam colour, intensity, cone angle and range,
// whether it is visible light or infrared, and whether it is a laser, a flashlight, or both.
//
// The engine spreads these across a light component, a laser component and a lens configuration,
// and a variant prefab overrides only some of them, so each value is searched up the container
// ancestry. ExtractColor and ExtractBoolProperty sit here because only emission reads them.
//------------------------------------------------------------------------------------------------

class TBD_IlluminatorEmissionExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract genuine tactical light and laser properties from BaseContainer component tree.
	//! Introspects SCR_FlashlightComponent, BaseLightComponent, LightComponent, SCR_LaserComponent,
	//! LaserComponent, and container ancestry.
	static void ExtractIllumination(map<string, ref array<BaseContainer>> comps, TBD_IlluminatorCapabilityInfo outIllum, string attachmentType)
	{
		bool hasLightComp = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_FlashlightComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "FlashlightComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "BaseLightComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "LightComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_BaseInteractiveLightComponent");

		bool hasLaserComp = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_LaserComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "LaserComponent")
			|| TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_LaserPointerComponent");

		string lowerAttType = attachmentType;
		lowerAttType.ToLower();

		bool hasLightType = lowerAttType.Contains("flashlight") || lowerAttType.Contains("light") || lowerAttType.Contains("illum");
		bool hasLaserType = lowerAttType.Contains("laser") || lowerAttType.Contains("pointer");

		outIllum.m_bHasFlashlight = hasLightComp || hasLightType;
		outIllum.m_bHasLaser = hasLaserComp || hasLaserType;

		array<string> irProps = { "m_bIsIR", "m_bIR", "m_bIsInfrared", "m_bInfraRed", "IsIR", "IsInfrared" };

		// 1. Introspect Flashlight / Light Components
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("FlashlightComponent") && !cls.EndsWith("LightComponent") && !cls.EndsWith("InteractiveLightComponent"))
				continue;

			foreach (BaseContainer flComp : bucket)
			{
				BaseContainer cur = flComp;
				while (cur)
				{
					// Emissive intensity
					if (!outIllum.m_bHasEmissiveIntensity)
					{
						float emInt;
						if (cur.Get("m_fEmissiveIntensity", emInt) && emInt >= 0)
						{
							outIllum.m_fEmissiveIntensity = emInt;
							outIllum.m_bHasEmissiveIntensity = true;
						}
						else if (cur.Get("EmissiveIntensity", emInt) && emInt >= 0)
						{
							outIllum.m_fEmissiveIntensity = emInt;
							outIllum.m_bHasEmissiveIntensity = true;
						}
						else if (cur.Get("m_fIntensity", emInt) && emInt >= 0)
						{
							outIllum.m_fEmissiveIntensity = emInt;
							outIllum.m_bHasEmissiveIntensity = true;
						}
					}

					// Light near plane
					if (!outIllum.m_bHasLightNearPlane)
					{
						float nearPlane;
						if (cur.Get("m_fLightNearPlaneHand", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
						else if (cur.Get("m_fLightNearPlane", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
						else if (cur.Get("LightNearPlaneHand", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
						else if (cur.Get("LightNearPlane", nearPlane) && nearPlane >= 0)
						{
							outIllum.m_fLightNearPlane = nearPlane;
							outIllum.m_bHasLightNearPlane = true;
						}
					}

					// Adjust offset vector
					if (!outIllum.m_bHasAdjustOffset)
					{
						vector adjOff;
						if (cur.Get("m_vFlashlightAdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
						else if (cur.Get("FlashlightAdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
						else if (cur.Get("m_vAdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
						else if (cur.Get("AdjustOffset", adjOff) && adjOff != "0 0 0")
						{
							outIllum.m_vAdjustOffset = adjOff;
							outIllum.m_bHasAdjustOffset = true;
						}
					}

					// Infrared (IR) flag
					if (!outIllum.m_bHasIsIR)
					{
						bool irVal;
						if (ExtractBoolProperty(cur, irProps, irVal))
						{
							outIllum.m_bIsIR = irVal;
							outIllum.m_bHasIsIR = true;
						}
					}

					// Lenses array
					if (!outIllum.m_bHasLenses)
					{
						BaseContainerList lenses = cur.GetObjectArray("m_aLenseArray");
						if (!lenses) lenses = cur.GetObjectArray("m_aLensArray");
						if (!lenses) lenses = cur.GetObjectArray("LenseArray");
						if (!lenses) lenses = cur.GetObjectArray("LensArray");
						if (!lenses) lenses = cur.GetObjectArray("Lenses");

						if (lenses && lenses.Count() > 0)
						{
							for (int li = 0, ln = lenses.Count(); li < ln; li++)
							{
								BaseContainer lObj = lenses.Get(li);
								if (!lObj)
									continue;

								ref TBD_TacticalIlluminatorLensInfo lens = new TBD_TacticalIlluminatorLensInfo();

								// Description (raw localization token preserved!)
								if (!lObj.Get("m_sDescription", lens.m_sDescription))
								{
									if (!lObj.Get("Description", lens.m_sDescription))
										lObj.Get("m_Description", lens.m_sDescription);
								}

								// Color vector
								vector col;
								if (lObj.Get("m_vLenseColor", col) || lObj.Get("m_vLensColor", col) || lObj.Get("LenseColor", col) || lObj.Get("LensColor", col) || lObj.Get("Color", col))
								{
									lens.m_vColor = col;
								}

								// Light intensity
								float lv;
								if (lObj.Get("m_fLightValue", lv) || lObj.Get("LightValue", lv) || lObj.Get("m_fIntensity", lv) || lObj.Get("Intensity", lv))
								{
									lens.m_fLightValue = lv;
								}

								outIllum.m_aLenses.Insert(lens);
							}
							outIllum.m_bHasLenses = (outIllum.m_aLenses.Count() > 0);
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// 2. Introspect Laser Components
		foreach (string lCls, array<BaseContainer> lBucket : comps)
		{
			if (!lCls.EndsWith("LaserComponent") && !lCls.EndsWith("LaserPointerComponent"))
				continue;

			foreach (BaseContainer lComp : lBucket)
			{
				BaseContainer curLaser = lComp;
				while (curLaser)
				{
					// Infrared (IR) flag on laser
					if (!outIllum.m_bHasIsIR)
					{
						bool lIrVal;
						if (ExtractBoolProperty(curLaser, irProps, lIrVal))
						{
							outIllum.m_bIsIR = lIrVal;
							outIllum.m_bHasIsIR = true;
						}
					}

					// Laser color array (RGB or RGBA)
					if (!outIllum.m_bHasLaserColor)
					{
						array<string> colorProps = { "m_cLaserColor", "m_vLaserColor", "LaserColor", "m_Color", "Color", "m_vBeamColor", "m_vDotColor", "m_cDotColor" };
						foreach (string cProp : colorProps)
						{
							if (ExtractColor(curLaser, cProp, outIllum.m_aLaserColor))
							{
								outIllum.m_bHasLaserColor = true;
								break;
							}
						}
					}

					curLaser = curLaser.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Helper for extracting RGB/RGBA color arrays from vector or string property.
	protected static bool ExtractColor(BaseContainer container, string propName, notnull array<float> outColor)
	{
		string rawStr = "";
		if (container.GetDefaultAsString(propName, rawStr) && !rawStr.IsEmpty())
		{
			array<string> parts = {};
			rawStr.Split(" ", parts, false);
			if (parts.Count() >= 3)
			{
				outColor.Clear();
				outColor.Insert(parts[0].ToFloat());
				outColor.Insert(parts[1].ToFloat());
				outColor.Insert(parts[2].ToFloat());
				if (parts.Count() >= 4)
					outColor.Insert(parts[3].ToFloat());
				return true;
			}
		}

		vector vCol;
		if (container.Get(propName, vCol))
		{
			outColor.Clear();
			outColor.Insert(vCol[0]);
			outColor.Insert(vCol[1]);
			outColor.Insert(vCol[2]);
			return true;
		}

		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Helper for reading boolean properties across multiple naming variations.
	protected static bool ExtractBoolProperty(BaseContainer container, array<string> propNames, out bool outVal)
	{
		foreach (string prop : propNames)
		{
			if (container.IsVariableSet(prop))
			{
				if (container.Get(prop, outVal))
					return true;
			}
		}
		return false;
	}
}
