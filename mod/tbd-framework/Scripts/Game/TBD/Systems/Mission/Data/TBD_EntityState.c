/**
 * @file TBD_EntityState.c
 * @brief Applies authored `entities[]` health, allowDamage, showModel and size to placed bodies.
 *
 * Role: a second `JsonLoadContext` pass over the held mission JSON whose root declares only those
 * `entities[]` keys, an index of the placed bodies, and the engine calls that apply them:
 *   health      -> `DamageManagerComponent.SetHealthScaled` (fraction 0..1; 0 is destroyed);
 *   allowDamage -> `DamageManagerComponent.EnableDamageHandling(true)` when bound true;
 *   showModel   -> `IEntity.SetFlags(EntityFlags.VISIBLE)` when bound true;
 *   size        -> `IEntity.SetScale` (1 = native; 0 or below is skipped);
 *   stamina     -> not applied: the engine's stamina components expose no enable toggle, so an
 *                  authored true logs one WARNING per pass.
 * Position: `TBD_MissionWorldApplier` fills the index (`ResetIndex`, `RecordSpawn`);
 * `ApplySpawned` runs from `TBD_SlotBodyMaterializer` after `TBD_VehicleState.ApplySpawned`.
 * Reads `TBD_MissionJsonPass.LoadRoot`.
 * State: the static rows and body index, server only.  Invariants: a row finds its body by `uid`,
 * else by an alias, x and z fingerprint, never by a nearby entity; absent `health` and `size` read
 * `ABSENT`; bools apply only when true, because an absent bool and an authored false bind the same.
 */

//! One `entities[]` row's state fields. Field names are the JSON keys.
//! @contract mission.schema.json#/$defs/entity
class TBD_EntityStateWireStruct
{
	static const float ABSENT = -1000000; //!< "key absent from JSON" sentinel for `health` and `size`

	string uid; //!< `uid`, or empty
	string alias; //!< `alias`
	float x; //!< `x`, world metres
	float z; //!< `z`, world metres
	float health = ABSENT;     //!< Fraction 0..1. ABSENT when omitted. 0 is authored destroyed.
	bool allowDamage;          //!< Schema boolean. Absent and authored-false bind the same.
	bool showModel;            //!< Schema boolean. Absent and authored-false bind the same.
	float size = ABSENT;       //!< Uniform scale. ABSENT when omitted. 1 = native.
	bool stamina;              //!< Schema boolean: stamina enabled. See header: no toggle API.

	//! Whether the row authors `health`.
	bool HasHealth()
	{
		return health != ABSENT;
	}

	//! Whether the row authors `size`.
	bool HasSize()
	{
		return size != ABSENT;
	}

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

	//! Whether the row authors anything this reader applies or logs, `stamina` included.
	bool HasAny()
	{
		if (HasApplyable())
			return true;
		if (stamina)
			return true;
		return false;
	}
}

//! Root of the second parse. Declares `entities` and nothing else.
//! @contract mission.schema.json#/ partial
class TBD_EntityStateDocStruct
{
	ref array<ref TBD_EntityStateWireStruct> entities; //!< `entities[]`
}

//! One placed `entities[]` body, recorded at placement so Apply can find it without a box
//! query (two props can share a metre).
class TBD_EntityStateTwin
{
	string uid; //!< the row's `uid`, or empty
	string fingerprint; //!< `alias|x|z`
	IEntity body; //!< the placed entity (not owned)
}

//! Server-side reader: bind entities[] health/allowDamage/showModel/size/stamina and apply.
class TBD_EntityState
{
	protected static ref array<ref TBD_EntityStateWireStruct> s_aRows; //!< rows of the last pass; empty when none
	protected static ref array<ref TBD_EntityStateTwin> s_aTwins; //!< placed bodies in wire order; rebuilt on each mission load
	protected static bool s_bStaminaSkipLogged; //!< the stamina WARNING was written this pass

	//! Drop the entities[] -> world index. Called at the top of the `entities[]` placement pass,
	//! before its early return, so a reload whose new mission authors no entities[] cannot inherit
	//! the previous mission's pointers.
	static void ResetIndex()
	{
		s_aTwins = new array<ref TBD_EntityStateTwin>();
	}

	//! Record one `entities[]` row that reached the world. Skipped spawn rows are not recorded.
	//! @param body the placed entity; null records nothing
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

	//! Apply every authored row to its placed body. Runs after `TBD_VehicleState.ApplySpawned`,
	//! so every `entities[]` body already exists and roster vehicles have joined or spawned. Does
	//! nothing when the document has no entities[] state keys; a row with no indexed body logs a
	//! WARNING and is skipped.
	//! @authority server
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

	//! Apply authored state to one spawned body. Unset numerics are ABSENT and leave engine
	//! defaults. Bools apply only when the bound value is true (see header). Health is last so a
	//! 0-health destroy still receives scale / visibility first. Null arguments do nothing.
	//! @authority server
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

	//! Run the entity-state pass into `s_aRows`.
	//! @return false when no mission text is held; true otherwise, with an ERROR line and no rows
	//! when the text or its root does not read
	protected static bool Parse()
	{
		s_aRows = new array<ref TBD_EntityStateWireStruct>();

		TBD_EMissionJsonPassOutcome outcome;
		JsonLoadContext ctx = TBD_MissionJsonPass.LoadRoot(outcome);
		if (outcome == TBD_EMissionJsonPassOutcome.NO_DOCUMENT)
			return false;

		if (!ctx)
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

	//! The indexed body of a row: by `uid` first, else by fingerprint, skipping a twin that
	//! carries a different non-empty uid.
	//! @return the body, or null when none is indexed
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

	//! Set scaled health; outside 0..1 or no `DamageManagerComponent` logs a WARNING instead.
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

	//! Enable damage handling; no `DamageManagerComponent` logs a WARNING instead.
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

	//! Set the body's `VISIBLE` flag.
	protected static void ApplyShowModel(IEntity body)
	{
		body.SetFlags(EntityFlags.VISIBLE, false);
	}

	//! Set the body's uniform scale; 0 or below, or a scale the prefab does not keep, logs a WARNING.
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

	//! Log once per pass that an authored stamina toggle is not applied.
	protected static void LogStaminaSkip()
	{
		if (s_bStaminaSkipLogged)
			return;
		s_bStaminaSkipLogged = true;
		Print("[TBD][EntityState] authored stamina=true but Reforger exposes no per-character stamina enable toggle (BaseStaminaComponent.GetStamina only; CharacterStaminaComponent is empty) -- stamina NOT applied", LogLevel.WARNING);
	}
}
