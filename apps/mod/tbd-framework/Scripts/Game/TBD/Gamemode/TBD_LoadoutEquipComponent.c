/**
 * TBD_LoadoutEquipComponent.c - T-068.5 / T-068.5.1 Virtual Arsenal loadout equip test.
 *
 * Reads $profile:TBD_LoadoutTest.json (the web Arsenal "loadout-export.json" download,
 * packages/tbd-schema/schema/loadout-export.schema.json) and equips its four gear slots
 * (primary / uniform / vest / helmet) onto a freshly spawned, otherwise-empty US character.
 *
 * T-068.5.1 — VISUAL FIX: the previous pass used SCR_InventoryStorageManagerComponent.TryInsertItem,
 * which returns true while the item sits in storage (not worn) → character spawned naked despite
 * "equip OK" logs. The wear path now uses the real equip APIs and a deferred worn-verify gate:
 *   - clothing (uniform/vest/helmet): SCR_InventoryStorageManagerComponent.EquipCloth(item) (void),
 *     verified via SCR_CharacterInventoryStorageComponent.GetClothFromArea(<LoadoutAreaType>).
 *   - primary weapon: SCR_InventoryStorageManagerComponent.EquipWeapon(item),
 *     verified via SCR_CharacterInventoryStorageComponent.GetCurrentWeapon() (owner == item).
 * "equip OK" is logged ONLY after worn-verify; an inserted-but-not-worn item logs FAILED and is
 * deleted. Verify is deferred one CallLater tick because EquipCloth/EquipWeapon settle async.
 *
 * Server-only, dev-gated. Wired onto Prefabs/Systems/TBD_GameMode.et so a Workbench wb_play of
 * Missions/TBD_Dev_POC.conf runs it. Spawn @ 6400/6400 = the TBD_Dev_POC game-mode coords (the
 * player lands there), so the dressed pawn is visible without flying the camera.
 */

[ComponentEditorProps(category: "TBD/Framework", description: "Dev test: equip $profile:TBD_LoadoutTest.json gear onto a spawned empty US character.")]
class TBD_LoadoutEquipComponentClass : SCR_BaseGameModeComponentClass {}

//------------------------------------------------------------------------------------------------
//! DTO mirrors loadout-export.schema.json "gear" object (each value a ResourceName or null/"").
//! @contract loadout-export.schema.json#/$defs/gear
class TBD_LoadoutGearStruct
{
	string primary; //!< Primary weapon ResourceName (empty = none).
	string uniform; //!< Uniform ResourceName (empty = none).
	string vest;    //!< Vest ResourceName (empty = none).
	string helmet;  //!< Helmet ResourceName (empty = none).
}

//! DTO mirrors loadout-export.schema.json root.
//! @contract loadout-export.schema.json#/
class TBD_LoadoutExportStruct
{
	string loadoutVersion;          //!< Export format version (const "1").
	string modpackId;               //!< Source modpack id.
	ref TBD_LoadoutGearStruct gear; //!< The four gear slots.
}

//------------------------------------------------------------------------------------------------
//! One issued equip awaiting its deferred worn-verify pass.
class TBD_PendingEquip
{
	string label;
	string resName;
	IEntity item;
	bool isWeapon;
	typename areaType; // LoadoutAreaType subclass for clothing; ignored for weapon
}

//------------------------------------------------------------------------------------------------
class TBD_LoadoutEquipComponent : SCR_BaseGameModeComponent
{
	protected static const string LOADOUT_PATH = "$profile:TBD_LoadoutTest.json";
	//! Canonical modpack id the web exporter / registry emit (T-122 T14/M10).
	protected static const string EXPECTED_MODPACK_ID = "00000000-0000-4000-a000-000000000001";

	[Attribute("0", desc: "Run the loadout equip test on play (dev only — default OFF; do not ship enabled on TBD_GameMode).")]
	bool m_bRunLoadoutTest;

