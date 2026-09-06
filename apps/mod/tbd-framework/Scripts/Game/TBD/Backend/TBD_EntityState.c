//! T-681 - entity states: health, allowDamage, showModel, size, stamina.
//!
//! == What was missing ========================================================================
//! T-706 put those five keys on `$defs/entity`. `TBD_MissionEntityStruct` is still
//! alias/uid/x/z/headingDeg/faction only, so the primary parse cannot see them. Spawned
//! entities kept engine defaults. This file is the reader. Editor UI is NOT this slice.
//!
//! == Why a second JsonLoadContext pass =======================================================
//! Same pattern as `TBD_VehicleState.c` (T-680) / `TBD_WaypointRuntime.c` (T-677): a second
//! pass over `TBD_MissionLoader.GetRawJson()` with a root that declares `entities[]` those
//! five fields and nothing else. Do not grow `TBD_MissionEntityStruct`.
//!
//! == Presence, and the nested-ref landmine ===================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key
//! is ABSENT. `entities` is an ARRAY, so presence is a null-or-Count() test. Numeric fields
//! that can be authored as 0 (`health`) carry an ABSENT sentinel. `size` is exclusiveMinimum 0
//! so authored 0 is illegal; the sentinel still distinguishes omit from a positive scale.
//! Bools cannot: omit and authored-false bind the same (T-676 `repeat`, T-946.37 lock). Apply
//! allowDamage / showModel only when the bound value is true. Authored false leaves the engine
//! default. `stamina` is a SCHEMA BOOLEAN (whether stamina is enabled), not a 0..1 fraction.
//!
//! == ATTR-FIELD-OBJ-HEALTH / -ALLOW-DAMAGE / -SHOW-MODEL / -SIZE / -STAMINA ==================
//!   health      -> DamageManagerComponent.SetHealthScaled (fraction 0..1). 0 is destroyed.
//!   allowDamage -> DamageManagerComponent.EnableDamageHandling(true) when bound true.
//!   showModel   -> IEntity.SetFlags(EntityFlags.VISIBLE) when bound true.
//!   size        -> IEntity.SetScale (1 = native). exclusiveMinimum 0: <=0 is skipped.
//!   stamina     -> NOT APPLIED. BaseStaminaComponent exposes GetStamina only;
//!                  CharacterStaminaComponent / SCR_CharacterStaminaComponent add no enable
//!                  toggle (CRF calls AddStamina to restore drain; that is not a toggle).
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. Whether a half-health hidden-scale entity actually
//! spawns that way is a human checklist item.
//! @contract mission.schema.json#/$defs/entity

//------------------------------------------------------------------------------------------------
//! One `entities[]` row's state fields. Field names are the JSON keys.
class TBD_EntityStateWireStruct
{
	static const float ABSENT = -1000000;

	string uid;
	string alias;
	float x;
	float z;
	float health = ABSENT;     //!< Fraction 0..1. ABSENT when omitted. 0 is authored destroyed.
	bool allowDamage;          //!< Schema boolean. Absent and authored-false bind the same.
	bool showModel;            //!< Schema boolean. Absent and authored-false bind the same.
	float size = ABSENT;       //!< Uniform scale. ABSENT when omitted. 1 = native.
	bool stamina;              //!< Schema boolean: stamina enabled. See header: no toggle API.

	//------------------------------------------------------------------------------------------------
	bool HasHealth()
	{
		return health != ABSENT;
	}

	//------------------------------------------------------------------------------------------------
	bool HasSize()
	{
		return size != ABSENT;
	}

