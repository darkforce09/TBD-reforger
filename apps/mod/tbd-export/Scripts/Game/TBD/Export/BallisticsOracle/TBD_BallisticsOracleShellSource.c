/**
 * @file TBD_BallisticsOracleShellSource.c
 * @brief One vanilla mortar shell prefab, read for the ballistics oracle: speed, charges, tables.
 *
 * Role: lists the seven vanilla mortar shells and reads from each prefab the values the oracle
 * samples with: the ShellMoveComponent InitSpeed and BallisticTableConfig, the
 * SCR_MortarShellGadgetComponent charge rings, and the InitSpeedCoefficient of every native table
 * in that config.  Position: used by the Workbench forward-angle writer and the play-mode
 * simulation component; reads prefab and config sources only, never a spawned entity.
 * State: the loaded resource and the values read, per instance.  Invariants: a value that cannot
 * be read is recorded in m_aErrors and never guessed; native coefficients keep table order and
 * appear once each.
 */

//! The values the oracle needs from one mortar shell prefab, read from its entity source.
class TBD_BallisticsOracleShellSource
{
	protected static const float SAME_COEFFICIENT = 0.00001; //!< coefficients closer than this are one coefficient
	protected static const string INDIRECT_LIST = "Indirect fire Table data"; //!< BallisticTableArray list of indirect-fire tables
	protected static const string DIRECT_LIST = "Table data"; //!< BallisticTableArray list of direct-fire tables

	ResourceName m_sPrefab; //!< shell prefab resource name, GUID included; JSON key `prefab`
	float m_fInitSpeed; //!< ShellMoveComponent InitSpeed, m/s; JSON key `init_speed_m_s`
	string m_sBallisticTableConfig; //!< ShellMoveComponent BallisticTableConfig; JSON key `ballistic_table_config`
	ref array<vector> m_aChargeRings = {}; //!< per charge: rings, init speed coefficient, default flag (0 or 1)
	ref array<float> m_aNativeCoefficients = {}; //!< distinct InitSpeedCoefficient of the native tables, table order
	ref array<string> m_aNativeLists = {}; //!< parallel to m_aNativeCoefficients: `indirect`, `direct` or `indirect+direct`
	ref array<string> m_aErrors = {}; //!< what could not be read, one sentence each; JSON key `errors`
	protected ref Resource m_Resource; //!< keeps the prefab resource, and so its entity source, loaded
	protected IEntitySource m_Source; //!< the prefab's entity source; null when it did not load

	//! Fills `prefabs` with the seven vanilla mortar shell prefabs, 81 mm then 82 mm.
	static void VanillaMortarShells(notnull array<ResourceName> prefabs)
	{
		prefabs.Insert("{38BAE094333E31BF}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_HE_M821.et");
		prefabs.Insert("{DD6844AB03FDA84F}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_Practice_M879.et");
		prefabs.Insert("{F7807293E94D3C88}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_Smoke_M819.et");
		prefabs.Insert("{DD2065AE34D8DFA9}Prefabs/Weapons/Ammo/Ammo_Shell_81mm_Illum_M853A1.et");
		prefabs.Insert("{98EC9C526AFBA282}Prefabs/Weapons/Ammo/Ammo_Shell_82mm_HE_O832DU.et");
		prefabs.Insert("{A544A2C131DE2C64}Prefabs/Weapons/Ammo/Ammo_Shell_82mm_Smoke_D832DU.et");
		prefabs.Insert("{C8A906FB198D1A33}Prefabs/Weapons/Ammo/Ammo_Shell_82mm_Illum_S832S.et");
	}