	[Attribute("{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et", desc: "Empty/minimal US body to equip onto (no baked kit).")]
	ResourceName m_sTestCharacter;

	[Attribute("6400 0 6400", desc: "World origin for the test spawn (TBD_Dev_POC game mode coords).")]
	vector m_vSpawnOrigin;

	protected IEntity m_Character;
	protected ref array<ref TBD_PendingEquip> m_aPending = {};

	//------------------------------------------------------------------------------------------------
	//! @authority server — the dev equip test spawns and dresses the test NPC server-side only.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Authority only — entity spawn + equip must run on the server.
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

		// --- A1.1: contract guards (T-122 M9/M10) ---------------------------------------------
		// loadoutVersion is pinned to "1" by loadout-export.schema.json; reject a future shape
		// rather than equipping it as if it were v1.
		if (doc.loadoutVersion != "1")
		{
			Print("[TBD][Loadout] FAILED: unsupported loadoutVersion '" + doc.loadoutVersion + "' (expected '1')", LogLevel.ERROR);
			return;
		}
		// A loadout built for a different modpack likely references prefab GUIDs this mod can't
		// resolve — warn (don't hard-fail, so a known-good cross-pack test can still proceed).
		if (doc.modpackId != EXPECTED_MODPACK_ID)
			Print("[TBD][Loadout] WARNING: modpackId '" + doc.modpackId + "' != expected '" + EXPECTED_MODPACK_ID + "' — prefabs may not resolve", LogLevel.WARNING);

		// --- spawn the empty test character ---------------------------------------------------
		m_Character = SpawnTestCharacter();
		if (!m_Character)
		{
			Print("[TBD][Loadout] FAILED: could not spawn test character " + m_sTestCharacter, LogLevel.ERROR);
			return;
		}

		// --- A2-A5: issue each equip (worn-verify is deferred below) ---------------------------
		m_aPending.Clear();
		IssueEquip("primary", doc.gear.primary, true,  LoadoutAreaType); // areaType unused for weapon
		IssueEquip("uniform", doc.gear.uniform, false, LoadoutJacketArea);
		IssueEquip("vest",    doc.gear.vest,    false, LoadoutVestArea);
		IssueEquip("helmet",  doc.gear.helmet,  false, LoadoutHeadCoverArea);

		// EquipCloth/EquipWeapon settle asynchronously — verify next tick.
		GetGame().GetCallqueue().CallLater(VerifyEquips, 1000, false);
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
			Print(string.Format("[TBD][Loadout] test spawn %1 (%2) @ %3", ent.GetID().ToString(), m_sTestCharacter, pos.ToString()));

		return ent;
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn the gear item and hand it to the equip API. Worn-verify happens later in VerifyEquips.
	protected void IssueEquip(string label, string resName, bool isWeapon, typename areaType)
	{
		if (resName.IsEmpty())
		{
			Print(string.Format("[TBD][Loadout] %1: skipped (empty slot)", label));
			return; // documented skip, not a FAIL
		}

		SCR_InventoryStorageManagerComponent mgr = SCR_InventoryStorageManagerComponent.Cast(
			m_Character.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!mgr)
		{
			Print(string.Format("[TBD][Loadout] %1 FAILED: character has no inventory manager (%2)", label, resName), LogLevel.ERROR);
			return;
		}

		Resource resource = Resource.Load(resName);
		if (!resource || !resource.IsValid())
		{
			Print(string.Format("[TBD][Loadout] %1 FAILED to load %2", label, resName), LogLevel.ERROR);
			return;
		}

		// Spawn the item entity at the character, then issue the real equip.
		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = m_Character.GetOrigin();

		IEntity item = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (!item)
		{
			Print(string.Format("[TBD][Loadout] %1 FAILED to spawn item %2", label, resName), LogLevel.ERROR);
			return;
		}

		if (isWeapon)
			mgr.EquipWeapon(item);
		else
			mgr.EquipCloth(item);

		TBD_PendingEquip pending = new TBD_PendingEquip();
		pending.label = label;
		pending.resName = resName;
		pending.item = item;
		pending.isWeapon = isWeapon;
		pending.areaType = areaType;
		m_aPending.Insert(pending);
	}

