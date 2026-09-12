//! T-705 - player gadget flags: map, compass, watch, GPS, radio.
//!
//! == What was missing ========================================================================
//! T-706 put `$defs/gadgetFlags` on `$defs/slot.gadgets`. `TBD_MissionSlotStruct` does not
//! declare that member, so the primary parse cannot see it. Every spawned player kept the
//! kit's gadgets regardless of the scenario. This file is the reader. Flatten does not emit
//! the keys (T-946.36); hand-staged 1.3 JSON and golden `schema-1_3-wire-fields.json` reach
//! this pass. Editor UI is NOT this slice (owns list has no panel).
//!
//! == Why a second JsonLoadContext pass =======================================================
//! Same pattern as `TBD_PlacementScatter.c` (T-679) / `TBD_EntityState.c` (T-681): a second
//! pass over `TBD_MissionLoader.GetRawJson()` with a root that declares `slots[]` gadgets
//! and nothing else. MissionSlotStruct stays out of this slice's owns list. MissionLoader
//! calls `Bind()` after a valid parse (the T-682 EnvironmentReader seam).
//!
//! == Presence: nested-ref landmine AND the bool hole ========================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref` even when the JSON key is ABSENT.
//! `if (slot.gadgets)` is ALWAYS TRUE. Bools cannot carry a sentinel (T-676 / T-946.37),
//! so a struct of five bools cannot tell "gadgets omitted" from "all five authored false"
//! -- applying false in that case would STRIP gadgets on every mission that never authored
//! the block. `gadgets` is therefore a `map<string, bool>`: Count()==0 is omit/empty
//! (keep kit defaults); Find(key) is per-flag presence; the bool is the authored on/off.
//! @contract mission.schema.json#/$defs/gadgetFlags
//! @contract mission.schema.json#/$defs/slot

//------------------------------------------------------------------------------------------------
//! One `slots[]` row's gadget flags. Field names are the JSON keys.
class TBD_GadgetFlagsSlotWireStruct
{
	string id;
	string uid;
	//! Per-flag presence is Find(), not a null check. Allocated-empty when the key is omitted.
	ref map<string, bool> gadgets;
}

//------------------------------------------------------------------------------------------------
//! Root of the second parse. Declares `slots` and nothing else.
class TBD_GadgetFlagsDocStruct
{
	ref array<ref TBD_GadgetFlagsSlotWireStruct> slots;
}

//------------------------------------------------------------------------------------------------
//! Server-side reader: bind slot.gadgets and apply after loadout on player spawn.
class TBD_GadgetFlags
{
	static const string CH = "Gadgets";
	static const string KEY_MAP = "map";
	static const string KEY_COMPASS = "compass";
	static const string KEY_WATCH = "watch";
	static const string KEY_GPS = "gps";
	static const string KEY_RADIO = "radio";

	//! Kit-neutral defaults used only when the flag is authored true and the body has none.
	static const string PREFAB_MAP = "{7B6990100263E2B3}Prefabs/Items/Core/Map_Base.et";
	static const string PREFAB_COMPASS = "{61D4F80E49BF9B12}Prefabs/Items/Equipment/Compass/Compass_SY183.et";
	static const string PREFAB_WATCH = "{6FD6C96121905202}Prefabs/Items/Equipment/Watches/Watch_Vostok.et";
	static const string PREFAB_RADIO = "{E1A5D4B878AA8980}Prefabs/Items/Equipment/Radios/Radio_R148.et";

	//! Loadout cargo lands in VerifyTick (500 ms after Run). Apply after that so a cargo
	//! row cannot put a withheld gadget back.
	protected static const int POST_LOADOUT_MS = 800;

	protected static ref array<ref TBD_GadgetFlagsSlotWireStruct> s_aSlots;
	protected static bool s_bParsed;
	protected static bool s_bArmed;
	protected static string s_sParsedForMission;

