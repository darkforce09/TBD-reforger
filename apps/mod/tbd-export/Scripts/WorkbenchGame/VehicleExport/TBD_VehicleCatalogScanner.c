/**
 * TBD_VehicleCatalogScanner.c
 *
 * Groups the standard vehicle extractor's resolved records into the catalog view.
 * Catalog, summary and platform files serialize the same vehicle objects.
 */
class TBD_VehicleCatalogScanner
{
	ref TBD_VehicleDeepExtractor m_Resolved;
	ref array<ref TBD_VehiclePlatformInfo> m_aPlatforms = {};
	ref map<string, ref TBD_VehiclePlatformInfo> m_mPlatformMap = new map<string, ref TBD_VehiclePlatformInfo>();
	ref array<ref TBD_VehicleVariantInfo> m_aAllVariants = {};

	//! Runs the standard vehicle scan once and creates catalog views of its resolved objects.
	bool ScanAllAddons()
	{
		m_aPlatforms.Clear();
		m_mPlatformMap.Clear();
		m_aAllVariants.Clear();
		m_Resolved = new TBD_VehicleDeepExtractor();
		if (!m_Resolved.ScanAllAddons()) return false;
		foreach (TBD_VehicleDeepPlatform resolvedPlatform : m_Resolved.m_aPlatforms)
		{
			TBD_VehiclePlatformInfo platform = new TBD_VehiclePlatformInfo();
			platform.m_Resolved = resolvedPlatform;
			platform.m_sPlatformId = resolvedPlatform.m_sPlatformId;
			platform.m_sDisplayName = resolvedPlatform.m_sDisplayName;
			platform.m_sVehicleDomain = resolvedPlatform.m_sVehicleDomain;
			platform.m_sPrimaryFaction = resolvedPlatform.m_sPrimaryFaction;
			foreach (TBD_VehicleDeepVariant resolvedVariant : resolvedPlatform.m_aVariants)
			{
				TBD_VehicleVariantInfo variant = CatalogVariant(resolvedVariant);
				platform.m_aVariants.Insert(variant);
				m_aAllVariants.Insert(variant);
			}
			m_aPlatforms.Insert(platform);
			m_mPlatformMap.Insert(platform.m_sPlatformId, platform);
		}
		return !m_aAllVariants.IsEmpty();
	}

	//! Keeps catalog identity aliases tied to their original resolved vehicle record.
	protected TBD_VehicleVariantInfo CatalogVariant(TBD_VehicleDeepVariant resolved)
	{
		TBD_VehicleVariantInfo variant = new TBD_VehicleVariantInfo();
		variant.m_Resolved = resolved;
		variant.m_sId = resolved.m_sId;
		variant.m_sResourceName = resolved.m_sResourceName;
		variant.m_sDisplayName = resolved.m_sDisplayName;
		variant.m_sDescription = resolved.m_sDescription;
		variant.m_sPlatform = resolved.m_sPlatform;
		variant.m_sFaction = resolved.m_sFaction;
		variant.m_sVehicleDomain = resolved.m_sVehicleDomain;
		variant.m_sFilePath = resolved.m_sFilePath;
		variant.m_sAddonId = resolved.m_sAddonId;
		variant.m_sParentPrefab = resolved.m_sParentPrefab;
		variant.m_fMassKg = resolved.m_fWeightKg;
		return variant;
	}

	//! Serializes the resolved objects through the standard vehicle serializer.
	string SerializeAllToJson()
	{
		if (!m_Resolved) return "null";
		return TBD_VehicleDeepSerializer.SerializeAllToJson(m_Resolved.m_aPlatforms, m_Resolved.m_aAllVariants.Count());
	}

	//! Builds summary counts from the same resolved platform membership as the catalog.
	string SerializeSummaryToJson()
	{
		if (!m_Resolved) return "null";
		return TBD_VehicleDeepSerializer.SerializeSummaryToJson(m_Resolved.m_aPlatforms, m_Resolved.m_aAllVariants.Count());
	}
}