	//------------------------------------------------------------------------------------------------
	//! True if entity's parent chain roots at the given character (attached/worn, not loose).
	protected bool IsRootedOn(IEntity entity, IEntity root)
	{
		IEntity cur = entity;
		while (cur)
		{
			if (cur == root)
				return true;
			cur = cur.GetParent();
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! Deferred: confirm each issued item is actually WORN before logging equip OK.
	protected void VerifyEquips()
	{
		SCR_CharacterInventoryStorageComponent charStorage;
		if (m_Character)
			charStorage = SCR_CharacterInventoryStorageComponent.Cast(
				m_Character.FindComponent(SCR_CharacterInventoryStorageComponent));

		if (!charStorage)
		{
			Print("[TBD][Loadout] FAILED: character has no SCR_CharacterInventoryStorageComponent (cannot verify worn state)", LogLevel.ERROR);
			return;
		}

		foreach (TBD_PendingEquip p : m_aPending)
		{
			bool worn = false;
			string detail;

			if (p.isWeapon)
			{
				// SCR_CharacterInventoryStorageComponent.GetCurrentWeapon is protected — use the
				// public BaseWeaponManagerComponent on the character instead.
				IEntity wornEnt;
				BaseWeaponManagerComponent weaponMgr = BaseWeaponManagerComponent.Cast(
					m_Character.FindComponent(BaseWeaponManagerComponent));
				if (weaponMgr)
				{
					BaseWeaponComponent weapon = weaponMgr.GetCurrentWeapon();
					if (weapon)
						wornEnt = weapon.GetOwner();
				}
				// Accept either: the equipped item is the current weapon, OR it is rooted on the
				// character (slung in a weapon slot, not loose in the world / a vicinity drop).
				worn = (wornEnt && wornEnt == p.item) || IsRootedOn(p.item, m_Character);
				detail = "weapon";
				if (wornEnt)
					detail = "weapon=" + wornEnt.GetID().ToString();
			}
			else
			{
				// Clothing area typenames vary per item (a plate carrier reports
				// LoadoutArmoredVestSlotArea, not LoadoutVestArea), so search the expected area first
				// then the other body areas — a single fixed typename would false-FAIL (Amendment 3).
				// GetClothFromArea is the proven worn signal; IsRootedOn is a safety fallback.
				bool foundArea = false;
				string foundName;
				array<typename> candidates = {
					p.areaType,
					LoadoutJacketArea, LoadoutVestArea, LoadoutArmoredVestSlotArea,
					LoadoutHeadCoverArea, LoadoutCoverArea, LoadoutBackpackArea
				};
				foreach (typename area : candidates)
				{
					if (charStorage.GetClothFromArea(area) == p.item)
					{
						foundArea = true;
						foundName = area.ToString();
						break;
					}
				}

				if (foundArea)
				{
					worn = true;
					detail = foundName + " ent=" + p.item.GetID().ToString();
				}
				else if (IsRootedOn(p.item, m_Character))
				{
					worn = true;
					detail = "rooted on character (no matching loadout area)";
				}
				else
				{
					detail = "not in any loadout area";
				}
			}

			if (worn)
			{
				Print(string.Format("[TBD][Loadout] %1 equip OK %2 [%3]", p.label, p.resName, detail));
			}
			else
			{
				Print(string.Format("[TBD][Loadout] %1 FAILED (not worn) %2 [%3]", p.label, p.resName, detail), LogLevel.ERROR);
				if (p.item)
					SCR_EntityHelper.DeleteEntityAndChildren(p.item);
			}
		}

		Print("[TBD][Loadout] equip pass complete");
	}
}