	//! Reads `prefab`. Returns true when the entity source, a positive InitSpeed and at least one
	//! charge ring were read; every value that could not be read is listed in m_aErrors.
	bool Load(ResourceName prefab)
	{
		m_sPrefab = prefab;
		m_Resource = Resource.Load(prefab);
		if (!m_Resource || !m_Resource.IsValid() || !m_Resource.GetResource())
		{
			m_aErrors.Insert("The prefab does not load");
			return false;
		}

		m_Source = m_Resource.GetResource().ToEntitySource();
		if (!m_Source)
		{
			m_aErrors.Insert("The prefab has no entity source");
			return false;
		}

		ReadComponents();
		if (!m_sBallisticTableConfig.IsEmpty())
		{
			ReadNativeList(INDIRECT_LIST, "indirect");
			ReadNativeList(DIRECT_LIST, "direct");
		}

		return m_fInitSpeed > 0 && !m_aChargeRings.IsEmpty();
	}

	//! Returns the prefab's entity source, the argument the BallisticTable queries take; null when
	//! the prefab did not load.
	IEntitySource Source()
	{
		return m_Source;
	}

	//! Returns the 16-hex-digit GUID of the prefab resource name, or an empty string.
	string PrefabGuid()
	{
		if (m_sPrefab.Length() >= 18 && m_sPrefab.StartsWith("{"))
			return m_sPrefab.Substring(1, 16);

		return string.Empty;
	}

	//! Returns the rings of charge `index`.
	int ChargeRings(int index)
	{
		vector charge = m_aChargeRings[index];
		return Math.Round(charge[0]);
	}

	//! Returns the init speed coefficient of charge `index`.
	float ChargeCoefficient(int index)
	{
		vector charge = m_aChargeRings[index];
		return charge[1];
	}

	//! Returns true when charge `index` carries the default flag.
	bool ChargeIsDefault(int index)
	{
		vector charge = m_aChargeRings[index];
		return !float.AlmostEqual(charge[2], 0);
	}

	//! Returns the index of the first charge whose coefficient equals `coefficient`, or -1.
	int FindCharge(float coefficient)
	{
		for (int index = 0; index < m_aChargeRings.Count(); index++)
		{
			if (float.AlmostEqual(ChargeCoefficient(index), coefficient, SAME_COEFFICIENT))
				return index;
		}

		return -1;
	}

	//! Returns the index of `coefficient` among the native coefficients, or -1.
	int FindNative(float coefficient)
	{
		for (int index = 0; index < m_aNativeCoefficients.Count(); index++)
		{
			if (float.AlmostEqual(m_aNativeCoefficients[index], coefficient, SAME_COEFFICIENT))
				return index;
		}

		return -1;
	}

	//! Returns the shell's members without braces: `prefab`, `prefab_guid`, `init_speed_m_s`,
	//! `ballistic_table_config`, `charge_rings` [{index, rings, init_speed_coef, is_default}],
	//! `native_tables` [{list, init_speed_coef}] and `errors`.
	string MetadataMembers()
	{
		string json = "\"prefab\":" + TBD_BallisticsOracleJson.Quote(m_sPrefab);
		json += ",\"prefab_guid\":" + TBD_BallisticsOracleJson.QuoteOrNull(PrefabGuid());
		json += ",\"init_speed_m_s\":" + TBD_BallisticsOracleJson.Number(m_fInitSpeed);
		json += ",\"ballistic_table_config\":" + TBD_BallisticsOracleJson.QuoteOrNull(m_sBallisticTableConfig);
		json += ",\"charge_rings\":[";
		for (int charge = 0; charge < m_aChargeRings.Count(); charge++)
		{
			if (charge > 0)
				json += ",";

			json += "{\"index\":" + charge.ToString() + ",\"rings\":" + ChargeRings(charge).ToString();
			json += ",\"init_speed_coef\":" + TBD_BallisticsOracleJson.Number(ChargeCoefficient(charge));
			json += ",\"is_default\":" + TBD_BallisticsOracleJson.Boolean(ChargeIsDefault(charge)) + "}";
		}

		json += "],\"native_tables\":[";
		for (int table = 0; table < m_aNativeCoefficients.Count(); table++)
		{
			if (table > 0)
				json += ",";

			json += "{\"list\":" + TBD_BallisticsOracleJson.Quote(m_aNativeLists[table]);
			json += ",\"init_speed_coef\":" + TBD_BallisticsOracleJson.Number(m_aNativeCoefficients[table]) + "}";
		}

		return json + "],\"errors\":" + ErrorsJson();
	}

