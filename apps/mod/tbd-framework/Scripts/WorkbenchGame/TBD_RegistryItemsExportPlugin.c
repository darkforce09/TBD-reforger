/**
 * TBD_RegistryItemsExportPlugin.c - T-068.1 Virtual Arsenal flat ResourceName export.
 *
 * Workbench plugin: resolves a curated list of vanilla Arma Reforger character +
 * Phase 1 gear prefabs to their canonical Enfusion ResourceName ({GUID}Prefabs/.../File.et)
 * via Resource.Load + BaseContainer.GetResourceName(), and writes a registry-items
 * envelope (packages/tbd-schema/schema/registry-items.schema.json) to
 *   $profile:TBD_RegistryItems.json
 *
 * The curated path list is seeded from enfusion-mcp discovery (asset_search /
 * game_read) against the base game pak set. The plugin re-resolves every entry through
 * the engine so the committed JSON carries engine-canonical GUIDs (never hand-typed).
 *
 * Run: Workbench > Plugins > "Export TBD Registry Items"
 *   (or NetAPI: wb_execute_action menuPath "Plugins,Export TBD Registry Items").
 * Then copy $profile:TBD_RegistryItems.json to
 *   packages/tbd-schema/registry/registry-items.workbench.json
 *
 * @contract registry-items.schema.json#/
 */

//! Internal export row (camelCase); the writer emits snake_case keys per the schema item.
//! @contract registry-items.schema.json#/$defs/item
class TBD_RegistryItemRow
{
	string path;          // pak-relative prefab path (no GUID) OR full ResourceName
	string displayName;
	string category;      // slash-delimited browse path
	string kind;          // character | gear_primary | gear_uniform | gear_vest | gear_helmet
}

[WorkbenchPluginAttribute(name: "Export TBD Registry Items", description: "Resolve curated vanilla prefabs to canonical ResourceNames and write registry-items JSON.", category: "TBD")]
class TBD_RegistryItemsExportPlugin : WorkbenchPlugin
{
	protected static const string OUT_PATH = "$profile:TBD_RegistryItems.json";
	protected static const string MODPACK_ID = "00000000-0000-4000-a000-000000000001";
	protected static const string ITEMS_VERSION = "1";

	//------------------------------------------------------------------------------------------------
	//! Curated source list. Paths discovered via enfusion-mcp asset_search / game_read.
	protected ref array<ref TBD_RegistryItemRow> BuildCuratedRows()
	{
		array<ref TBD_RegistryItemRow> rows = {};

		// ---- Characters (US Army, BLUFOR) -------------------------------------------------
		// Full canonical ResourceNames (GUID via Workbench "Copy Resource Name" / vanilla POC).
		AddRow(rows, "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et", "US Rifleman",           "NATO/US_Army/Rifleman",          "character");
		AddRow(rows, "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et",       "US Grenadier",          "NATO/US_Army/Grenadier",         "character");
		AddRow(rows, "{C9E4FEAF5AAC8D8C}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Medic.et",    "US Medic",              "NATO/US_Army/Medic",             "character");
		AddRow(rows, "{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et",       "US Automatic Rifleman", "NATO/US_Army/AutomaticRifleman", "character");
		AddRow(rows, "{1623EA3AEFACA0E4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_MG.et",       "US Machine Gunner",     "NATO/US_Army/MachineGunner",     "character");
		AddRow(rows, "{0B3167BB0FB68110}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_PL.et",       "US Platoon Leader",     "NATO/US_Army/Leadership",        "character");
		AddRow(rows, "{27BF1FF235DD6036}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_LAT.et",      "US Light Anti-Tank",    "NATO/US_Army/AntiTank",          "character");
		AddRow(rows, "{36CCDB4556ECDA06}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Engineer.et", "US Engineer",           "NATO/US_Army/Engineer",          "character");

		// ---- Gear: primary weapons --------------------------------------------------------
		AddRow(rows, "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",        "M16A2",        "NATO/Weapons/Primary", "gear_primary");
		AddRow(rows, "{5A987A8A13763769}Prefabs/Weapons/Rifles/M16/Rifle_M16A2_M203.et",   "M16A2 + M203", "NATO/Weapons/Primary", "gear_primary");
		AddRow(rows, "{D2B48DEBEF38D7D7}Prefabs/Weapons/MachineGuns/M249/MG_M249.et",      "M249 SAW",     "NATO/Weapons/Primary", "gear_primary");
		AddRow(rows, "{D182DCDD72BF7E34}Prefabs/Weapons/MachineGuns/M60/MG_M60.et",        "M60",          "NATO/Weapons/Primary", "gear_primary");

		// ---- Gear: uniforms ---------------------------------------------------------------
		AddRow(rows, "{C7861F11D5334C0E}Prefabs/Characters/Uniforms/Jacket_US_BDU.et",          "BDU Jacket (Woodland)", "NATO/Uniform", "gear_uniform");
		AddRow(rows, "{3CCA7A9BB4FD3197}Prefabs/Characters/Uniforms/Jacket_US_BDU_rolledup.et", "BDU Jacket (Rolled)",   "NATO/Uniform", "gear_uniform");
		AddRow(rows, "{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et",           "BDU Pants (Woodland)",  "NATO/Uniform", "gear_uniform");

		// ---- Gear: vests ------------------------------------------------------------------
		AddRow(rows, "{4B57C11AA5161760}Prefabs/Characters/Vests/Vest_PASGT/Vest_PASGT.et",                   "PASGT Vest",            "NATO/Vest", "gear_vest");
		AddRow(rows, "{2835A0EA3B79E63E}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_rifleman.et", "ALICE Vest (Rifleman)", "NATO/Vest", "gear_vest");
		AddRow(rows, "{156DC7109CEE6F69}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_AR.et",       "ALICE Vest (Automatic Rifleman)", "NATO/Vest", "gear_vest");
		AddRow(rows, "{725C5E1C75CADAF4}Prefabs/Characters/Vests/Vest_M69/Vest_M69_M81woodland.et",           "M69 Vest (M81 Woodland)", "NATO/Vest", "gear_vest");

		// ---- Gear: helmets ----------------------------------------------------------------
		AddRow(rows, "{FE5C49069C2499D9}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover.et",           "PASGT Helmet (Cover)",          "NATO/Helmet", "gear_helmet");
		AddRow(rows, "{E685A8D337D36204}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover_w_goggles.et", "PASGT Helmet (Cover + Goggles)", "NATO/Helmet", "gear_helmet");

		return rows;
	}

