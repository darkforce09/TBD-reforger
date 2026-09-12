//! T-679 - placement scatter: radius and area shape.
//!
//! == What was missing ========================================================================
//! T-706 put `placementRadius` / `placementShape` on `$defs/slot` and `$defs/group`.
//! `TBD_MissionSlotStruct` and `TBD_MissionOrbatGroupStruct` do not declare those members, so
//! the primary parse cannot see them. Spawn put every body on the exact authored (x, z).
//! This file is the reader. Flatten does not emit the keys (T-946.36); hand-staged 1.3 JSON
//! and golden `schema-1_3-wire-fields.json` reach this pass. Editor UI is NOT this slice.
//!
//! == Why a second JsonLoadContext pass =======================================================
//! Same pattern as `TBD_VehicleState.c` (T-680) / `TBD_GroupState.c` (T-678): a second pass
//! over `TBD_MissionLoader.GetRawJson()` with a root that declares `slots[]` and
//! `orbat.*.groups[]` placementRadius/placementShape and nothing else. MissionLoader and
//! MissionSlotStruct stay out of this slice's owns list.
//!
//! == Presence, and the nested-ref landmine ===================================================
//! `JsonLoadContext.ReadValue` ALLOCATES a nested `ref <class>` field even when the JSON key
//! is ABSENT. `slots` / `groups` are ARRAYS, so presence is a null-or-Count() test. Radius
//! can be authored as 0 (exact spawn), so it carries an ABSENT sentinel. Shape is a STRING;
//! presence is emptiness. Absent radius and authored 0 both scatter as exact spawn.
//!
//! == Scatter contract ========================================================================
//! `Scatter(center, radius, shape, seed)` is deterministic. Radius <= 0 returns `center`
//! unchanged (byte-identical spawn to pre-T-679). Shape `square` is axis-aligned
//! [-radius, +radius] on X and Z; anything else (including empty / `circle`) is a uniform
//! disk of that radius.
//!
//! Slot spawn: scatter around the slot's authored (x, z) with the slot's radius/shape.
//! Group spawn: one SHARED offset (same seed per faction:callsign) added to every member,
//! so a group radius jitters the squad together and keeps relative seats. Both compose.
//! Seed is derived from the slot key (uid, else id) as the spec requires.
//!
//! Horizontal only. SpawnSlotBody still owns Y (JSON y, else surface + capsule).
//!
//! == What this file CANNOT prove =============================================================
//! The gate is `cargo xtask mod compile`. It cannot run a round. Whether a squad with
//! radius 20 actually spreads, and whether radius 0 matches today's pin, is a human
//! checklist item. Live `/compiled` will not carry the keys until flatten emits them.
//! @contract mission.schema.json#/$defs/slot
//! @contract mission.schema.json#/$defs/group

//------------------------------------------------------------------------------------------------
//! One `slots[]` row's scatter fields. Field names are the JSON keys.
class TBD_PlacementScatterSlotWireStruct
{
	static const float ABSENT = -1000000;

	string id;
	string uid;
	float placementRadius = -1000000; //!< Metres. ABSENT when omitted. 0 is authored exact.
	string placementShape;             //!< circle|square. Empty when omitted.

	//------------------------------------------------------------------------------------------------
	bool HasRadius()
	{
		return placementRadius != ABSENT;
	}
}

//------------------------------------------------------------------------------------------------
//! One `$defs/group` object's scatter fields. Field names are the JSON keys.
class TBD_PlacementScatterGroupWireStruct
{
	static const float ABSENT = -1000000;

	string callsign;
	float placementRadius = -1000000; //!< Metres. ABSENT when omitted. 0 is authored exact.
	string placementShape;             //!< circle|square. Empty when omitted.

	//------------------------------------------------------------------------------------------------
	bool HasRadius()
	{
		return placementRadius != ABSENT;
	}
}

//------------------------------------------------------------------------------------------------
class TBD_PlacementScatterFactionWireStruct
{
	ref array<ref TBD_PlacementScatterGroupWireStruct> groups;
}