	//------------------------------------------------------------------------------------------------
	//! Called from `TBD_MissionLoader.ParseMissionJson` after a valid parse, on the server-only
	//! load path. Parses the gadgets block and arms the spawn hook. No-ops when no slot authors
	//! the block, so missions without flags boot unchanged.
	static void Bind()
	{
		s_bParsed = false;
		s_sParsedForMission = string.Empty;
		Parse();
		ArmSpawnHook();
	}

	//------------------------------------------------------------------------------------------------
	protected static void ArmSpawnHook()
	{
		if (s_bArmed)
			return;

		SCR_BaseGameMode gm = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		if (!gm)
			return;

		gm.GetOnPlayerSpawned().Insert(OnPlayerSpawned);
		s_bArmed = true;
	}

	//------------------------------------------------------------------------------------------------
	//! Spawn-notify sink. Dressing already ran in SpawnSlotBody; cargo verify is still in
	//! flight, so apply is deferred POST_LOADOUT_MS.
	protected static void OnPlayerSpawned(int playerId, IEntity controlledEntity)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;
		if (playerId <= 0)
			return;

		GetGame().GetCallqueue().CallLater(ApplyForPlayer, POST_LOADOUT_MS, false, playerId);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyForPlayer(int playerId)
	{
		if (RplSession.Mode() == RplMode.Client)
			return;

		PlayerManager pm = GetGame().GetPlayerManager();
		if (!pm)
			return;

		IEntity body = pm.GetPlayerControlledEntity(playerId);
		if (!body)
			return;

		TBD_SpawnManager spawn = TBD_SpawnManager.GetInstance();
		if (!spawn)
			return;

		TBD_MissionSlotStruct slot = spawn.GetAssignedSlot(playerId);
		if (!slot)
			return;

		ApplyToBody(body, slot.Key(), slot.id);
	}