	//------------------------------------------------------------------------------------------------
	//! True when a field this file can actually apply is present (stamina is logged, not applied).
	bool HasApplyable()
	{
		if (allowDamage)
			return true;
		if (showModel)
			return true;
		if (health != ABSENT)
			return true;
		if (size != ABSENT)
			return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	bool HasAny()
	{
		if (HasApplyable())
			return true;
		if (stamina)
			return true;
		return false;
	}
}

//------------------------------------------------------------------------------------------------
//! Root of the second parse. Declares `entities` and nothing else.
class TBD_EntityStateDocStruct
{
	ref array<ref TBD_EntityStateWireStruct> entities;
}

//------------------------------------------------------------------------------------------------
//! One spawned `entities[]` body, recorded from `SpawnMissionEntities` so Apply can find it
//! without an AABB guess (two props can share a metre).
class TBD_EntityStateTwin
{
	string uid;
	string fingerprint;
	IEntity body;
}

//------------------------------------------------------------------------------------------------
//! Server-side reader: bind entities[] health/allowDamage/showModel/size/stamina and apply.
class TBD_EntityState
{
	protected static ref array<ref TBD_EntityStateWireStruct> s_aRows;
	protected static ref array<ref TBD_EntityStateTwin> s_aTwins;
	protected static bool s_bStaminaSkipLogged;

	//------------------------------------------------------------------------------------------------
	//! Drop the entities[] -> world index. Called at the TOP of `SpawnMissionEntities`, before
	//! its early return, so a reload whose new mission authors no entities[] cannot inherit the
	//! previous mission's pointers.
	static void ResetIndex()
	{
		s_aTwins = new array<ref TBD_EntityStateTwin>();
	}

	//------------------------------------------------------------------------------------------------
	//! Record one `entities[]` row that reached the world. Skipped spawn rows are not recorded.
	static void RecordSpawn(string uid, string alias, float x, float z, IEntity body)
	{
		if (!s_aTwins)
			ResetIndex();
		if (!body)
			return;

		TBD_EntityStateTwin twin = new TBD_EntityStateTwin();
		twin.uid = uid;
		twin.fingerprint = string.Format("%1|%2|%3", alias, x, z);
		twin.body = body;
		s_aTwins.Insert(twin);
	}

	//------------------------------------------------------------------------------------------------
	//! Called from `TBD_SpawnManager.MaterializeSlotBodies` AFTER `TBD_VehicleState.ApplySpawned`,
	//! so every `entities[]` body already exists (SpawnMissionEntities ran at parse) and roster
	//! vehicles have joined or spawned. No-ops when the document has no entities[] state keys.
	static void ApplySpawned()
	{
		s_bStaminaSkipLogged = false;

		if (!Parse())
			return;
		if (!s_aRows)
			return;
		if (s_aRows.Count() < 1)
			return;

		int applied = 0;
		int healthy = 0;
		int damaged = 0;
		int shown = 0;
		int sized = 0;
		int missed = 0;

		foreach (TBD_EntityStateWireStruct wire : s_aRows)
		{
			if (!wire)
				continue;
			if (!wire.HasAny())
				continue;

			if (wire.stamina)
				LogStaminaSkip();

			if (!wire.HasApplyable())
				continue;

			IEntity body = FindBody(wire);
			if (!body)
			{
				missed++;
				Print(string.Format("[TBD][EntityState] no world entity for uid='%1' alias='%2' at %3,%4 -- state NOT applied",
					wire.uid, wire.alias, wire.x, wire.z), LogLevel.WARNING);
				continue;
			}

			Apply(body, wire);
			applied++;
			if (wire.HasHealth())
				healthy++;
			if (wire.allowDamage)
				damaged++;
			if (wire.showModel)
				shown++;
			if (wire.HasSize())
				sized++;
		}

		if (applied < 1 && missed < 1)
			return;

		Print(string.Format("[TBD][EntityState] applied=%1 health=%2 allowDamage=%3 showModel=%4 size=%5 missed=%6",
			applied, healthy, damaged, shown, sized, missed));
	}

