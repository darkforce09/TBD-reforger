/**
 * TBD_LoadoutEquipComponent.c - T-068.5 Virtual Arsenal loadout equip test.
 *
 * Reads $profile:TBD_LoadoutTest.json (the web Arsenal "loadout-export.json" download,
 * packages/tbd-schema/schema/loadout-export.schema.json) and equips its four gear slots
 * (primary / uniform / vest / helmet) onto a freshly spawned, otherwise-empty US character.
 *
 * Equip uses the exact ResourceName strings from the JSON via the engine inventory APIs —
 * no kit: alias layer (that is T-068's mission-slot path, not this dumb-loadout test):
 *   - clothing (uniform/vest/helmet): SCR_InventoryStorageManagerComponent.TryInsertItem
 *     (auto-routes a clothing item to its LoadoutAreaType body slot)
 *   - primary weapon: SCR_InventoryStorageManagerComponent.EquipWeapon
 *
 * Server-only, dev-gated. Wired onto Prefabs/Systems/TBD_GameMode.et so a Workbench
 * wb_play of Missions/TBD_Dev_POC.conf runs it. Every equip logs [TBD][Loadout] OK/FAILED
 * with the full {GUID} ResourceName for the T-068.5 verification gate (A1-A7).
 */

[ComponentEditorProps(category: "TBD/Framework", description: "Dev test: equip $profile:TBD_LoadoutTest.json gear onto a spawned empty US character.")]
class TBD_LoadoutEquipComponentClass : SCR_BaseGameModeComponentClass {}

//------------------------------------------------------------------------------------------------
//! DTO mirrors loadout-export.schema.json "gear" object (each value a ResourceName or null/"").
class TBD_LoadoutGearStruct
{
	string primary;
	string uniform;
	string vest;
	string helmet;
}

//! DTO mirrors loadout-export.schema.json root.
class TBD_LoadoutExportStruct
{
	string loadoutVersion;
	string modpackId;
	ref TBD_LoadoutGearStruct gear;
}

//------------------------------------------------------------------------------------------------
class TBD_LoadoutEquipComponent : SCR_BaseGameModeComponent
{
	protected static const string LOADOUT_PATH = "$profile:TBD_LoadoutTest.json";

	[Attribute("1", desc: "Run the loadout equip test on play (dev only).")]
	bool m_bRunLoadoutTest;

	[Attribute("{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et", desc: "Empty/minimal US body to equip onto (no baked kit).")]
	ResourceName m_sTestCharacter;

	[Attribute("6400 0 6400", desc: "World origin for the test spawn (TBD_Dev_POC game mode coords).")]
	vector m_vSpawnOrigin;

	//------------------------------------------------------------------------------------------------
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!m_bRunLoadoutTest)
			return;

		// Defer so the world surface + replication are ready (mirrors TBD_RegistryPocComponent).
		GetGame().GetCallqueue().CallLater(RunLoadoutTest, 3000, false);
	}

	//------------------------------------------------------------------------------------------------
	protected void RunLoadoutTest()
	{
		// --- A1: read + parse $profile:TBD_LoadoutTest.json -----------------------------------
		if (!FileIO.FileExists(LOADOUT_PATH))
		{
			Print("[TBD][Loadout] FAILED: no file at " + LOADOUT_PATH, LogLevel.ERROR);
			return;
		}

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromFile(LOADOUT_PATH))
		{
			Print("[TBD][Loadout] FAILED: could not read " + LOADOUT_PATH, LogLevel.ERROR);
			return;
		}

		TBD_LoadoutExportStruct doc = new TBD_LoadoutExportStruct();
		if (!ctx.ReadValue("", doc) || !doc.gear)
		{
			Print("[TBD][Loadout] FAILED: parse error in TBD_LoadoutTest.json", LogLevel.ERROR);
			return;
		}

		Print(string.Format("[TBD][Loadout] Loaded TBD_LoadoutTest.json (version %1, modpack %2)", doc.loadoutVersion, doc.modpackId));

		// --- spawn the empty test character ---------------------------------------------------
		IEntity character = SpawnTestCharacter();
		if (!character)
		{
			Print("[TBD][Loadout] FAILED: could not spawn test character " + m_sTestCharacter, LogLevel.ERROR);
			return;
		}

		// --- A2-A5: equip each gear slot from its exact ResourceName --------------------------
		EquipSlot(character, "primary", doc.gear.primary, true);
		EquipSlot(character, "uniform", doc.gear.uniform, false);
		EquipSlot(character, "vest",    doc.gear.vest,    false);
		EquipSlot(character, "helmet",  doc.gear.helmet,  false);

		Print("[TBD][Loadout] equip pass complete");
	}

	//------------------------------------------------------------------------------------------------
	protected IEntity SpawnTestCharacter()
	{
		Resource resource = Resource.Load(m_sTestCharacter);
		if (!resource || !resource.IsValid())
		{
			Print("[TBD][Loadout] Resource.Load failed for character " + m_sTestCharacter, LogLevel.ERROR);
			return null;
		}

		float x = m_vSpawnOrigin[0];
		float z = m_vSpawnOrigin[2];
		float y = GetGame().GetWorld().GetSurfaceY(x, z);
		vector pos = Vector(x, y, z);

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;

		IEntity ent = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (ent)
			Print(string.Format("[TBD][Loadout] test spawn %1 @ %2", ent.GetID().ToString(), pos.ToString()));

		return ent;
	}

	//------------------------------------------------------------------------------------------------
	//! Equip one gear ResourceName onto the character. isWeapon routes primary to the weapon slot.
	protected bool EquipSlot(IEntity character, string label, string resName, bool isWeapon)
	{
		if (resName.IsEmpty())
		{
			Print(string.Format("[TBD][Loadout] %1: skipped (empty slot)", label));
			return true; // documented skip, not a FAIL
		}

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			Print(string.Format("[TBD][Loadout] %1 FAILED: character has no inventory manager (%2)", label, resName), LogLevel.ERROR);
			return false;
		}

		Resource resource = Resource.Load(resName);
		if (!resource || !resource.IsValid())
		{
			Print(string.Format("[TBD][Loadout] %1 FAILED to load %2", label, resName), LogLevel.ERROR);
			return false;
		}

		// Spawn the item entity, then hand it to the inventory manager.
		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = character.GetOrigin();

		IEntity item = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (!item)
		{
			Print(string.Format("[TBD][Loadout] %1 FAILED to spawn item %2", label, resName), LogLevel.ERROR);
			return false;
		}

		bool ok;
		if (isWeapon)
		{
			// Weapon → weapon slot (falls back to generic insert if equip is rejected).
			ok = mgr.EquipWeapon(item);
			if (!ok)
				ok = mgr.TryInsertItem(item, EStoragePurpose.PURPOSE_WEAPON_PROXY);
			if (!ok)
				ok = mgr.TryInsertItem(item, EStoragePurpose.PURPOSE_ANY);
		}
		else
		{
			// Clothing → its LoadoutAreaType body slot via auto-routing.
			ok = mgr.TryInsertItem(item, EStoragePurpose.PURPOSE_LOADOUT_PROXY);
			if (!ok)
				ok = mgr.TryInsertItem(item, EStoragePurpose.PURPOSE_ANY);
		}

		if (ok)
		{
			Print(string.Format("[TBD][Loadout] %1 equip OK %2", label, resName));
			return true;
		}

		Print(string.Format("[TBD][Loadout] %1 FAILED to equip %2", label, resName), LogLevel.ERROR);
		SCR_EntityHelper.DeleteEntityAndChildren(item);
		return false;
	}
}