	//------------------------------------------------------------------------------------------------
	protected void AddRow(array<ref TBD_RegistryItemRow> rows, string path, string displayName, string category, string kind)
	{
		TBD_RegistryItemRow row = new TBD_RegistryItemRow();
		row.path = path;
		row.displayName = displayName;
		row.category = category;
		row.kind = kind;
		rows.Insert(row);
	}

	//------------------------------------------------------------------------------------------------
	//! Resolve a pak-relative path (or ResourceName) to canonical {GUID}path via the engine.
	protected ResourceName ResolveCanonical(string pathOrName)
	{
		Resource res = Resource.Load(pathOrName);
		if (!res || !res.IsValid())
			return string.Empty;

		BaseResourceObject obj = res.GetResource();
		if (!obj)
			return string.Empty;

		BaseContainer ctr = obj.ToBaseContainer();
		if (!ctr)
			return string.Empty;

		return ctr.GetResourceName();
	}

	//------------------------------------------------------------------------------------------------
	protected string JsonEscape(string s)
	{
		s.Replace("\\", "\\\\");
		s.Replace("\"", "\\\"");
		return s;
	}

	//------------------------------------------------------------------------------------------------
	override void Run()
	{
		array<ref TBD_RegistryItemRow> rows = BuildCuratedRows();

		string body;
		int written = 0;

		foreach (TBD_RegistryItemRow row : rows)
		{
			ResourceName canonical = ResolveCanonical(row.path);
			if (canonical.IsEmpty())
			{
				Print("[TBD][RegistryExport] FAILED to resolve " + row.path, LogLevel.ERROR);
				continue;
			}

			Print(string.Format("[TBD][RegistryExport] %1  ->  %2", row.kind, canonical));

			if (written > 0)
				body += ",\n";

			body += "    {\n";
			body += "      \"resource_name\": \"" + JsonEscape(canonical) + "\",\n";
			body += "      \"display_name\": \"" + JsonEscape(row.displayName) + "\",\n";
			body += "      \"category\": \"" + JsonEscape(row.category) + "\",\n";
			body += "      \"kind\": \"" + JsonEscape(row.kind) + "\"\n";
			body += "    }";

			written++;
		}

		string json;
		json += "{\n";
		json += "  \"registryItemsVersion\": \"" + ITEMS_VERSION + "\",\n";
		json += "  \"modpackId\": \"" + MODPACK_ID + "\",\n";
		json += "  \"items\": [\n";
		json += body + "\n";
		json += "  ]\n";
		json += "}\n";

		FileHandle handle = FileIO.OpenFile(OUT_PATH, FileMode.WRITE);
		if (!handle)
		{
			Print("[TBD][RegistryExport] Could not open " + OUT_PATH + " for write", LogLevel.ERROR);
			return;
		}

		handle.Write(json);
		handle.Close();

		Print(string.Format("[TBD][RegistryExport] Wrote %1 items to %2", written, OUT_PATH));
	}
}
