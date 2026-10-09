/**
 * @file TBD_MissionSettingsStruct.c
 * @brief The mission policy block: respawn, spectator policy and night vision.
 *
 * Role: the typed form of `settings`.  Position: filled by `TBD_MissionLoader`'s parse; applied by
 * `TBD_MissionWorldApplier.ApplyMissionSettings`; the unhonoured values are warned by
 * `TBD_MissionUnconsumedKeyCheck`.
 * State: none; plain data.  Invariants: the block is always allocated, so callers read field
 * values, never the reference; `nightVision` absent and authored false are the same value.
 */

//! Mission policy. All three fields are optional; an empty string means the key was absent.
//! @contract mission.schema.json#/$defs/settings
class TBD_MissionSettingsStruct
{
	string respawn;         //!< JSON `respawn`: respawn mode from the schema enum ("none", "tickets", ...); empty = absent.
	string spectatorPolicy; //!< JSON `spectatorPolicy`: "none", "own_side_delayed_60s" or "free"; empty = absent.
	bool nightVision;       //!< JSON `nightVision`: authored NVG policy; default false = off or absent.
}