	//------------------------------------------------------------------------------------------------
	static void ApplyToBody(IEntity body, string slotKey, string slotId)
	{
		if (!body)
			return;
		if (!Parse())
			return;

		TBD_GadgetFlagsSlotWireStruct wire = FindSlot(slotKey, slotId);
		if (!wire)
			return;
		if (!wire.gadgets)
			return;
		if (wire.gadgets.Count() < 1)
			return;

		SCR_GadgetManagerComponent gadgets = SCR_GadgetManagerComponent.GetGadgetManager(body);
		if (!gadgets)
		{
			Print(string.Format("[TBD][%1] slot=%2 no gadget manager -- flags not applied", CH, slotKey), LogLevel.WARNING);
			return;
		}

		bool wantMap;
		bool compass;
		bool watch;
		bool gps;
		bool radio;

		if (wire.gadgets.Find(KEY_MAP, wantMap))
			ApplyOne(body, gadgets, KEY_MAP, wantMap, EGadgetType.MAP, PREFAB_MAP);
		if (wire.gadgets.Find(KEY_COMPASS, compass))
			ApplyOne(body, gadgets, KEY_COMPASS, compass, EGadgetType.COMPASS, PREFAB_COMPASS);
		if (wire.gadgets.Find(KEY_WATCH, watch))
			ApplyByNeedle(body, KEY_WATCH, watch, "Watch", PREFAB_WATCH);
		if (wire.gadgets.Find(KEY_GPS, gps))
			ApplyByNeedle(body, KEY_GPS, gps, "GPS", string.Empty);
		if (wire.gadgets.Find(KEY_RADIO, radio))
			ApplyRadio(body, gadgets, radio);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyOne(IEntity body, notnull SCR_GadgetManagerComponent gadgets, string key, bool want, EGadgetType type, string prefab)
	{
		if (want)
		{
			EnsureType(body, gadgets, key, type, prefab);
			return;
		}

		RemoveType(gadgets, key, type);
	}

	//------------------------------------------------------------------------------------------------
	//! Watch and GPS have no EGadgetType member on this engine (compile-proved: WATCH
	//! is undefined). Match inventory prefab paths. GPS has no item in the TBD registry,
	//! so authored true cannot add one; authored false still withholds a GPS-named item.
	protected static void ApplyByNeedle(IEntity body, string key, bool want, string needle, string prefab)
	{
		if (want)
		{
			if (HasPrefabNeedle(body, needle))
				return;
			if (prefab.IsEmpty())
			{
				Print(string.Format("[TBD][%1] %2=true but this engine has no %2 item to add -- kit default kept", CH, key), LogLevel.WARNING);
				return;
			}
			EnsurePrefab(body, key, prefab);
			return;
		}

		RemoveByNeedle(body, key, needle);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyRadio(IEntity body, notnull SCR_GadgetManagerComponent gadgets, bool want)
	{
		if (want)
		{
			IEntity handheld = gadgets.GetGadgetByType(EGadgetType.RADIO);
			if (handheld)
				return;
			IEntity backpack = gadgets.GetGadgetByType(EGadgetType.RADIO_BACKPACK);
			if (backpack)
				return;
			EnsurePrefab(body, KEY_RADIO, PREFAB_RADIO);
			return;
		}

		RemoveType(gadgets, KEY_RADIO, EGadgetType.RADIO);
		RemoveType(gadgets, KEY_RADIO, EGadgetType.RADIO_BACKPACK);
	}

	//------------------------------------------------------------------------------------------------
	protected static void EnsureType(IEntity body, notnull SCR_GadgetManagerComponent gadgets, string key, EGadgetType type, string prefab)
	{
		IEntity existing = gadgets.GetGadgetByType(type);
		if (existing)
			return;
		EnsurePrefab(body, key, prefab);
	}

	//------------------------------------------------------------------------------------------------
	protected static void EnsurePrefab(IEntity body, string key, string prefab)
	{
		if (prefab.IsEmpty())
		{
			Print(string.Format("[TBD][%1] slot body wants %2=true but no prefab is known -- not added", CH, key), LogLevel.WARNING);
			return;
		}

		IEntity item = SpawnItem(body, prefab);
		if (!item)
		{
			Print(string.Format("[TBD][%1] %2=true prefab failed to spawn %3", CH, key, prefab), LogLevel.WARNING);
			return;
		}

		SCR_InventoryStorageManagerComponent inv = SCR_InventoryStorageManagerComponent.Cast(
			body.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!inv)
		{
			Print(string.Format("[TBD][%1] %2=true no inventory manager -- spawned item discarded", CH, key), LogLevel.WARNING);
			SCR_EntityHelper.DeleteEntityAndChildren(item);
			return;
		}

		if (inv.CanInsertItem(item))
		{
			if (inv.TryInsertItem(item))
			{
				Print(string.Format("[TBD][%1] added %2", CH, key));
				return;
			}
		}

		Print(string.Format("[TBD][%1] %2=true insert refused -- item discarded", CH, key), LogLevel.WARNING);
		SCR_EntityHelper.DeleteEntityAndChildren(item);
	}

	//------------------------------------------------------------------------------------------------
	protected static void RemoveType(notnull SCR_GadgetManagerComponent gadgets, string key, EGadgetType type)
	{
		array<SCR_GadgetComponent> found = gadgets.GetGadgetsByType(type);
		if (!found)
			return;
		if (found.Count() < 1)
			return;

		int removed = 0;
		foreach (SCR_GadgetComponent gadget : found)
		{
			if (!gadget)
				continue;
			IEntity item = gadget.GetOwner();
			if (!item)
				continue;
			SCR_EntityHelper.DeleteEntityAndChildren(item);
			removed++;
		}

		if (removed > 0)
			Print(string.Format("[TBD][%1] withheld %2 count=%3", CH, key, removed));
	}

	//------------------------------------------------------------------------------------------------
	protected static bool HasPrefabNeedle(IEntity body, string needle)
	{
		array<IEntity> items = CollectItems(body);
		if (!items)
			return false;

		foreach (IEntity item : items)
		{
			if (PrefabMatchesNeedle(item, needle))
				return true;
		}
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static void RemoveByNeedle(IEntity body, string key, string needle)
	{
		array<IEntity> items = CollectItems(body);
		if (!items)
			return;

		int removed = 0;
		foreach (IEntity item : items)
		{
			if (!PrefabMatchesNeedle(item, needle))
				continue;
			SCR_EntityHelper.DeleteEntityAndChildren(item);
			removed++;
		}

		if (removed > 0)
			Print(string.Format("[TBD][%1] withheld %2 count=%3", CH, key, removed));
	}

	//------------------------------------------------------------------------------------------------
	protected static bool PrefabMatchesNeedle(IEntity item, string needle)
	{
		if (!item)
			return false;
		if (needle.IsEmpty())
			return false;

		string prefab = PrefabOf(item);
		if (prefab.Contains(needle))
			return true;
		if (needle == "GPS" && prefab.Contains("Gps"))
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	protected static array<IEntity> CollectItems(IEntity body)
	{
		if (!body)
			return null;

		SCR_InventoryStorageManagerComponent inv = SCR_InventoryStorageManagerComponent.Cast(
			body.FindComponent(SCR_InventoryStorageManagerComponent));
		if (!inv)
			return null;

		array<IEntity> items = new array<IEntity>();
		inv.GetItems(items);
		return items;
	}

	//------------------------------------------------------------------------------------------------
	protected static string PrefabOf(IEntity ent)
	{
		if (!ent)
			return string.Empty;
		EntityPrefabData pd = ent.GetPrefabData();
		if (!pd)
			return string.Empty;
		return pd.GetPrefabName();
	}

	//------------------------------------------------------------------------------------------------
	protected static IEntity SpawnItem(IEntity body, string prefab)
	{
		Resource resource = Resource.Load(prefab);
		if (!resource)
			return null;
		if (!resource.IsValid())
			return null;

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		body.GetTransform(params.Transform);
		return GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
	}

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc)
			return string.Empty;
		if (!doc.meta)
			return string.Empty;
		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool Parse()
	{
		string missionId = CurrentMissionId();
		if (s_bParsed && missionId == s_sParsedForMission)
			return true;

		s_aSlots = new array<ref TBD_GadgetFlagsSlotWireStruct>();

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
		{
			Print(string.Format("[TBD][%1] the mission document did not parse as JSON on the gadgets pass -- kit defaults kept", CH), LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		TBD_GadgetFlagsDocStruct doc = new TBD_GadgetFlagsDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print(string.Format("[TBD][%1] the mission document parsed but its root would not read on the gadgets pass -- kit defaults kept", CH), LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		if (doc.slots)
			s_aSlots = doc.slots;

		int authored = CountAuthored();
		s_bParsed = true;
		s_sParsedForMission = missionId;
		Print(string.Format("[TBD][%1] bound slots=%2 withFlags=%3", CH, s_aSlots.Count(), authored));
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static int CountAuthored()
	{
		if (!s_aSlots)
			return 0;

		int n = 0;
		foreach (TBD_GadgetFlagsSlotWireStruct row : s_aSlots)
		{
			if (!row)
				continue;
			if (!row.gadgets)
				continue;
			if (row.gadgets.Count() < 1)
				continue;
			n++;
		}
		return n;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_GadgetFlagsSlotWireStruct FindSlot(string slotKey, string slotId)
	{
		if (!s_aSlots)
			return null;

		foreach (TBD_GadgetFlagsSlotWireStruct row : s_aSlots)
		{
			if (!row)
				continue;
			if (slotKey.IsEmpty())
				continue;
			if (row.uid.IsEmpty())
				continue;
			if (row.uid == slotKey)
				return row;
		}

		foreach (TBD_GadgetFlagsSlotWireStruct row2 : s_aSlots)
		{
			if (!row2)
				continue;
			if (slotId.IsEmpty())
				continue;
			if (row2.id == slotId)
				return row2;
		}

		return null;
	}
}
