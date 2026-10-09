//------------------------------------------------------------------------------------------------
// TBD_WeaponMuzzleExtractor.c
//
// Reads the muzzles a weapon declares: the magazine wells each one feeds from, its default
// magazine, its barrel and chamber properties, and the fire modes it supports with their rate of
// fire and burst length.
//
// A weapon reaches its muzzles through MuzzleComponent and its subclasses, and a variant prefab
// exposes its effective values through Workbench containers. Each installed muzzle owns its settings.
//------------------------------------------------------------------------------------------------

class TBD_WeaponMuzzleExtractor
{
	//! Verify native mode getters on a temporary initialized prefab without advancing gameplay.
	static string VerifyInitializedFireModes(string resourceName)
	{
		WorldEditor editor = Workbench.GetModule(WorldEditor);
		if (!editor || !editor.GetApi() || !GetGame()) return "{\"status\":\"no_preview_world\"}";
		Resource resource = Resource.Load(resourceName);
		if (!resource || !resource.IsValid()) return "{\"status\":\"load_failed\"}";
		IEntity entity = GetGame().SpawnEntityPrefab(resource, editor.GetApi().GetWorld());
		if (!entity) return "{\"status\":\"spawn_failed\"}";
		array<string> entries = {};
		array<Managed> components = {};
		entity.FindComponents(BaseMuzzleComponent, components);
		foreach (Managed component : components)
		{
			BaseMuzzleComponent muzzle = BaseMuzzleComponent.Cast(component);
			if (!muzzle) continue;
			array<BaseFireMode> modes = {};
			muzzle.GetFireModesList(modes);
			array<string> modeEntries = {};
			foreach (BaseFireMode mode : modes)
			{
				if (!mode) { modeEntries.Insert("null"); continue; }
				int modeType = mode.GetFiremodeType();
				string entry = "{\"value\":" + modeType.ToString();
				entry += ",\"name\":" + TBD_EquipmentExportJson.Quote(typename.EnumToString(EWeaponFiremodeType, modeType));
				entry += ",\"ui_name\":" + TBD_EquipmentExportJson.Quote(mode.GetUIName()) + "}";
				modeEntries.Insert(entry);
			}
			string data = TBD_EquipmentExportJson.Context(muzzle.GetComponentSource(entity));
			entries.Insert(TBD_EquipmentExportJson.Member(data, "modes", "[" + TBD_EquipmentExportJson.Join(modeEntries) + "]"));
		}
		SCR_EntityHelper.DeleteEntityAndChildren(entity);
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//! Match the exact initialized component source; ambiguous repeated identifiers never pick an arbitrary instance.
	protected static BaseMuzzleComponent MatchPreviewMuzzle(IEntity preview, BaseContainer source)
	{
		array<BaseMuzzleComponent> matches = {};
		array<BaseMuzzleComponent> exact = {};
		CollectPreviewMuzzles(preview, source, matches, exact);
		if (exact.Count() == 1) return exact[0];
		if (exact.IsEmpty() && matches.Count() == 1) return matches[0];
		return null;
	}

	//! Search initialized entity ownership without combining distinct muzzle installations.
	protected static void CollectPreviewMuzzles(IEntity entity, BaseContainer source, array<BaseMuzzleComponent> matches, array<BaseMuzzleComponent> exact)
	{
		if (!entity) return;
		array<Managed> components = {};
		entity.FindComponents(BaseMuzzleComponent, components);
		foreach (Managed component : components)
		{
			BaseMuzzleComponent muzzle = BaseMuzzleComponent.Cast(component);
			if (!muzzle) continue;
			BaseContainer nativeSource = muzzle.GetComponentSource(entity);
			if (!nativeSource) continue;
			if (nativeSource == source) exact.Insert(muzzle);
			if (nativeSource.GetResourceName() == source.GetResourceName() && nativeSource.GetClassName() == source.GetClassName()) matches.Insert(muzzle);
		}
		IEntity child = entity.GetChildren();
		while (child)
		{
			CollectPreviewMuzzles(child, source, matches, exact);
			child = child.GetSibling();
		}
	}

	//! Use the initialized native fire-mode getter only when configuration does not expose the enum.
	protected static string ReadFireMode(BaseContainer source, BaseMuzzleComponent muzzle, int index, string path)
	{
		if (source.GetVarIndex("FireMode") >= 0) return TBD_EquipmentExportJson.Field(source, "FireMode", path);
		BaseFireMode nativeMode;
		if (muzzle && index < muzzle.GetFireModesCount()) nativeMode = muzzle.GetFireMode(index);
		if (!nativeMode)
		{
			TBD_EquipmentExportJson.RecordNativeField(source, path, "error", "BaseFireMode.GetFiremodeType", "No uniquely matched initialized muzzle/fire-mode instance", "");
			TBD_EquipmentExportJson.ExtractionError(TBD_EquipmentComponentGraph.m_CurrentResource, path, "No uniquely matched initialized fire mode");
			return "null";
		}
		int nativeValue = nativeMode.GetFiremodeType();
		string nativeName = typename.EnumToString(EWeaponFiremodeType, nativeValue);
		TBD_EquipmentExportJson.RecordNativeField(source, path, "present", "BaseFireMode.GetFiremodeType", "Temporary initialized prefab; no gameplay frames advanced; entity deleted after reading", nativeName);
		return nativeValue.ToString();
	}

	//------------------------------------------------------------------------------------------------
	//! Returns true if the class name represents a true weapon firing muzzle component (not visual effects).
	static bool IsMuzzleComponentClass(string cls)
	{
		if (cls.Contains("MuzzleEffect"))
			return false;
		return TBD_EquipmentComponentGraph.IsA(cls, "MuzzleComponent") || TBD_EquipmentComponentGraph.IsA(cls, "MuzzleInMagComponent");
	}

	//------------------------------------------------------------------------------------------------
	//! Extract muzzles, magazine wells, default magazines, and fire modes.
	static void ExtractMuzzles(map<string, ref array<BaseContainer>> comps, notnull array<ref TBD_WeaponMuzzleInfo> outMuzzles)
	{
		IEntity preview;
		foreach (string previewClass, array<BaseContainer> previewSources : comps)
		{
			if (!IsMuzzleComponentClass(previewClass)) continue;
			bool needsInitializedGetter;
			foreach (BaseContainer previewSource : previewSources)
			{
				BaseContainerList configuredModes = previewSource.GetObjectArray("FireModes");
				if (!configuredModes) continue;
				for (int mode = 0; mode < configuredModes.Count(); mode++)
				{
					BaseContainer configured = configuredModes.Get(mode);
					if (configured && configured.GetVarIndex("FireMode") < 0) needsInitializedGetter = true;
				}
			}
			if (!needsInitializedGetter) continue;
			WorldEditor editor = Workbench.GetModule(WorldEditor);
			Resource resource = Resource.Load(TBD_EquipmentComponentGraph.m_PrefabResource);
			if (editor && editor.GetApi() && GetGame() && resource && resource.IsValid())
				preview = GetGame().SpawnEntityPrefab(resource, editor.GetApi().GetWorld());
			break;
		}
		foreach (string cls, array<BaseContainer> bucket : comps)
		{
			if (!IsMuzzleComponentClass(cls)) continue;
			foreach (BaseContainer source : bucket)
			{
				TBD_WeaponMuzzleInfo muzzle = new TBD_WeaponMuzzleInfo();
				muzzle.m_iIndex = outMuzzles.Count();
				muzzle.m_sMuzzleClass = cls;
				muzzle.m_sInstanceId = TBD_EquipmentComponentGraph.InstanceId(source);
				string path = "/muzzles/" + muzzle.m_iIndex.ToString();
				string json = TBD_EquipmentExportJson.Fields(source, "default_magazine=MagazineTemplate|default_projectile=AmmoTemplate|disposable=Disposable|allow_weapon_deployment=AllowWeaponDeployment", path);
				json = TBD_EquipmentExportJson.Member(json, "source", TBD_EquipmentExportJson.Context(source));
				BaseContainer well = source.GetObject("MagazineWell");
				if (well) muzzle.m_aMagazineWells.Insert(well.GetClassName());
				BaseContainerList wells = source.GetObjectArray("MagazineWells");
				if (wells)
					for (int w = 0; w < wells.Count(); w++)
						if (wells.Get(w)) muzzle.m_aMagazineWells.Insert(wells.Get(w).GetClassName());
				json = TBD_EquipmentExportJson.Member(json, "magazine_wells", TBD_EquipmentExportJson.Strings(muzzle.m_aMagazineWells));
				source.Get("MagazineTemplate", muzzle.m_sDefaultMagazineTemplate);
				source.Get("AmmoTemplate", muzzle.m_sDefaultProjectile);
				string ballistics = TBD_EquipmentExportJson.Fields(source, "init_speed_coefficient=BulletInitSpeedCoef|dispersion_diameter=DispersionDiameter|dispersion_range=DispersionRange", path + "/ballistics");
				json = TBD_EquipmentExportJson.Member(json, "ballistics", ballistics);
				array<string> fireModes = {};
				BaseContainerList modes = source.GetObjectArray("FireModes");
				BaseMuzzleComponent nativeMuzzle = MatchPreviewMuzzle(preview, source);
				if (modes)
				{
					for (int modeIndex = 0; modeIndex < modes.Count(); modeIndex++)
					{
						BaseContainer mode = modes.Get(modeIndex);
						if (!mode) { fireModes.Insert("null"); continue; }
						string modePath = path + "/fire_modes/" + modeIndex.ToString();
						string entry = TBD_EquipmentExportJson.Fields(mode, "name=UIName|rpm=RoundsPerMinute|max_burst=MaxBurst|max_salvo=MaxSalvo|burst_type=BurstType|manual_action=ManualAction", modePath);
						entry = TBD_EquipmentExportJson.Member(entry, "fire_mode", ReadFireMode(mode, nativeMuzzle, modeIndex, modePath + "/fire_mode"));
						entry = TBD_EquipmentExportJson.Member(entry, "source", TBD_EquipmentExportJson.Context(mode));
						fireModes.Insert(entry);
						TBD_WeaponFireModeInfo modeInfo = new TBD_WeaponFireModeInfo();
						modeInfo.m_sSourceJson = entry;
						muzzle.m_aFireModes.Insert(modeInfo);
					}
				}
				json = TBD_EquipmentExportJson.Member(json, "fire_modes", "[" + TBD_EquipmentExportJson.Join(fireModes) + "]");
				json = TBD_EquipmentExportJson.Member(json, "aim_modifiers", ExtractAimModifiers(source, path));
				muzzle.m_sSourceJson = json;
				outMuzzles.Insert(muzzle);
			}
		}
		if (preview) SCR_EntityHelper.DeleteEntityAndChildren(preview);
	}

	static string ExtractAimModifiers(BaseContainer muzzle, string path)
	{
		BaseContainerList modifiers = muzzle.GetObjectArray("WeaponAimModifiers");
		if (!modifiers) return "null";
		array<string> entries = {};
		for (int i = 0; i < modifiers.Count(); i++)
		{
			BaseContainer modifier = modifiers.Get(i);
			if (!modifier) { entries.Insert("null"); continue; }
			string location = path + "/aim_modifiers/" + i.ToString();
			string json = TBD_EquipmentExportJson.Context(modifier);
			if (TBD_EquipmentComponentGraph.IsA(modifier.GetClassName(), "RecoilWeaponAimModifier"))
			{
				array<string> dataNames = {"LinearData", "AngularData", "TurnOffsetData"};
				array<string> fieldNames = {"linear", "angular", "turn_offset"};
				for (int dataIndex = 0; dataIndex < dataNames.Count(); dataIndex++)
				{
					BaseContainer data = modifier.GetObject(dataNames[dataIndex]);
					if (!data) continue;
					string bindings = "curve_x=Curve X|curve_y=Curve Y|curve_z=Curve Z|time_scale=Curve Time Scale";
					bindings += "|magnitudes=Curve Magnitudes|minimums=Curve Mins|maximums=Curve Maxs|base_scale=Base Recoil Scale";
					json = TBD_EquipmentExportJson.Member(json, fieldNames[dataIndex], TBD_EquipmentExportJson.Fields(data, bindings, location + "/" + fieldNames[dataIndex]));
				}
			}
			if (TBD_EquipmentComponentGraph.IsA(modifier.GetClassName(), "SwayWeaponAimModifier"))
				json = TBD_EquipmentExportJson.Member(json, "sway", TBD_EquipmentExportJson.Fields(modifier, "lower_translation=Lower Translation|lower_rotation=Lower Rotation", location + "/sway"));
			entries.Insert(json);
		}
		return "[" + TBD_EquipmentExportJson.Join(entries) + "]";
	}

	//------------------------------------------------------------------------------------------------
	protected static string FiremodeTypeToString(EWeaponFiremodeType fmt)
	{
		switch (fmt)
		{
			case EWeaponFiremodeType.Safety: return "Safety";
			case EWeaponFiremodeType.Semiauto: return "Semiauto";
			case EWeaponFiremodeType.Auto: return "Auto";
			case EWeaponFiremodeType.Burst: return "Burst";
			case EWeaponFiremodeType.Manual: return "Manual";
		}
		return string.Empty;
	}

	//------------------------------------------------------------------------------------------------
	protected static string BurstTypeToString(EBurstType bt)
	{
		switch (bt)
		{
			case EBurstType.BT_Uninterruptable: return "Uninterruptable";
			case EBurstType.BT_Interruptable: return "Interruptable";
			case EBurstType.BT_InterruptableAndResetting: return "InterruptableAndResetting";
		}
		return string.Empty;
	}
}
