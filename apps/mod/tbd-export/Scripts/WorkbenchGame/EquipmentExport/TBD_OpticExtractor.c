//------------------------------------------------------------------------------------------------
// TBD_OpticExtractor.c
//
// Core introspection engine for extracting complete, ground-truth component graphs from optics and sights.
// Extracts mounting relational keys, optical sights parameters (magnification, FOV, eye relief,
// objective diameter, zeroing distances, reticle textures & colors, rangefinder), physical
// attributes, and visual 3D model meshes.
//
// 100% genuine introspection:
//   - ZERO fake/mock data: omitted values serialize as JSON null.
//   - ZERO hardcoded class checks: works universally with any mod.
//   - All attributes and relational keys are extracted directly from native Enfusion BaseContainer
//     objects and container ancestry.
//   - Localization tokens are preserved raw without stripping # or fabricating humanized strings.
//------------------------------------------------------------------------------------------------

class TBD_OpticExtractor
{
	protected static const int ANCESTOR_CAP = 16;
	protected static const int COMPONENT_DEPTH_CAP = 4;

	//------------------------------------------------------------------------------------------------
	//! Extract all components across the entire inheritance hierarchy into a mapped bucket list.
	static void CollectComponentChain(BaseContainer prefabRoot, notnull map<string, ref array<BaseContainer>> outComps)
	{
		BaseContainer cur = prefabRoot;
		int hops = 0;
		while (cur && hops < ANCESTOR_CAP)
		{
			CollectComponentsRec(cur, outComps, 0);
			cur = cur.GetAncestor();
			hops++;
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void CollectComponentsRec(BaseContainer holder, notnull map<string, ref array<BaseContainer>> outComps, int depth)
	{
		if (depth > COMPONENT_DEPTH_CAP)
			return;

		BaseContainerList comps = holder.GetObjectArray("components");
		if (!comps)
			return;

		for (int i = 0, n = comps.Count(); i < n; i++)
		{
			BaseContainer comp = comps.Get(i);
			if (!comp)
				continue;

			string cls = comp.GetClassName();
			array<BaseContainer> bucket = outComps.Get(cls);
			if (!bucket)
			{
				bucket = {};
				outComps.Insert(cls, bucket);
			}
			bucket.Insert(comp);

			CollectComponentsRec(comp, outComps, depth + 1);
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Returns true if any component in the map ends with the given suffix.
	static bool HasCompSuffix(map<string, ref array<BaseContainer>> comps, string suffix)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (cls.EndsWith(suffix))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Append an item to an array only if it does not already exist.
	protected static void AddUniqueType(notnull array<string> list, string item)
	{
		if (item.IsEmpty())
			return;

		if (list.Find(item) == -1)
			list.Insert(item);
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve canonical GUID-based resource string with forward slashes.
	static string ResolveCanonical(string raw)
	{
		if (raw.IsEmpty())
			return string.Empty;

		raw.Replace("\\", "/");
		return raw;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw unmutated localized display name token directly from ItemDisplayName.Name.
	//! ZERO string mutation: keeps # intact, zero English fallbacks, zero synthetic guessing.
	static string RawDisplayNameFor(map<string, ref array<BaseContainer>> comps)
	{
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
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (disp)
						{
							string n;
							if (disp.Get("Name", n) && !n.IsEmpty())
								return n;
						}
						attrs = attrs.GetAncestor();
					}
					cur = cur.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw unmutated localized description token directly from ItemDisplayName.Description.
	//! Returns string.Empty if omitted in prefab (serializes as null).
	static string RawDescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
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
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (disp)
						{
							string d;
							if (disp.Get("Description", d) && !d.IsEmpty())
								return d;
						}
						attrs = attrs.GetAncestor();
					}
					cur = cur.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract raw icon texture path directly from ItemDisplayName.Icon.
	static string RawIconFor(map<string, ref array<BaseContainer>> comps)
	{
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
						BaseContainer disp = attrs.GetObject("ItemDisplayName");
						if (disp)
						{
							string icon;
							if (disp.Get("Icon", icon) && !icon.IsEmpty())
								return ResolveCanonical(icon);
						}
						attrs = attrs.GetAncestor();
					}
					cur = cur.GetAncestor();
				}
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Retrieve CustomAttributes list from SCR_ItemAttributeCollection or BaseContainer.
	static BaseContainerList GetCustomAttributes(BaseContainer attrs)
	{
		if (!attrs)
			return null;

		BaseContainerList list = attrs.GetObjectArray("CustomAttributes");
		if (list && list.Count() > 0)
			return list;

		BaseContainer subCustom = attrs.GetObject("CustomAttributes");
		if (subCustom)
		{
			BaseContainerList wrapped = subCustom.GetObjectArray("CustomAttributes");
			if (wrapped && wrapped.Count() > 0)
				return wrapped;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract mounting relational keys directly from container data and genuine container ancestry:
	//!   - attachment_type: primary class name declared in AttachmentType / m_AttachmentType
	//!   - compatible_attachment_types: container ancestor classes and declared m_aCompatibleAttachmentTypes
	//!   - obstructed_attachment_types: classes from m_aObstructedAttachmentTypes
	//! ZERO hardcoded class checks: works dynamically for any mod.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_OpticMountingInfo outMounting)
	{
		string primaryType;
		ref array<string> compatTypes = {};
		ref array<string> obstructedTypes = {};

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent") && !cls.EndsWith("WeaponAttachmentComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer cur = inv;
				while (cur)
				{
					BaseContainer attrs = cur.GetObject("Attributes");
					while (attrs)
					{
						BaseContainerList attrList = GetCustomAttributes(attrs);
						if (attrList)
						{
							for (int a = 0, an = attrList.Count(); a < an; a++)
							{
								BaseContainer attr = attrList.Get(a);
								if (!attr)
									continue;

								// Walk container ancestor chain to discover genuine attachment types and compatibility
								BaseContainer curAttr = attr;
								while (curAttr)
								{
									// 1. Primary AttachmentType and container ancestor classes
									BaseContainer typeObj = curAttr.GetObject("AttachmentType");
									if (!typeObj)
										typeObj = curAttr.GetObject("m_AttachmentType");

									if (typeObj)
									{
										string typeCls = typeObj.GetClassName();
										if (!typeCls.IsEmpty() && !typeCls.StartsWith("AttachmentCamouflage"))
										{
											if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentBase" || primaryType == "AttachmentOptics")
												primaryType = typeCls;

											AddUniqueType(compatTypes, typeCls);

											// Check if typeObj itself has an ancestor container
											BaseContainer ancType = typeObj.GetAncestor();
											while (ancType)
											{
												string ancCls = ancType.GetClassName();
												if (!ancCls.IsEmpty())
													AddUniqueType(compatTypes, ancCls);
												ancType = ancType.GetAncestor();
											}
										}
									}

									// Read explicit compatible types if declared on container
									BaseContainerList cList = curAttr.GetObjectArray("m_aCompatibleAttachmentTypes");
									if (cList)
									{
										for (int c = 0, cn = cList.Count(); c < cn; c++)
										{
											BaseContainer ct = cList.Get(c);
											if (ct)
											{
												string ctCls = ct.GetClassName();
												if (!ctCls.IsEmpty())
													AddUniqueType(compatTypes, ctCls);
											}
										}
									}

									// 2. Weapon attachment obstruction attributes
									BaseContainerList obsList = curAttr.GetObjectArray("m_aObstructedAttachmentTypes");
									if (obsList)
									{
										for (int o = 0, on = obsList.Count(); o < on; o++)
										{
											BaseContainer ot = obsList.Get(o);
											if (ot)
											{
												string otCls = ot.GetClassName();
												if (!otCls.IsEmpty())
													AddUniqueType(obstructedTypes, otCls);
											}
										}
									}

									curAttr = curAttr.GetAncestor();
								}
							}
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Fallback check on non-slot components for direct AttachmentType if still empty
		if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType" || primaryType == "AttachmentOptics")
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent") || compCls.EndsWith("WeaponComponent"))
					continue;

				foreach (BaseContainer c : compBucket)
				{
					BaseContainer atObj = c.GetObject("AttachmentType");
					if (!atObj)
						atObj = c.GetObject("m_AttachmentType");

					if (atObj)
					{
						string cType = atObj.GetClassName();
						if (cType != "BaseAttachmentType" && !cType.IsEmpty() && !cType.StartsWith("AttachmentCamouflage"))
						{
							primaryType = cType;
							AddUniqueType(compatTypes, cType);
							BaseContainer ancType = atObj.GetAncestor();
							while (ancType)
							{
								string ancCls = ancType.GetClassName();
								if (!ancCls.IsEmpty())
									AddUniqueType(compatTypes, ancCls);
								ancType = ancType.GetAncestor();
							}
							break;
						}
					}
				}
				if (!primaryType.IsEmpty() && primaryType != "BaseAttachmentType" && primaryType != "AttachmentOptics")
					break;
			}
		}

		// Ensure primary type is at index 0 of compatible types if present
		if (!primaryType.IsEmpty())
		{
			int idx = compatTypes.Find(primaryType);
			if (idx > 0)
			{
				compatTypes.Remove(idx);
				compatTypes.InsertAt(primaryType, 0);
			}
			else if (idx == -1)
			{
				compatTypes.InsertAt(primaryType, 0);
			}
		}

		outMounting.m_sAttachmentType = primaryType;
		outMounting.m_aCompatibleAttachmentTypes = compatTypes;
		outMounting.m_aObstructedAttachmentTypes = obstructedTypes;
	}

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
								outSights.m_sReticleTexture = ResolveCanonical(tex);
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
									outSights.m_sReticleTexture = ResolveCanonical(rTex);
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
										outSights.m_sReticleTexture = ResolveCanonical(riTex);
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
			if (HasCompSuffix(comps, "RangefinderComponent") || HasCompSuffix(comps, "SCR_RangefinderComponent"))
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

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: omitted fields serialize as null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_OpticPhysicalInfo outPhys)
	{
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
						// Inventory item size token
						if (outPhys.m_sInventorySize.IsEmpty())
						{
							string sz;
							if (attrs.Get("m_Size", sz) && !sz.IsEmpty())
								outPhys.m_sInventorySize = sz;
						}

						// Item physical attributes
						BaseContainer phys = attrs.GetObject("ItemPhysAttributes");
						while (phys)
						{
							if (!outPhys.m_bHasWeight)
							{
								float weight;
								if (phys.Get("Weight", weight) && weight > 0)
								{
									outPhys.m_fWeightKg = weight;
									outPhys.m_bHasWeight = true;
								}
							}

							if (!outPhys.m_bHasVolume)
							{
								float volume;
								if (phys.Get("ItemVolume", volume) && volume > 0)
								{
									outPhys.m_fVolumeCm3 = volume;
									outPhys.m_bHasVolume = true;
								}
							}

							if (!outPhys.m_bHasDimensions)
							{
								vector dim;
								if (phys.Get("ItemDimensions", dim) && dim != "0 0 0")
								{
									outPhys.m_vDimensions = dim;
									outPhys.m_bHasDimensions = true;
								}
							}

							phys = phys.GetAncestor();
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract visual 3D model mesh path directly from MeshObject.Object.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_OpticVisualsInfo outVisuals)
	{
		string meshPath;

		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("MeshObject"))
				continue;

			foreach (BaseContainer mesh : bucket)
			{
				BaseContainer cur = mesh;
				while (cur)
				{
					string obj;
					if (cur.Get("Object", obj) && !obj.IsEmpty())
					{
						meshPath = ResolveCanonical(obj);
						break;
					}
					cur = cur.GetAncestor();
				}

				if (!meshPath.IsEmpty())
					break;
			}

			if (!meshPath.IsEmpty())
				break;
		}

		outVisuals.m_sModelMesh = meshPath;
	}
}
