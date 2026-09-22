//------------------------------------------------------------------------------------------------
// TBD_RifleScanner.c
//
// Sweeps every loaded addon for rifle prefabs, has TBD_RifleExtractor and TBD_RifleNaming read
// each one, and serializes the result to rifles.json.
//
// The export carries no pre-resolved cross-product arrays. Magazine wells and required attachment
// types are written as foreign keys, so the platform links a rifle against the ammunition and
// attachment catalogs at query time rather than this scan baking the pairing in.
//------------------------------------------------------------------------------------------------

class TBD_RifleScanner
{
	protected static const int COMPONENT_DEPTH_CAP = 4;

	protected ref TBD_EquipmentExportConfig m_Config;
	protected ref array<ref TBD_RifleWeaponInfo> m_aRifles = {};
	protected string m_sCurrentAddonId;

	//------------------------------------------------------------------------------------------------
	void TBD_RifleScanner(TBD_EquipmentExportConfig config = null)
	{
		if (config)
			m_Config = config;
		else
			m_Config = new TBD_EquipmentExportConfig();
	}

	//------------------------------------------------------------------------------------------------
	int RunScan()
	{
		m_aRifles.Clear();

		Print("[TBD][RifleExport] Starting generic deep scan across all rifles...", LogLevel.NORMAL);
		int startMs = System.GetTickCount();

		array<string> guids = {};
		GameProject.GetLoadedAddons(guids);
		array<string> extEt = { "et" };

		foreach (string guid : guids)
		{
			string addonId = GameProject.GetAddonID(guid);
			m_sCurrentAddonId = addonId;
			string rootPath = "$" + addonId + ":Prefabs/Weapons/Rifles";
			Workbench.SearchResources(OnRifleResourceFound, extEt, null, rootPath, true);
		}

		int elapsedMs = System.GetTickCount() - startMs;
		Print(string.Format("[TBD][RifleExport] Completed in %1 ms. Discovered %2 rifle prefabs.", elapsedMs, m_aRifles.Count()), LogLevel.NORMAL);

		return m_aRifles.Count();
	}

	//------------------------------------------------------------------------------------------------
	protected void OnRifleResourceFound(ResourceName resName, string filePath = "")
	{
		string path = filePath;
		if (path.IsEmpty())
			path = resName;

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

		// Must carry WeaponComponent
		if (!TBD_EquipmentComponentGraph.HasCompSuffix(comps, "WeaponComponent"))
			return;

		string canonical = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(root.GetResourceName());
		if (canonical.IsEmpty())
			canonical = resName;

		TBD_RifleWeaponInfo info = new TBD_RifleWeaponInfo();
		info.m_sResourceName = canonical;
		info.m_sFilePath = path;
		info.m_sId = TBD_EquipmentResourceNames.GenerateSlug(path);
		info.m_sDisplayName = TBD_RifleNaming.DisplayNameFor(comps, path);
		info.m_sFamily = TBD_RifleNaming.ExtractFamily(path);
		info.m_sAddonId = m_sCurrentAddonId;
		info.m_bIsAbstract = path.EndsWith("_base.et") || path.Contains("/base/");
		info.m_fWeightKg = TBD_RifleExtractor.ReadWeight(comps);
		info.m_fVolumeCm3 = TBD_RifleExtractor.ReadVolume(comps);

		// Ancestor / Variant Of
		BaseContainer anc = root.GetAncestor();
		if (anc)
		{
			string ancRn = TBD_EquipmentResourceNames.ResolveCanonicalResourceName(anc.GetResourceName());
			if (!ancRn.IsEmpty() && ancRn != canonical)
				info.m_sVariantOf = ancRn;
		}

		// Sights zeroing distances
		TBD_RifleExtractor.ExtractZeroingDistances(comps, info.m_aZeroingDistances);

		// Intrinsic Muzzles
		TBD_RifleExtractor.ExtractMuzzles(comps, info.m_aMuzzles);

		// Intrinsic Attachment Slots
		TBD_RifleExtractor.ExtractAttachmentSlots(comps, info.m_aAttachmentSlots);

		m_aRifles.Insert(info);
	}