//------------------------------------------------------------------------------------------------
//! Root of the second parse. Declares `slots` and `orbat` and nothing else.
class TBD_PlacementScatterDocStruct
{
	ref array<ref TBD_PlacementScatterSlotWireStruct> slots;
	ref map<string, ref TBD_PlacementScatterFactionWireStruct> orbat;
}

//------------------------------------------------------------------------------------------------
//! Flattened group row after parse (faction key is the orbat map key, not a JSON field).
class TBD_PlacementScatterGroupRow
{
	string faction;
	string callsign;
	float placementRadius;
	string placementShape;
}

//------------------------------------------------------------------------------------------------
//! Server-side reader: bind placementRadius/placementShape and scatter slot spawn points.
class TBD_PlacementScatter
{
	static const string CH = "Scatter";
	static const string SHAPE_CIRCLE = "circle";
	static const string SHAPE_SQUARE = "square";

	protected static ref array<ref TBD_PlacementScatterSlotWireStruct> s_aSlots;
	protected static ref array<ref TBD_PlacementScatterGroupRow> s_aGroups;
	protected static bool s_bParsed;
	protected static string s_sParsedForMission;

	//------------------------------------------------------------------------------------------------
	//! Deterministic scatter. Radius <= 0 returns center unchanged.
	static vector Scatter(vector center, float radius, string shape, int seed)
	{
		if (radius <= 0)
			return center;

		if (shape == SHAPE_SQUARE)
		{
			float dx = (UnitFloat(seed, 1) * 2.0) - 1.0;
			float dz = (UnitFloat(seed, 2) * 2.0) - 1.0;
			return Vector(center[0] + (dx * radius), center[1], center[2] + (dz * radius));
		}

		float u = UnitFloat(seed, 1);
		float theta = UnitFloat(seed, 2) * Math.PI * 2.0;
		float r = Math.Sqrt(u) * radius;
		float cx = r * Math.Cos(theta);
		float cz = r * Math.Sin(theta);
		return Vector(center[0] + cx, center[1], center[2] + cz);
	}

	//------------------------------------------------------------------------------------------------
	//! Slot + group scatter for one spawn. Call from SpawnSlotBody (initial and respawn).
	//! Zero / absent radius on both layers returns Vector(x, 0, z) -- the authored point.
	static vector ForSlot(string slotKey, string slotId, string faction, string groupCallsign, float x, float z)
	{
		vector center = Vector(x, 0, z);
		if (!EnsureParsed())
			return center;

		TBD_PlacementScatterSlotWireStruct slotWire = FindSlot(slotKey, slotId);
		float slotR = 0;
		string slotShape = SHAPE_CIRCLE;
		if (slotWire && slotWire.HasRadius())
		{
			slotR = slotWire.placementRadius;
			if (!slotWire.placementShape.IsEmpty())
				slotShape = slotWire.placementShape;
		}

		int slotSeed = SeedFromString(slotKey);
		if (slotKey.IsEmpty())
			slotSeed = SeedFromString(slotId);

		vector pos = Scatter(center, slotR, slotShape, slotSeed);

		TBD_PlacementScatterGroupRow groupRow = FindGroup(faction, groupCallsign);
		float groupR = 0;
		string groupShape = SHAPE_CIRCLE;
		if (groupRow && groupRow.placementRadius != TBD_PlacementScatterSlotWireStruct.ABSENT)
		{
			groupR = groupRow.placementRadius;
			if (!groupRow.placementShape.IsEmpty())
				groupShape = groupRow.placementShape;
		}

		int groupSeed = SeedFromString(faction + ":" + groupCallsign);
		vector origin = Vector(0, 0, 0);
		vector groupOffset = Scatter(origin, groupR, groupShape, groupSeed);
		vector outPos = Vector(pos[0] + groupOffset[0], 0, pos[2] + groupOffset[2]);

		if (slotR > 0 || groupR > 0)
		{
			Print(string.Format("[TBD][Scatter] slot=%1 from=%2,%3 to=%4,%5 slotR=%6 groupR=%7",
				slotKey, x, z, outPos[0], outPos[2], slotR, groupR));
		}

		return outPos;
	}

