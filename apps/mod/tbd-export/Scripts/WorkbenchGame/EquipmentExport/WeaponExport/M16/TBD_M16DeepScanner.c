/**
 * TBD_M16DeepScanner.c
 *
 * Builds the M16 compatibility matrix in two passes: one over every magazine and attachment in
 * every loaded addon to form the candidate pool, and one over every M16 variant. It then matches
 * each variant's magazine wells and slot types against that pool and serializes the result.
 *
 * TBD_M16Extractor reads a variant, TBD_M16CandidateExtractor reads a pool entry and owns the
 * rule that decides whether an attachment fits a slot, and TBD_M16Naming names all three. This
 * class owns the two sweeps, the pairing, and the JSON.
 */

class TBD_M16DeepScanner
{
	protected static const string TAG = "[TBD][M16Scanner]";
	protected static const int COMPONENT_DEPTH_CAP = 8;

	ref array<ref TBD_M16WeaponVariantInfo> m_aM16Variants = {};
	ref array<ref TBD_M16CandidateMagazine> m_aAllMagazines = {};
	ref array<ref TBD_M16CandidateAttachment> m_aAllAttachments = {};

	protected ref map<string, int> m_mMagIndexByResource = new map<string, int>();
	protected ref map<string, int> m_mAttachIndexByResource = new map<string, int>();

	protected string m_sCurrentAddonId;
	protected int m_iSeen = 0;

	//------------------------------------------------------------------------------------------------
	//! Execute full discovery: scans all addons for magazines & attachments, then inspects all M16s.
	bool RunDeepScan()
	{
		int startMs = System.GetTickCount();
		Print(TAG + " Starting deep discovery across loaded addons...", LogLevel.NORMAL);

		// 1. Build global pool of all magazines and attachments
		CollectGlobalPool();

		// 2. Discover and inspect all M16 prefabs
		CollectM16Variants();

		// 3. Match compatibility
		ResolveCompatibilities();

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("%1 Deep scan finished in %2 ms: %3 M16 variants, %4 magazines in pool, %5 attachments in pool",
			TAG, elapsedMs, m_aM16Variants.Count(), m_aAllMagazines.Count(), m_aAllAttachments.Count()), LogLevel.NORMAL);

		return !m_aM16Variants.IsEmpty();
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 1: Collect all magazines and attachment items across loaded addons.
	protected void CollectGlobalPool()
	{
		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;
			string rootPath = "$" + addonId + ":Prefabs";
			Workbench.SearchResources(OnPoolResourceFound, extEt, null, rootPath, true);
		}

		if (m_aAllMagazines.IsEmpty() && m_aAllAttachments.IsEmpty())
		{
			m_sCurrentAddonId = string.Empty;
			Workbench.SearchResources(OnPoolResourceFound, extEt, null, string.Empty, true);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void OnPoolResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		if (!path.Contains("Prefabs/"))
			return;

		// Skip non-item paths
		if (path.Contains("/Structures/") || path.Contains("/Rocks/") || path.Contains("/Trees/")
			|| path.Contains("/Foliage/") || path.Contains("Prefabs/Editor/"))
			return;

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
			return;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return;

		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return;

		string rootClass = root.GetClassName();
		if (rootClass == "SCR_ChimeraCharacter" || rootClass == "ChimeraCharacter" || rootClass == "Vehicle" || rootClass.EndsWith("Vehicle"))
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		string addonId = m_sCurrentAddonId;
		if (addonId.IsEmpty() && path.StartsWith("$"))
		{
			int colon = path.IndexOf(":");
			if (colon > 1)
				addonId = path.Substring(1, colon - 1);
		}

		// Check for Magazine
		bool hasMag = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "MagazineComponent");
		bool hasWeapon = TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent");
		if (hasMag && !hasWeapon)
		{
			if (!m_mMagIndexByResource.Contains(canonical))
			{
				string magWell = TBD_M16CandidateExtractor.ReadMagazineWell(comps);
				if (!magWell.IsEmpty())
				{
					TBD_M16CandidateMagazine mag = new TBD_M16CandidateMagazine();
					mag.m_sResourceName = canonical;
					mag.m_sDisplayName = TBD_M16Naming.DisplayNameFor(comps, path);
					mag.m_sMagWellClass = magWell;
					mag.m_iCapacity = TBD_M16CandidateExtractor.ReadMagazineCapacity(comps, path);
					mag.m_fWeightKg = TBD_M16CandidateExtractor.ReadWeight(comps);
					mag.m_sAddonId = addonId;

					m_mMagIndexByResource.Insert(canonical, m_aAllMagazines.Count());
					m_aAllMagazines.Insert(mag);
				}
			}
			return;
		}