	//! Returns m_aErrors as a JSON array of strings.
	string ErrorsJson()
	{
		string json = "[";
		for (int index = 0; index < m_aErrors.Count(); index++)
		{
			if (index > 0)
				json += ",";

			json += TBD_BallisticsOracleJson.Quote(m_aErrors[index]);
		}

		return json + "]";
	}

	//! Reads InitSpeed and BallisticTableConfig from the ProjectileMoveComponent and the charge
	//! rings from the SCR_MortarShellGadgetComponent.
	protected void ReadComponents()
	{
		bool foundMove = false;
		bool foundGadget = false;
		int count = m_Source.GetComponentCount();
		for (int index = 0; index < count; index++)
		{
			IEntityComponentSource component = m_Source.GetComponent(index);
			if (!component)
				continue;

			typename type = component.GetClassName().ToType();
			if (!type)
				continue;

			if (!foundMove && type.IsInherited(ProjectileMoveComponent))
			{
				foundMove = true;
				if (!component.Get("InitSpeed", m_fInitSpeed))
					m_aErrors.Insert("ProjectileMoveComponent InitSpeed does not read");

				if (!component.Get("BallisticTableConfig", m_sBallisticTableConfig) || m_sBallisticTableConfig.IsEmpty())
					m_aErrors.Insert("ProjectileMoveComponent BallisticTableConfig is empty");
			}

			if (!foundGadget && type.IsInherited(SCR_MortarShellGadgetComponent))
			{
				foundGadget = true;
				if (!component.Get("m_aChargeRingConfig", m_aChargeRings) || m_aChargeRings.IsEmpty())
					m_aErrors.Insert("SCR_MortarShellGadgetComponent m_aChargeRingConfig is empty");
			}
		}

		if (!foundMove)
			m_aErrors.Insert("The prefab has no ProjectileMoveComponent");

		if (!foundGadget)
			m_aErrors.Insert("The prefab has no SCR_MortarShellGadgetComponent");
	}

	//! Adds the InitSpeedCoefficient of every table in list `property` of the BallisticTableConfig,
	//! labelled `label`; a coefficient already listed gains the label instead.
	protected void ReadNativeList(string property, string label)
	{
		Resource config = Resource.Load(m_sBallisticTableConfig);
		if (!config || !config.IsValid() || !config.GetResource())
		{
			m_aErrors.Insert("BallisticTableConfig does not load");
			return;
		}

		BaseContainer tables = config.GetResource().ToBaseContainer();
		if (!tables || tables.GetVarIndex(property) < 0)
			return;

		BaseContainerList entries = tables.GetObjectArray(property);
		if (!entries)
		{
			m_aErrors.Insert("BallisticTableConfig list '" + property + "' does not read");
			return;
		}

		for (int index = 0; index < entries.Count(); index++)
		{
			BaseContainer entry = entries.Get(index);
			float coefficient;
			if (!entry || !entry.Get("InitSpeedCoefficient", coefficient))
			{
				m_aErrors.Insert("BallisticTableConfig list '" + property + "' entry " + index.ToString() + " has no InitSpeedCoefficient");
				continue;
			}

			int known = FindNative(coefficient);
			if (known < 0)
			{
				m_aNativeCoefficients.Insert(coefficient);
				m_aNativeLists.Insert(label);
			}
			else if (m_aNativeLists[known] != label && !m_aNativeLists[known].EndsWith("+" + label))
			{
				m_aNativeLists[known] = m_aNativeLists[known] + "+" + label;
			}
		}
	}
}
