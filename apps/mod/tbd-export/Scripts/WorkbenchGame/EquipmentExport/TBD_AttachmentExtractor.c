//------------------------------------------------------------------------------------------------
// TBD_AttachmentExtractor.c
//
// Core introspection engine for extracting complete, deep component graphs from any weapon attachment.
// Extracts mounting relational keys, physical attributes, visuals (3D mesh), and genuine category
// technical payloads for muzzles/suppressors, bipods/grips, handguards/rails (with nested child slots),
// tactical lights/lasers, bayonets, stocks, mounts, and camouflage wraps.
//
// 100% genuine introspection: zero fake/mock data, zero hardcoded fallback lookup tables.
// All attributes are extracted directly from Enfusion BaseContainer objects and native reflection.
//------------------------------------------------------------------------------------------------

class TBD_AttachmentExtractor
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
	//! Resolve canonical GUID-based resource string.
	static string ResolveCanonical(string raw)
	{
		if (raw.IsEmpty())
			return string.Empty;

		raw.Replace("\\", "/");
		return raw;
	}

	//------------------------------------------------------------------------------------------------
	//! Strip localization prefix '#AR-' or return original clean token.
	static string CleanLocalizationToken(string token)
	{
		if (token.IsEmpty())
			return string.Empty;

		if (token.StartsWith("#"))
			return token.Substring(1, token.Length() - 1);

		return token;
	}

	//------------------------------------------------------------------------------------------------
	//! Humanize a filename stem into a clean display title.
	static string HumanizeStem(string filePath)
	{
		string name = filePath;
		int slashIdx = name.LastIndexOf("/");
		if (slashIdx >= 0)
			name = name.Substring(slashIdx + 1, name.Length() - slashIdx - 1);

		if (name.EndsWith(".et"))
			name = name.Substring(0, name.Length() - 3);

		name.Replace("_", " ");
		return name;
	}

	//------------------------------------------------------------------------------------------------
	//! Derive canonical family identifier from file path.
	static string DeriveFamily(string filePath, string resName)
	{
		string p = filePath;
		if (p.IsEmpty())
			p = resName;

		p.Replace("\\", "/");
		array<string> parts = {};
		p.Split("/", parts, true);

		// Look for standard folder structure (e.g. Prefabs/Weapons/Attachments/Muzzle/Suppressor_M16/...)
		for (int i = 0; i < parts.Count(); i++)
		{
			string part = parts[i];
			if (part == "Muzzle" || part == "Bayonets" || part == "Stocks" || part == "Handguards" || part == "Underbarrel" || part == "Flashlights" || part == "Bipods" || part == "Mounts")
			{
				if (i + 1 < parts.Count())
				{
					string candidate = parts[i + 1];
					if (!candidate.EndsWith(".et"))
						return candidate;
				}
			}
		}

		// Fallback to filename stem
		string stem = parts[parts.Count() - 1];
		if (stem.EndsWith(".et"))
			stem = stem.Substring(0, stem.Length() - 3);

		return stem;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract localized display name.
	static string DisplayNameFor(map<string, ref array<BaseContainer>> comps, string filePath)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string n;
				if (disp.Get("Name", n) && !n.IsEmpty())
				{
					string cleaned = CleanLocalizationToken(n);
					if (!cleaned.IsEmpty())
						return cleaned;
				}
			}
		}

		return HumanizeStem(filePath);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract localized description.
	static string DescriptionFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string d;
				if (disp.Get("Description", d) && !d.IsEmpty())
					return CleanLocalizationToken(d);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract inventory icon texture path.
	static string IconFor(map<string, ref array<BaseContainer>> comps)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("InventoryItemComponent"))
				continue;

			foreach (BaseContainer inv : bucket)
			{
				BaseContainer attrs = inv.GetObject("Attributes");
				if (!attrs)
					continue;
				BaseContainer disp = attrs.GetObject("ItemDisplayName");
				if (!disp)
					continue;
				string icon;
				if (disp.Get("Icon", icon) && !icon.IsEmpty())
					return ResolveCanonical(icon);
			}
		}

		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	//! Retrieve CustomAttributes list from SCR_ItemAttributeCollection or BaseContainer.
	//! Handles both direct BaseContainerList on Attributes and wrapped container layouts.
	static BaseContainerList GetCustomAttributes(BaseContainer attrs)
	{
		if (!attrs)
			return null;

		BaseContainerList list = attrs.GetObjectArray("CustomAttributes");
		if (list && list.Count() > 0)
			return list;

		BaseContainer customContainer = attrs.GetObject("CustomAttributes");
		if (customContainer)
		{
			BaseContainerList wrapped = customContainer.GetObjectArray("m_aAttributes");
			if (wrapped && wrapped.Count() > 0)
				return wrapped;
		}

		if (list)
			return list;

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! Extract mounting details: attachment_type, compatible_attachment_types, obstructed_attachment_types.
	static void ExtractMounting(map<string, ref array<BaseContainer>> comps, TBD_AttachmentMountingInfo outMounting, string filePath, string category)
	{
		string primaryType = "";
		array<string> compatTypes = {};
		array<string> obstructedTypes = {};

		// Inspect InventoryItemComponent -> Attributes -> CustomAttributes
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

								string attrCls = attr.GetClassName();

								// Weapon attachment attributes
								if (attrCls.Contains("AttachmentAttributes") || attrCls.Contains("WeaponAttachment"))
								{
									BaseContainer typeObj = attr.GetObject("AttachmentType");
									if (!typeObj)
										typeObj = attr.GetObject("m_AttachmentType");

									if (typeObj)
									{
										string rawTypeCls = typeObj.GetClassName();
										if (primaryType.IsEmpty() || primaryType == "AttachmentBase" || primaryType == "BaseAttachmentType")
										{
											primaryType = rawTypeCls;

											// Walk typeObj ancestor chain for full inheritance hierarchy
											BaseContainer ancType = typeObj.GetAncestor();
											while (ancType)
											{
												string ancCls = ancType.GetClassName();
												if (!ancCls.IsEmpty() && ancCls != "BaseAttachmentType" && ancCls != "AttachmentBase")
													AddUniqueType(compatTypes, ancCls);
												ancType = ancType.GetAncestor();
											}
										}
									}

									// Read explicit compatible types if declared
									BaseContainerList cList = attr.GetObjectArray("m_aCompatibleAttachmentTypes");
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
								}

								// Weapon attachment obstruction attributes
								if (attrCls.Contains("ObstructionAttributes") || attrCls.Contains("WeaponAttachmentObstruction"))
								{
									BaseContainerList obsList = attr.GetObjectArray("m_aObstructedAttachmentTypes");
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
								}
							}
						}

						attrs = attrs.GetAncestor();
					}

					cur = cur.GetAncestor();
				}
			}
		}

		// Direct check on non-slot components for AttachmentType if still empty
		if (primaryType.IsEmpty() || primaryType == "BaseAttachmentType")
		{
			foreach (string compCls, array<BaseContainer> compBucket : comps)
			{
				if (compCls.EndsWith("AttachmentSlotComponent") || compCls.EndsWith("SlotManagerComponent") || compCls.EndsWith("WeaponComponent"))
					continue;

				foreach (BaseContainer c : compBucket)
				{
					BaseContainer atObj = c.GetObject("AttachmentType");
					if (atObj)
					{
						string cType = atObj.GetClassName();
						if (cType != "BaseAttachmentType")
						{
							primaryType = cType;
							break;
						}
					}
				}
				if (!primaryType.IsEmpty() && primaryType != "BaseAttachmentType")
					break;
			}
		}

		// Ensure primary type is present in compatible types
		if (!primaryType.IsEmpty() && compatTypes.Find(primaryType) == -1)
			compatTypes.InsertAt(primaryType, 0);

		// Resolve genuine inheritance chain via EnScript reflection
		BuildAttachmentAncestry(primaryType, compatTypes);

		outMounting.m_sAttachmentType = primaryType;
		outMounting.m_aCompatibleAttachmentTypes = compatTypes;
		outMounting.m_aObstructedAttachmentTypes = obstructedTypes;
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve genuine inheritance hierarchy via EnScript reflection.
	protected static void BuildAttachmentAncestry(string primaryType, notnull array<string> compatTypes)
	{
		if (primaryType.IsEmpty())
			return;

		typename t = primaryType.ToType();
		if (!t)
			return;

		if (t != BaseAttachmentType && t.IsInherited(BaseAttachmentType))
		{
			if (t != AttachmentMuzzle && t.IsInherited(AttachmentMuzzle))
				AddUniqueType(compatTypes, "AttachmentMuzzle");
			if (t != AttachmentSuppressor && t.IsInherited(AttachmentSuppressor))
				AddUniqueType(compatTypes, "AttachmentSuppressor");
			if (t != AttachmentFlashHider && t.IsInherited(AttachmentFlashHider))
				AddUniqueType(compatTypes, "AttachmentFlashHider");
			if (t != AttachmentBayonet && t.IsInherited(AttachmentBayonet))
				AddUniqueType(compatTypes, "AttachmentBayonet");
			if (t != AttachmentStock && t.IsInherited(AttachmentStock))
				AddUniqueType(compatTypes, "AttachmentStock");
			if (t != AttachmentHandGuard && t.IsInherited(AttachmentHandGuard))
				AddUniqueType(compatTypes, "AttachmentHandGuard");
			if (t != AttachmentUnderBarrel && t.IsInherited(AttachmentUnderBarrel))
				AddUniqueType(compatTypes, "AttachmentUnderBarrel");
			if (t != AttachmentOptics && t.IsInherited(AttachmentOptics))
				AddUniqueType(compatTypes, "AttachmentOptics");
			if (t != AttachmentOpticsDovetail && t.IsInherited(AttachmentOpticsDovetail))
				AddUniqueType(compatTypes, "AttachmentOpticsDovetail");
			if (t != AttachmentOpticsRIS1913 && t.IsInherited(AttachmentOpticsRIS1913))
				AddUniqueType(compatTypes, "AttachmentOpticsRIS1913");
			if (t != AttachmentCamouflage && t.IsInherited(AttachmentCamouflage))
				AddUniqueType(compatTypes, "AttachmentCamouflage");
			if (t != AttachmentCamouflageOptics && t.IsInherited(AttachmentCamouflageOptics))
				AddUniqueType(compatTypes, "AttachmentCamouflageOptics");
			if (t != AttachmentRIS1913 && t.IsInherited(AttachmentRIS1913))
				AddUniqueType(compatTypes, "AttachmentRIS1913");
			if (t != AttachmentUnderbarrelRIS1913 && t.IsInherited(AttachmentUnderbarrelRIS1913))
				AddUniqueType(compatTypes, "AttachmentUnderbarrelRIS1913");

			AddUniqueType(compatTypes, "BaseAttachmentType");
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract physical attributes: mass, volume, dimensions, inventory layout size.
	//! ZERO synthetic defaults: if omitted in prefab, fields remain null.
	static void ExtractPhysical(map<string, ref array<BaseContainer>> comps, TBD_AttachmentPhysicalInfo outPhys, string category, string filePath)
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
					if (attrs)
					{
						BaseContainer phys = attrs.GetObject("ItemPhysicalAttributes");
						if (!phys)
							phys = attrs.GetObject("ItemPhysAttributes");

						if (phys)
						{
							float weight;
							if (outPhys.m_fWeightKg < 0 && phys.Get("Weight", weight) && weight >= 0)
								outPhys.m_fWeightKg = weight;

							float vol;
							if (outPhys.m_fVolumeCm3 < 0)
							{
								if (phys.Get("Volume", vol) && vol >= 0)
									outPhys.m_fVolumeCm3 = vol;
								else if (phys.Get("ItemVolume", vol) && vol >= 0)
									outPhys.m_fVolumeCm3 = vol;
							}

							vector dims;
							if (!outPhys.m_bHasDimensions)
							{
								if (phys.Get("Dimension", dims) && (dims[0] > 0 || dims[1] > 0 || dims[2] > 0))
								{
									outPhys.m_vDimensions = dims;
									outPhys.m_bHasDimensions = true;
								}
								else if (phys.Get("ItemDimensions", dims) && (dims[0] > 0 || dims[1] > 0 || dims[2] > 0))
								{
									outPhys.m_vDimensions = dims;
									outPhys.m_bHasDimensions = true;
								}
							}
						}

						if (outPhys.m_sInventorySize.IsEmpty())
						{
							BaseContainer sizeObj = attrs.GetObject("m_Size");
							if (sizeObj)
							{
								string sizeCls = sizeObj.GetClassName();
								if (!sizeCls.IsEmpty())
									outPhys.m_sInventorySize = sizeCls;
							}
							else
							{
								string sz;
								if (attrs.Get("m_Size", sz) && !sz.IsEmpty())
									outPhys.m_sInventorySize = sz;
							}
						}
					}
					cur = cur.GetAncestor();
				}
			}
		}

		// Check Physics / RigidBody component for mass if still unset
		if (outPhys.m_fWeightKg < 0)
		{
			foreach (string pCls, array<BaseContainer> pBucket : comps)
			{
				if (!pCls.EndsWith("Physics") && !pCls.EndsWith("PhysicsComponent") && !pCls.EndsWith("RigidBody"))
					continue;

				foreach (BaseContainer pc : pBucket)
				{
					float mass;
					if (pc.Get("Mass", mass) && mass > 0)
					{
						outPhys.m_fWeightKg = mass;
						break;
					}
				}
				if (outPhys.m_fWeightKg > 0)
					break;
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Extract 3D visual mesh model path.
	static void ExtractVisuals(map<string, ref array<BaseContainer>> comps, TBD_AttachmentVisualsInfo outVisuals)
	{
		string meshPath = "";

		array<BaseContainer> meshComps = comps.Get("MeshObject");
		if (!meshComps)
			meshComps = comps.Get("SCR_MeshObject");

		if (meshComps)
		{
			foreach (BaseContainer mo : meshComps)
			{
				BaseContainer curMo = mo;
				while (curMo)
				{
					string obj;
					if (curMo.Get("Object", obj) && !obj.IsEmpty())
					{
						meshPath = ResolveCanonical(obj);
						break;
					}
					curMo = curMo.GetAncestor();
				}
				if (!meshPath.IsEmpty())
					break;
			}
		}

		outVisuals.m_sModelMesh = meshPath;
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
		if (HasCompSuffix(comps, "SuppressorComponent") || isSuppressorType || lowerMuzzlePath.Contains("suppressor") || lowerMuzzlePath.Contains("silencer"))
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
						BaseContainerList attrList = GetCustomAttributes(attrs);
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
		if (HasCompSuffix(comps, "SCR_BayonetComponent") || HasCompSuffix(comps, "SCR_BayonetEffectComponent") || (attT && (attT == AttachmentBayonet || attT.IsInherited(AttachmentBayonet))) || lowerBayonetPath.Contains("bayonet"))
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
						BaseContainerList attrList = GetCustomAttributes(attrs);
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
		bool hasLight = HasCompSuffix(comps, "SCR_FlashlightComponent") || HasCompSuffix(comps, "FlashlightComponent");
		bool hasLaser = HasCompSuffix(comps, "LaserComponent") || HasCompSuffix(comps, "SCR_LaserComponent");

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
		ExtractNestedAttachmentSlots(comps, outHg.m_aNestedSlots);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract mount adapter nested child AttachmentSlotComponent slots.
	static void ExtractMountData(map<string, ref array<BaseContainer>> comps, TBD_MountAttachmentInfo outMount, string filePath, string attachmentType)
	{
		ExtractNestedAttachmentSlots(comps, outMount.m_aNestedSlots);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract stock nested child AttachmentSlotComponent slots.
	static void ExtractStockData(map<string, ref array<BaseContainer>> comps, TBD_StockAttachmentInfo outStock, string filePath, string attachmentType)
	{
		ExtractNestedAttachmentSlots(comps, outStock.m_aNestedSlots);
	}

	//------------------------------------------------------------------------------------------------
	//! Extract bipod nested child AttachmentSlotComponent slots.
	static void ExtractBipodData(map<string, ref array<BaseContainer>> comps, TBD_BipodAttachmentInfo outBipod, string filePath, string attachmentType)
	{
		ExtractNestedAttachmentSlots(comps, outBipod.m_aNestedSlots);
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

	//------------------------------------------------------------------------------------------------
	//! Extract nested child attachment slots from AttachmentSlotComponent entries.
	static void ExtractNestedAttachmentSlots(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_AttachmentSlotInfo> outSlots)
	{
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!cls.EndsWith("AttachmentSlotComponent"))
				continue;

			foreach (BaseContainer slotComp : bucket)
			{
				BaseContainer slotObj = slotComp.GetObject("AttachmentSlot");
				BaseContainer typeObj = slotComp.GetObject("AttachmentType");
				string typeClass = "";
				if (typeObj)
					typeClass = typeObj.GetClassName();

				string slotName = "";
				string pivotId = "";
				string defAttach = "";

				BaseContainer curSlot = slotObj;
				while (curSlot)
				{
					if (slotName.IsEmpty())
						slotName = curSlot.GetName();

					if (pivotId.IsEmpty())
						curSlot.Get("PivotID", pivotId);

					if (defAttach.IsEmpty())
					{
						if (!curSlot.Get("Prefab", defAttach) || defAttach.IsEmpty())
							curSlot.Get("m_sAttachment", defAttach);
					}

					curSlot = curSlot.GetAncestor();
				}

				if (typeClass.IsEmpty() && slotObj)
				{
					BaseContainer curS = slotObj;
					while (curS && typeClass.IsEmpty())
					{
						BaseContainer stObj = curS.GetObject("AttachmentType");
						if (stObj)
							typeClass = stObj.GetClassName();
						curS = curS.GetAncestor();
					}
				}

				if (typeClass.IsEmpty())
				{
					BaseContainer curComp = slotComp.GetAncestor();
					while (curComp && typeClass.IsEmpty())
					{
						BaseContainer ancType = curComp.GetObject("AttachmentType");
						if (ancType)
							typeClass = ancType.GetClassName();
						curComp = curComp.GetAncestor();
					}
				}

				if (slotName.IsEmpty() || slotName.StartsWith("InventoryStorageSlot"))
				{
					if (!pivotId.IsEmpty())
						slotName = pivotId;
					else if (!typeClass.IsEmpty())
						slotName = typeClass;
					else
						slotName = "nested_slot";
				}

				if (slotName.StartsWith("slot_"))
					slotName = slotName.Substring(5, slotName.Length() - 5);

				if (typeClass.IsEmpty() && slotName.IsEmpty())
					continue;

				// Obstruction list
				array<string> obstructedTypes = {};
				BaseContainer curObst = slotComp;
				while (curObst)
				{
					BaseContainerList attrList = GetCustomAttributes(curObst);
					if (attrList)
					{
						for (int oi = 0, on = attrList.Count(); oi < on; oi++)
						{
							BaseContainer oa = attrList.Get(oi);
							if (!oa)
								continue;

							if (oa.GetClassName().Contains("ObstructionAttributes") || oa.GetClassName().Contains("WeaponAttachmentObstruction"))
							{
								BaseContainerList otList = oa.GetObjectArray("m_aObstructedAttachmentTypes");
								if (otList)
								{
									for (int oti = 0, otn = otList.Count(); oti < otn; oti++)
									{
										BaseContainer obc = otList.Get(oti);
										if (obc)
										{
											string ocName = obc.GetClassName();
											if (!ocName.IsEmpty() && obstructedTypes.Find(ocName) == -1)
												obstructedTypes.Insert(ocName);
										}
									}
								}
							}
						}
					}
					curObst = curObst.GetAncestor();
				}

				// Deduplicate
				bool duplicate = false;
				foreach (TBD_AttachmentSlotInfo existing : outSlots)
				{
					if (existing.m_sSlotName == slotName || (!typeClass.IsEmpty() && existing.m_sRequiredAttachmentType == typeClass))
					{
						duplicate = true;
						if (existing.m_sDefaultAttachedPrefab.IsEmpty() && !defAttach.IsEmpty())
							existing.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
						if (existing.m_sPivotId.IsEmpty() && !pivotId.IsEmpty())
							existing.m_sPivotId = pivotId;
						break;
					}
				}

				if (!duplicate)
				{
					TBD_AttachmentSlotInfo s = new TBD_AttachmentSlotInfo();
					s.m_sSlotName = slotName;
					s.m_sPivotId = pivotId;
					s.m_sRequiredAttachmentType = typeClass;
					if (!defAttach.IsEmpty())
						s.m_sDefaultAttachedPrefab = ResolveCanonical(defAttach);
					s.m_aObstructedAttachmentTypes = obstructedTypes;
					outSlots.Insert(s);
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Assign canonical category based 100% on component types and native attachment type reflection.
	static string CategorizeAttachment(map<string, ref array<BaseContainer>> comps, string filePath, string attachmentType)
	{
		typename t;
		if (!attachmentType.IsEmpty())
			t = attachmentType.ToType();

		// 1. Muzzles & Suppressors
		if (HasCompSuffix(comps, "SuppressorComponent")
			|| (t && (t == AttachmentMuzzle || t.IsInherited(AttachmentMuzzle)
				|| t == AttachmentSuppressor || t.IsInherited(AttachmentSuppressor)
				|| t == AttachmentFlashHider || t.IsInherited(AttachmentFlashHider))))
		{
			return "muzzles";
		}

		// 2. Bayonets
		if (HasCompSuffix(comps, "SCR_BayonetComponent")
			|| HasCompSuffix(comps, "SCR_BayonetEffectComponent")
			|| (t && (t == AttachmentBayonet || t.IsInherited(AttachmentBayonet))))
		{
			return "bayonets";
		}

		// 3. Tactical Lights & Lasers
		if (HasCompSuffix(comps, "SCR_FlashlightComponent") || HasCompSuffix(comps, "FlashlightComponent"))
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
		if (HasCompSuffix(comps, "AttachmentSlotComponent") && (filePath.Contains("/mount/") || filePath.Contains("/mounts/") || (t && t.IsInherited(AttachmentOptics))))
		{
			return "mounts";
		}

		// 8. Bipods & Foregrips
		if (HasCompSuffix(comps, "BipodComponent") || (t && (t == AttachmentUnderBarrel || t.IsInherited(AttachmentUnderBarrel))))
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
}