		// Check for Attachment / Optic
		string attachType = TBD_M16CandidateExtractor.ReadAttachmentType(comps);
		if (!attachType.IsEmpty())
		{
			if (!m_mAttachIndexByResource.Contains(canonical))
			{
				TBD_M16CandidateAttachment att = new TBD_M16CandidateAttachment();
				att.m_sResourceName = canonical;
				att.m_sDisplayName = TBD_M16Naming.DisplayNameFor(comps, path);
				att.m_sAttachmentTypeClass = attachType;
				att.m_bIsOptic = (attachType.Contains("Optic") || TBD_EquipmentComponentGraph.HasCompSuffix(comps, "SightsComponent") || path.Contains("/Optic"));
				att.m_fWeightKg = TBD_M16CandidateExtractor.ReadWeight(comps);
				att.m_sAddonId = addonId;
				att.m_sFilePath = path;

				m_mAttachIndexByResource.Insert(canonical, m_aAllAttachments.Count());
				m_aAllAttachments.Insert(att);
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 2: Find all M16 variants and extract their deep properties.
	protected void CollectM16Variants()
	{
		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;
			string rootPath = "$" + addonId + ":Prefabs/Weapons/Rifles/M16";
			Workbench.SearchResources(OnM16ResourceFound, extEt, null, rootPath, true);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected void OnM16ResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

		if (!path.Contains("Rifle_M16"))
			return;

		Resource res = Resource.Load(resName);
		if (!res || !res.IsValid())
			return;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return;

		BaseContainer root = obj.ToBaseContainer();
		if (!root)
			return;

		map<string, ref array<BaseContainer>> comps = new map<string, ref array<BaseContainer>>();
		TBD_EquipmentComponentGraph.CollectComponentChain(root, comps, COMPONENT_DEPTH_CAP);

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		TBD_M16WeaponVariantInfo info = new TBD_M16WeaponVariantInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		info.m_sDisplayName = TBD_M16Naming.DisplayNameFor(comps, path);
		info.m_sAddonId = m_sCurrentAddonId;
		info.m_bIsAbstract = path.EndsWith("_base.et") || path.Contains("/base/");
		info.m_fWeightKg = TBD_M16CandidateExtractor.ReadWeight(comps);
		info.m_fVolumeCm3 = TBD_M16CandidateExtractor.ReadVolume(comps);

		// Ancestor / Variant Of
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Extract Sights
		TBD_M16Extractor.ExtractZeroingDistances(comps, info.m_aZeroingDistances);

		// Extract Muzzles
		TBD_M16Extractor.ExtractMuzzles(comps, info.m_aMuzzles);

		// Extract Attachment Slots
		TBD_M16Extractor.ExtractAttachmentSlots(comps, info.m_aAttachmentSlots);

		m_aM16Variants.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Phase 3: Match M16 muzzles & attachment slots with candidate items in global pool.
	protected void ResolveCompatibilities()
	{
		foreach (TBD_M16WeaponVariantInfo weapon : m_aM16Variants)
		{
			// 1. Match magazines per muzzle
			foreach (TBD_M16MuzzleInfo muzzle : weapon.m_aMuzzles)
			{
				foreach (TBD_M16CandidateMagazine mag : m_aAllMagazines)
				{
					if (muzzle.m_aMagazineWells.Find(mag.m_sMagWellClass) != -1)
					{
						muzzle.m_aCompatibleMagResourceNames.Insert(mag.m_sResourceName);
						muzzle.m_aCompatibleMagDisplayNames.Insert(mag.m_sDisplayName);
						muzzle.m_aCompatibleMagCapacities.Insert(mag.m_iCapacity);
					}
				}
			}

			// 2. Match attachments per slot
			foreach (TBD_M16AttachmentSlotInfo slot : weapon.m_aAttachmentSlots)
			{
				if (slot.m_sRequiredAttachType.IsEmpty())
					continue;

				foreach (TBD_M16CandidateAttachment att : m_aAllAttachments)
				{
					if (TBD_M16CandidateExtractor.AttachTypeFits(att.m_sAttachmentTypeClass, slot.m_sRequiredAttachType))
					{
						slot.m_aCompatibleItemResourceNames.Insert(att.m_sResourceName);
						slot.m_aCompatibleItemDisplayNames.Insert(att.m_sDisplayName);
					}
				}
			}
		}
	}

	//------------------------------------------------------------------------------------------------
	//! Serialize entire M16 dataset to JSON string buffer.
	string SerializeToJson()
	{
		string json = "{\n";
		json += "  \"version\": \"1\",\n";
		json += "  \"platform\": \"M16\",\n";
		json += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
		json += "  \"totalVariants\": " + m_aM16Variants.Count().ToString() + ",\n";
		json += "  \"weapons\": [\n";

		for (int w = 0; w < m_aM16Variants.Count(); w++)
		{
			TBD_M16WeaponVariantInfo var = m_aM16Variants[w];
			json += "    {\n";
			json += "      \"resource_name\": \"" + TBD_EquipmentExportJson.Escape(var.m_sResourceName) + "\",\n";
			json += "      \"id\": \"" + TBD_EquipmentExportJson.Escape(var.m_sId) + "\",\n";
			json += "      \"display_name\": \"" + TBD_EquipmentExportJson.Escape(var.m_sDisplayName) + "\",\n";
			json += "      \"addon\": \"" + TBD_EquipmentExportJson.Escape(var.m_sAddonId) + "\",\n";
			json += "      \"file_path\": \"" + TBD_EquipmentExportJson.Escape(var.m_sFilePath) + "\",\n";

			string isAbsStr = "false";
			if (var.m_bIsAbstract) isAbsStr = "true";
			json += "      \"is_abstract\": " + isAbsStr + ",\n";

			if (!var.m_sVariantOf.IsEmpty())
				json += "      \"variant_of\": \"" + TBD_EquipmentExportJson.Escape(var.m_sVariantOf) + "\",\n";

			if (var.m_fWeightKg >= 0)
				json += "      \"weight_kg\": " + var.m_fWeightKg.ToString() + ",\n";
			if (var.m_fVolumeCm3 >= 0)
				json += "      \"volume_cm3\": " + var.m_fVolumeCm3.ToString() + ",\n";

			// Zeroing
			json += "      \"zeroing_distances\": [";
			for (int zd = 0; zd < var.m_aZeroingDistances.Count(); zd++)
			{
				json += "\"" + var.m_aZeroingDistances[zd] + "\"";
				if (zd < var.m_aZeroingDistances.Count() - 1)
					json += ", ";
			}
			json += "],\n";

			// Muzzles
			json += "      \"muzzles\": [\n";
			for (int m = 0; m < var.m_aMuzzles.Count(); m++)
			{
				TBD_M16MuzzleInfo muz = var.m_aMuzzles[m];
				json += "        {\n";
				json += "          \"index\": " + muz.m_iIndex.ToString() + ",\n";
				json += "          \"muzzle_class\": \"" + muz.m_sMuzzleClass + "\",\n";

				// Magazine wells
				json += "          \"magazine_wells\": [";
				for (int mw = 0; mw < muz.m_aMagazineWells.Count(); mw++)
				{
					json += "\"" + muz.m_aMagazineWells[mw] + "\"";
					if (mw < muz.m_aMagazineWells.Count() - 1)
						json += ", ";
				}
				json += "],\n";

				if (!muz.m_sDefaultMagazineTemplate.IsEmpty())
					json += "          \"default_magazine\": \"" + TBD_EquipmentExportJson.Escape(muz.m_sDefaultMagazineTemplate) + "\",\n";

				// Fire modes
				json += "          \"fire_modes\": [";
				for (int f = 0; f < muz.m_aFireModeNames.Count(); f++)
				{
					json += "{\"name\":\"" + muz.m_aFireModeNames[f] + "\",\"burst\":" + muz.m_aFireModeBursts[f].ToString() + ",\"rpm\":" + muz.m_aFireModeRpms[f].ToString() + "}";
					if (f < muz.m_aFireModeNames.Count() - 1)
						json += ", ";
				}
				json += "]\n";

				json += "        }";
				if (m < var.m_aMuzzles.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ],\n";

			// Attachment slots
			json += "      \"attachment_slots\": [\n";
			for (int s = 0; s < var.m_aAttachmentSlots.Count(); s++)
			{
				TBD_M16AttachmentSlotInfo slot = var.m_aAttachmentSlots[s];
				json += "        {\n";
				json += "          \"slot_name\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sSlotName) + "\",\n";
				json += "          \"required_attachment_type\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sRequiredAttachType) + "\"";
				if (!slot.m_sDefaultAttachedPrefab.IsEmpty())
					json += ",\n          \"default_attachment\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sDefaultAttachedPrefab) + "\"\n";
				else
					json += ",\n          \"default_attachment\": null\n";

				json += "        }";
				if (s < var.m_aAttachmentSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ]\n";

			json += "    }";
			if (w < m_aM16Variants.Count() - 1)
				json += ",";
			json += "\n";
		}

		json += "  ]\n}\n";
		return json;
	}
}