	//------------------------------------------------------------------------------------------------
	//! Serialize entire rifles dataset to normalized JSON
	string SerializeToJson()
	{
		string json = "{\n";
		json += "  \"version\": \"1\",\n";
		json += "  \"category\": \"rifles\",\n";
		json += "  \"generatedAt\": \"" + TBD_EquipmentExportJson.IsoNowUtc() + "\",\n";
		json += "  \"totalCount\": " + m_aRifles.Count().ToString() + ",\n";
		json += "  \"rifles\": [\n";

		for (int r = 0; r < m_aRifles.Count(); r++)
		{
			TBD_RifleWeaponInfo rif = m_aRifles[r];
			json += "    {\n";
			json += "      \"resource_name\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sResourceName) + "\",\n";
			json += "      \"id\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sId) + "\",\n";
			json += "      \"display_name\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sDisplayName) + "\",\n";
			json += "      \"family\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sFamily) + "\",\n";
			json += "      \"addon\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sAddonId) + "\",\n";
			json += "      \"file_path\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sFilePath) + "\",\n";

			string isAbsStr = "false";
			if (rif.m_bIsAbstract) isAbsStr = "true";
			json += "      \"is_abstract\": " + isAbsStr + ",\n";

			if (!rif.m_sVariantOf.IsEmpty())
				json += "      \"variant_of\": \"" + TBD_EquipmentExportJson.Escape(rif.m_sVariantOf) + "\",\n";

			if (rif.m_fWeightKg >= 0)
				json += "      \"weight_kg\": " + rif.m_fWeightKg.ToString() + ",\n";
			if (rif.m_fVolumeCm3 >= 0)
				json += "      \"volume_cm3\": " + rif.m_fVolumeCm3.ToString() + ",\n";

			// Zeroing
			json += "      \"zeroing_distances\": [";
			for (int zd = 0; zd < rif.m_aZeroingDistances.Count(); zd++)
			{
				json += "\"" + rif.m_aZeroingDistances[zd] + "\"";
				if (zd < rif.m_aZeroingDistances.Count() - 1)
					json += ", ";
			}
			json += "],\n";

			// Muzzles
			json += "      \"muzzles\": [\n";
			for (int m = 0; m < rif.m_aMuzzles.Count(); m++)
			{
				TBD_RifleMuzzleInfo muz = rif.m_aMuzzles[m];
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
				for (int f = 0; f < muz.m_aFireModes.Count(); f++)
				{
					TBD_RifleFireModeInfo fmi = muz.m_aFireModes[f];
					json += "{\"name\":\"" + fmi.m_sName + "\",\"burst\":" + fmi.m_iBurstCount.ToString() + ",\"rpm\":" + fmi.m_iRoundsPerMinute.ToString() + "}";
					if (f < muz.m_aFireModes.Count() - 1)
						json += ", ";
				}
				json += "]\n";

				json += "        }";
				if (m < rif.m_aMuzzles.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ],\n";

			// Attachment slots
			json += "      \"attachment_slots\": [\n";
			for (int s = 0; s < rif.m_aAttachmentSlots.Count(); s++)
			{
				TBD_RifleAttachmentSlotInfo slot = rif.m_aAttachmentSlots[s];
				json += "        {\n";
				json += "          \"slot_name\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sSlotName) + "\",\n";
				json += "          \"required_attachment_type\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sRequiredAttachmentType) + "\"";

				if (!slot.m_sDefaultAttachedPrefab.IsEmpty())
					json += ",\n          \"default_attachment\": \"" + TBD_EquipmentExportJson.Escape(slot.m_sDefaultAttachedPrefab) + "\"\n";
				else
					json += ",\n          \"default_attachment\": null\n";

				json += "        }";
				if (s < rif.m_aAttachmentSlots.Count() - 1)
					json += ",";
				json += "\n";
			}
			json += "      ]\n";

			json += "    }";
			if (r < m_aRifles.Count() - 1)
				json += ",";
			json += "\n";
		}

		json += "  ]\n}\n";
		return json;
	}

	//------------------------------------------------------------------------------------------------
	int GetCount()
	{
		return m_aRifles.Count();
	}
}