	//------------------------------------------------------------------------------------------------
	//! Apply authored state to one spawned body. Unset numerics are ABSENT and leave engine
	//! defaults. Bools apply only when the bound value is true (see header). Health is last so a
	//! 0-health destroy still receives scale / visibility first.
	static void Apply(IEntity body, TBD_EntityStateWireStruct wire)
	{
		if (!body || !wire)
			return;

		if (wire.HasSize())
			ApplySize(body, wire.size);

		if (wire.showModel)
			ApplyShowModel(body);

		if (wire.allowDamage)
			ApplyAllowDamage(body);

		if (wire.HasHealth())
			ApplyHealth(body, wire.health);
	}

	//------------------------------------------------------------------------------------------------
	protected static bool Parse()
	{
		s_aRows = new array<ref TBD_EntityStateWireStruct>();

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
		{
			Print("[TBD][EntityState] the mission document did not parse as JSON on the entity-state pass - no health/allowDamage/showModel/size applied this round", LogLevel.ERROR);
			return true;
		}

		TBD_EntityStateDocStruct doc = new TBD_EntityStateDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print("[TBD][EntityState] the mission document parsed but its root would not read on the entity-state pass - no health/allowDamage/showModel/size applied this round", LogLevel.ERROR);
			return true;
		}

		if (!doc.entities)
			return true;
		s_aRows = doc.entities;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static IEntity FindBody(TBD_EntityStateWireStruct wire)
	{
		if (!s_aTwins || !wire)
			return null;

		if (!wire.uid.IsEmpty())
		{
			foreach (TBD_EntityStateTwin byUid : s_aTwins)
			{
				if (!byUid || !byUid.body)
					continue;
				if (byUid.uid != wire.uid)
					continue;
				return byUid.body;
			}
		}

		string fingerprint = string.Format("%1|%2|%3", wire.alias, wire.x, wire.z);
		foreach (TBD_EntityStateTwin byPos : s_aTwins)
		{
			if (!byPos || !byPos.body)
				continue;
			if (byPos.fingerprint != fingerprint)
				continue;
			if (!byPos.uid.IsEmpty() && byPos.uid != wire.uid)
				continue;
			return byPos.body;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyHealth(IEntity body, float health)
	{
		if (health < 0 || health > 1)
		{
			Print(string.Format("[TBD][EntityState] health=%1 outside 0..1 -- not applied", health), LogLevel.WARNING);
			return;
		}

		DamageManagerComponent dmg = DamageManagerComponent.Cast(body.FindComponent(DamageManagerComponent));
		if (!dmg)
		{
			Print("[TBD][EntityState] authored health but the body has no DamageManagerComponent -- health NOT applied", LogLevel.WARNING);
			return;
		}

		dmg.SetHealthScaled(health);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyAllowDamage(IEntity body)
	{
		DamageManagerComponent dmg = DamageManagerComponent.Cast(body.FindComponent(DamageManagerComponent));
		if (!dmg)
		{
			Print("[TBD][EntityState] authored allowDamage=true but the body has no DamageManagerComponent -- allowDamage NOT applied", LogLevel.WARNING);
			return;
		}

		dmg.EnableDamageHandling(true);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplyShowModel(IEntity body)
	{
		body.SetFlags(EntityFlags.VISIBLE, false);
	}

	//------------------------------------------------------------------------------------------------
	protected static void ApplySize(IEntity body, float size)
	{
		if (size <= 0)
		{
			Print(string.Format("[TBD][EntityState] size=%1 is not > 0 -- not applied", size), LogLevel.WARNING);
			return;
		}

		body.SetScale(size);
		float got = body.GetScale();
		if (got - size > 0.001 || size - got > 0.001)
		{
			Print(string.Format("[TBD][EntityState] SetScale(%1) did not stick (GetScale=%2) -- prefab may not support size", size, got), LogLevel.WARNING);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static void LogStaminaSkip()
	{
		if (s_bStaminaSkipLogged)
			return;
		s_bStaminaSkipLogged = true;
		Print("[TBD][EntityState] authored stamina=true but Reforger exposes no per-character stamina enable toggle (BaseStaminaComponent.GetStamina only; CharacterStaminaComponent is empty) -- stamina NOT applied", LogLevel.WARNING);
	}
}