	//------------------------------------------------------------------------------------------------
	protected static string CurrentMissionId()
	{
		TBD_MissionDocumentStruct doc = TBD_MissionLoader.GetMission();
		if (!doc || !doc.meta)
			return string.Empty;

		return doc.meta.id;
	}

	//------------------------------------------------------------------------------------------------
	protected static bool EnsureParsed()
	{
		string missionId = CurrentMissionId();
		if (s_bParsed && missionId == s_sParsedForMission)
			return true;

		s_aSlots = new array<ref TBD_PlacementScatterSlotWireStruct>();
		s_aGroups = new array<ref TBD_PlacementScatterGroupRow>();

		string raw = TBD_MissionLoader.GetRawJson();
		if (raw.IsEmpty())
			return false;

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromString(raw))
		{
			Print("[TBD][Scatter] the mission document did not parse as JSON on the scatter pass - spawn stays exact", LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		TBD_PlacementScatterDocStruct doc = new TBD_PlacementScatterDocStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print("[TBD][Scatter] the mission document parsed but its root would not read on the scatter pass - spawn stays exact", LogLevel.ERROR);
			s_bParsed = true;
			s_sParsedForMission = missionId;
			return true;
		}

		if (doc.slots)
			s_aSlots = doc.slots;

		if (doc.orbat)
		{
			foreach (string factionKey, TBD_PlacementScatterFactionWireStruct faction : doc.orbat)
			{
				CollectFaction(factionKey, faction);
			}
		}

		s_bParsed = true;
		s_sParsedForMission = missionId;
		return true;
	}

	//------------------------------------------------------------------------------------------------
	protected static void CollectFaction(string factionKey, TBD_PlacementScatterFactionWireStruct faction)
	{
		if (!faction || !faction.groups)
			return;

		foreach (TBD_PlacementScatterGroupWireStruct group : faction.groups)
		{
			if (!group)
				continue;
			if (group.callsign.IsEmpty())
				continue;

			TBD_PlacementScatterGroupRow row = new TBD_PlacementScatterGroupRow();
			row.faction = factionKey;
			row.callsign = group.callsign;
			row.placementRadius = group.placementRadius;
			row.placementShape = group.placementShape;
			s_aGroups.Insert(row);
		}
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_PlacementScatterSlotWireStruct FindSlot(string slotKey, string slotId)
	{
		if (!s_aSlots)
			return null;

		foreach (TBD_PlacementScatterSlotWireStruct row : s_aSlots)
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

		foreach (TBD_PlacementScatterSlotWireStruct row : s_aSlots)
		{
			if (!row)
				continue;
			if (!slotId.IsEmpty() && row.id == slotId)
				return row;
			if (!slotKey.IsEmpty() && row.id == slotKey)
				return row;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	protected static TBD_PlacementScatterGroupRow FindGroup(string faction, string groupCallsign)
	{
		if (!s_aGroups)
			return null;
		if (faction.IsEmpty())
			return null;
		if (groupCallsign.IsEmpty())
			return null;

		foreach (TBD_PlacementScatterGroupRow row : s_aGroups)
		{
			if (!row)
				continue;
			if (row.faction != faction)
				continue;
			if (row.callsign != groupCallsign)
				continue;
			return row;
		}

		return null;
	}

	//------------------------------------------------------------------------------------------------
	//! djb2 over bytes. Slot ids are ASCII (faction:callsign:role:n / uid).
	protected static int SeedFromString(string s)
	{
		int h = 5381;
		if (s.IsEmpty())
			return h;

		int n = s.Length();
		int i = 0;
		while (i < n)
		{
			int code = s.Get(i).ToAscii();
			h = (h * 33) + code;
			i++;
		}

		return h;
	}

	//------------------------------------------------------------------------------------------------
	//! Deterministic unit float in [0, 1). Not Math.Random* (those are global / non-repeatable).
	protected static float UnitFloat(int seed, int salt)
	{
		int x = seed + (salt * 374761393);
		x = x * 1103515245 + 12345;
		int mag = x;
		if (mag < 0)
			mag = -1 - mag;
		int unit = mag % 1000000;
		return unit / 1000000.0;
	}
}
