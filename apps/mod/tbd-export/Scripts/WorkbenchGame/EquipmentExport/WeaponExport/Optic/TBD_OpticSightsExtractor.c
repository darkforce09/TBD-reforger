//------------------------------------------------------------------------------------------------
// TBD_OpticSightsExtractor.c
//
// Reads what a sight shows the player: magnification, field of view, eye relief, objective
// diameter, zeroing distances, reticle texture and colours, and whether it ranges.
//
// This is one walk, not several. The sights component is chosen by precedence - 2DOptics over
// collimator over plain sights - and then a single pass up the container ancestry fills every
// field of TBD_OpticSightsInfo, because a variant prefab declares only what it changes and each
// value has to be searched for independently. Splitting the pass would mean walking the same
// ancestry several times.
//
// ExtractRgbaColor sits here because only the reticle and glow colours use it.
//------------------------------------------------------------------------------------------------

class TBD_OpticSightsExtractor
{
	//------------------------------------------------------------------------------------------------
	//! Extract optical sights parameters directly from native Enfusion BaseContainer instances:
	//!   - sights_class: exact engine component class name
	//!   - magnification_min, magnification_max, magnification_steps, is_magnified
	//!   - fov_min_degrees, fov_max_degrees
	//!   - eye_relief_meters, objective_diameter_mm
	//!   - zeroing_distances
	//!   - reticle_texture, reticle_color, glow_color
	//!   - has_rangefinder
	//! ZERO synthetic defaults: unconfigured attributes serialize as JSON null.
	static void ExtractSights(map<string, ref array<BaseContainer>> comps, TBD_OpticSightsInfo outSights)
	{
		array<float> steps = {};
		string primarySightsClass = "";

		// 1. Identify primary sights component class with precedence: 2DOptics > Collimator > Sights
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith("2DOpticsComponent") || cls.EndsWith("2DSightsComponent") || cls.EndsWith("2DPIPSightsComponent") || cls.EndsWith("PIPSightsComponent"))
			{
				primarySightsClass = cls;
				break;
			}
		}

		if (primarySightsClass.IsEmpty())
		{
			foreach (string cCls, array<BaseContainer> cBucket : comps)
			{
				if (cCls.EndsWith("CollimatorSightsComponent"))
				{
					primarySightsClass = cCls;
					break;
				}
			}
		}

		if (primarySightsClass.IsEmpty())
		{
			foreach (string sCls, array<BaseContainer> sBucket : comps)
			{
				if (sCls.EndsWith("SightsComponent") && !sCls.EndsWith("AttachmentSlotComponent"))
				{
					primarySightsClass = sCls;
					break;
				}
			}
		}

		outSights.m_sSightsClass = primarySightsClass;

		// 2. Deep container walk across all sights and optics components
		foreach (string compCls, array<BaseContainer> compBucket : comps)
		{
			if (!compCls.EndsWith("SightsComponent") && !compCls.EndsWith("OpticsComponent") && !compCls.EndsWith("Sights"))
				continue;

			foreach (BaseContainer comp : compBucket)
			{
				BaseContainer cur = comp;
				while (cur)
				{
					// --- Eye Relief ---
					if (!outSights.m_bHasEyeRelief)
					{
						float er;
						if ((cur.Get("m_fEyeReliefMeters", er) || cur.Get("m_fEyeRelief", er) || cur.Get("EyeRelief", er)) && er > 0)
						{
							outSights.m_fEyeReliefMeters = er;
							outSights.m_bHasEyeRelief = true;
						}
					}

					// --- Objective Diameter ---
					if (!outSights.m_bHasObjectiveDiameter)
					{
						float od;
						if ((cur.Get("m_fObjectiveDiameterMm", od) || cur.Get("m_fObjectiveDiameter", od) || cur.Get("ObjectiveDiameter", od) || cur.Get("m_fObjectiveLensDiameter", od)) && od > 0)
						{
							outSights.m_fObjectiveDiameterMm = od;
							outSights.m_bHasObjectiveDiameter = true;
						}
					}

					// --- SightsFOVInfo inspection ---
					BaseContainer fovInfo = cur.GetObject("SightsFOVInfo");
					if (!fovInfo)
						fovInfo = cur.GetObject("m_pFOVInfo");
					if (!fovInfo)
						fovInfo = cur.GetObject("FOVInfo");

					if (fovInfo)
					{
						BaseContainer curFov = fovInfo;
						while (curFov)
						{
							// Discrete magnification levels
							BaseContainerList magLevels = curFov.GetObjectArray("m_aMagnificationLevels");
							if (!magLevels)
								magLevels = curFov.GetObjectArray("m_aLevels");

							if (magLevels)
							{
								for (int m = 0, mn = magLevels.Count(); m < mn; m++)
								{
									BaseContainer ml = magLevels.Get(m);
									if (ml)
									{
										float lv;
										if ((ml.Get("m_fMagnification", lv) || ml.Get("Magnification", lv)) && lv > 0)
										{
											if (steps.Find(lv) == -1)
												steps.Insert(lv);
										}
									}
								}
							}

							// Single magnification factor on FOVInfo
							float singleMag;
							if ((curFov.Get("m_fMagnification", singleMag) || curFov.Get("Magnification", singleMag)) && singleMag > 0)
							{
								if (steps.Find(singleMag) == -1)
									steps.Insert(singleMag);
							}

							// Min/Max magnification
							float mmMin;
							if ((curFov.Get("m_fMagnificationMin", mmMin) || curFov.Get("MagnificationMin", mmMin)) && mmMin > 0)
							{
								outSights.m_fMagnificationMin = mmMin;
								outSights.m_bHasMagnificationMin = true;
							}

							float mmMax;
							if ((curFov.Get("m_fMagnificationMax", mmMax) || curFov.Get("MagnificationMax", mmMax)) && mmMax > 0)
							{
								outSights.m_fMagnificationMax = mmMax;
								outSights.m_bHasMagnificationMax = true;
							}

							// FOV in degrees (convert from radians if < 3.15)
							float fovVal;
							if (curFov.Get("m_fFieldOfView", fovVal) || curFov.Get("m_fFOV", fovVal) || curFov.Get("FieldOfView", fovVal))
							{
								if (fovVal > 0)
								{
									if (fovVal < 3.15)
										fovVal = fovVal * Math.RAD2DEG;

									if (!outSights.m_bHasFovMin)
									{
										outSights.m_fFovMinDegrees = fovVal;
										outSights.m_bHasFovMin = true;
									}
									if (!outSights.m_bHasFovMax)
									{
										outSights.m_fFovMaxDegrees = fovVal;
										outSights.m_bHasFovMax = true;
									}
								}
							}

							float fMin;
							if ((curFov.Get("m_fFovMin", fMin) || curFov.Get("m_fFieldOfViewMin", fMin) || curFov.Get("FovMin", fMin)) && fMin > 0)
							{
								if (fMin < 3.15)
									fMin = fMin * Math.RAD2DEG;
								outSights.m_fFovMinDegrees = fMin;
								outSights.m_bHasFovMin = true;
							}

							float fMax;
							if ((curFov.Get("m_fFovMax", fMax) || curFov.Get("m_fFieldOfViewMax", fMax) || curFov.Get("FovMax", fMax)) && fMax > 0)
							{
								if (fMax < 3.15)
									fMax = fMax * Math.RAD2DEG;
								outSights.m_fFovMaxDegrees = fMax;
								outSights.m_bHasFovMax = true;
							}

							curFov = curFov.GetAncestor();
						}
					}

					// Direct magnification check on component if still empty
					if (steps.IsEmpty())
					{
						float cMag;
						if ((cur.Get("m_fMagnification", cMag) || cur.Get("Magnification", cMag)) && cMag > 0)
						{
							if (steps.Find(cMag) == -1)
								steps.Insert(cMag);
						}
					}

					// --- Zeroing Distances from SightsRanges ---
					if (!outSights.m_bHasZeroingDistances)
					{
						BaseContainerList sightRanges = cur.GetObjectArray("SightsRanges");
						if (sightRanges && sightRanges.Count() > 0)
						{
							for (int sr = 0, srn = sightRanges.Count(); sr < srn; sr++)
							{
								BaseContainer sri = sightRanges.Get(sr);
								if (!sri)
									continue;

								vector rVec;
								if (sri.Get("Range", rVec))
								{
									int distMeters = Math.Round(rVec[1]);
									if (distMeters > 0)
									{
										string dStr = distMeters.ToString() + "m";
										if (outSights.m_aZeroingDistances.Find(dStr) == -1)
											outSights.m_aZeroingDistances.Insert(dStr);
										continue;
									}
								}

								string rStr = "";
								if (sri.GetDefaultAsString("Range", rStr) && !rStr.IsEmpty())
								{
									array<string> parts = {};
									rStr.Split(" ", parts, false);
									if (!parts.IsEmpty())
									{
										string lastPart = parts[parts.Count() - 1];
										int dist = lastPart.ToInt();
										if (dist > 0)
										{
											string ds = dist.ToString() + "m";
											if (outSights.m_aZeroingDistances.Find(ds) == -1)
												outSights.m_aZeroingDistances.Insert(ds);
										}
										else if (!lastPart.IsEmpty())
										{
											string ds2 = lastPart + "m";
											if (outSights.m_aZeroingDistances.Find(ds2) == -1)
												outSights.m_aZeroingDistances.Insert(ds2);
										}
									}
								}
							}

							if (outSights.m_aZeroingDistances.Count() > 0)
								outSights.m_bHasZeroingDistances = true;
						}
					}

					// Direct zeroing distance array on component
					if (!outSights.m_bHasZeroingDistances)
					{
						array<float> ranges = {};
						cur.Get("m_aZeroingDistances", ranges);
						if (ranges && ranges.Count() > 0)
						{
							foreach (float r : ranges)
							{
								string rd = r.ToString() + "m";
								if (outSights.m_aZeroingDistances.Find(rd) == -1)
									outSights.m_aZeroingDistances.Insert(rd);
							}
							outSights.m_bHasZeroingDistances = true;
						}
						else
						{
							float singleZero;
							if (cur.Get("m_fZeroingDistance", singleZero) && singleZero > 0)
							{
								outSights.m_aZeroingDistances.Insert(singleZero.ToString() + "m");
								outSights.m_bHasZeroingDistances = true;
							}
						}
					}

					// --- Reticle Texture ---
					if (outSights.m_sReticleTexture.IsEmpty())
					{
						string tex;
						if (cur.Get("m_rReticleTexture", tex) || cur.Get("m_sReticleTexture", tex) || cur.Get("ReticleTexture", tex) || cur.Get("m_sTexture", tex))
						{
							if (!tex.IsEmpty())
								outSights.m_sReticleTexture = TBD_EquipmentResourceNames.NormalizePathSeparators(tex);
						}
					}

					// Reticle child container inspection (Collimator reticles)
					BaseContainer retInfo = cur.GetObject("m_sReticle");
					if (!retInfo)
						retInfo = cur.GetObject("m_Reticle");
					if (!retInfo)
						retInfo = cur.GetObject("BaseCollimatorReticleInfo");

					if (retInfo)
					{
						if (outSights.m_sReticleTexture.IsEmpty())
						{
							string rTex;
							if (retInfo.Get("m_sReticleTexture", rTex) || retInfo.Get("m_rReticleTexture", rTex) || retInfo.Get("ReticleTexture", rTex))
							{
								if (!rTex.IsEmpty())
									outSights.m_sReticleTexture = TBD_EquipmentResourceNames.NormalizePathSeparators(rTex);
							}
						}
					}

					// ReticleInfos list inspection
					BaseContainerList retInfos = cur.GetObjectArray("ReticleInfos");
					if (retInfos && outSights.m_sReticleTexture.IsEmpty())
					{
						for (int ri = 0, rin = retInfos.Count(); ri < rin; ri++)
						{
							BaseContainer rObj = retInfos.Get(ri);
							if (rObj)
							{
								string riTex;
								if (rObj.Get("m_sReticleTexture", riTex) || rObj.Get("m_rReticleTexture", riTex) || rObj.Get("ReticleTexture", riTex))
								{
									if (!riTex.IsEmpty())
									{
										outSights.m_sReticleTexture = TBD_EquipmentResourceNames.NormalizePathSeparators(riTex);
										break;
									}
								}
							}
						}
					}

					// --- Reticle Colors (ReticleColor & GlowColor) ---
					if (!outSights.m_bHasReticleColor || !outSights.m_bHasGlowColor)
					{
						BaseContainerList retColors = cur.GetObjectArray("ReticleColors");
						if (retColors && retColors.Count() > 0)
						{
							BaseContainer rc = retColors.Get(0);
							if (rc)
							{
								if (!outSights.m_bHasReticleColor)
									outSights.m_bHasReticleColor = ExtractRgbaColor(rc, "ReticleColor", outSights.m_aReticleColor);

								if (!outSights.m_bHasGlowColor)
									outSights.m_bHasGlowColor = ExtractRgbaColor(rc, "GlowColor", outSights.m_aGlowColor);
							}
						}
					}

					// Fallback reticle color on retInfo
					if (retInfo && !outSights.m_bHasReticleColor)
						outSights.m_bHasReticleColor = ExtractRgbaColor(retInfo, "m_cReticleColor", outSights.m_aReticleColor);

					// --- Rangefinder ---
					if (!outSights.m_bHasRangefinderSet)
					{
						bool rf;
						if (cur.IsVariableSet("m_bHasRangefinder") && cur.Get("m_bHasRangefinder", rf))
						{
							outSights.m_bHasRangefinder = rf;
							outSights.m_bHasRangefinderSet = true;
						}
						else if (cur.IsVariableSet("m_bRangefinder") && cur.Get("m_bRangefinder", rf))
						{
							outSights.m_bHasRangefinder = rf;
							outSights.m_bHasRangefinderSet = true;
						}
						else if (cur.IsVariableSet("HasRangefinder") && cur.Get("HasRangefinder", rf))
						{
							outSights.m_bHasRangefinder = rf;
							outSights.m_bHasRangefinderSet = true;
						}
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// 3. Post-process magnification steps and min/max
		if (steps.Count() > 0)
		{
			steps.Sort();
			outSights.m_aMagnificationSteps = steps;
			outSights.m_bHasMagnificationSteps = true;

			if (!outSights.m_bHasMagnificationMin)
			{
				outSights.m_fMagnificationMin = steps[0];
				outSights.m_bHasMagnificationMin = true;
			}
			if (!outSights.m_bHasMagnificationMax)
			{
				outSights.m_fMagnificationMax = steps[steps.Count() - 1];
				outSights.m_bHasMagnificationMax = true;
			}
		}

		if (outSights.m_bHasMagnificationMax && (outSights.m_fMagnificationMax > 1.05 || (outSights.m_bHasMagnificationMin && outSights.m_fMagnificationMin > 1.05)))
		{
			outSights.m_bIsMagnified = true;
		}
		else
		{
			outSights.m_bIsMagnified = false;
		}

		// 4. Component-level rangefinder check if not explicitly set on sights container
		if (!outSights.m_bHasRangefinderSet)
		{
			if (TBD_EquipmentComponentGraph.HasCompSuffix(comps, "RangefinderComponent") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SCR_RangefinderComponent"))
			{
				outSights.m_bHasRangefinder = true;
				outSights.m_bHasRangefinderSet = true;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract 4-component RGBA color array from BaseContainer property string or vector.
	protected static bool ExtractRgbaColor(BaseContainer container, string propName, notnull array<float> outColor)
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
				else
					outColor.Insert(1.0);

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
			outColor.Insert(1.0);
			return true;
		}

		return false;
	}
}
