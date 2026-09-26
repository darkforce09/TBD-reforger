/**
 * @file TBD_AudioBlockStruct.c
 * @brief The mission document's `audio` block: positional emitters and music cues.
 *
 * Role: typed JSON targets for the audio second-pass read.  Position: filled by
 * `TBD_AudioEmitter.ReadWire` from `TBD_MissionJsonPass.LoadRoot`; read by `TBD_AudioEmitter`.
 * State: none.  Invariants: `JsonLoadContext` allocates a nested ref even when its key is absent,
 * so presence is `emitters.Count()` / `musicCues.Count()`, never a null test on `audio`; the
 * JSON key `event` is renamed to `cueEvent` before the read because `event` is an Enforce keyword.
 */

//! One positional sound source: a client spawns it at (x, z) and plays `sound` inside `radiusM`.
//! @contract mission.schema.json#/$defs/audioEmitter
class TBD_AudioEmitterStruct
{
	string id; //!< JSON `id`; unique across emitters and cues
	float x; //!< JSON `x`; world X in metres
	float z; //!< JSON `z`; world Z in metres
	float y; //!< JSON `y`; metres above sea level; `TBD_AudioEmitter.ABSENT` when omitted
	string sound; //!< JSON `sound`; an `SCR_SoundEvent` name
	float radiusM; //!< JSON `radiusM`; listener radius in metres; must be greater than 0
	bool loop; //!< JSON `loop`; true repeats inside the radius, false plays once on first entry
	string triggerId; //!< JSON `triggerId`; an `editorTriggers[].id` that arms the emitter; empty arms at LIVE

	//! Defaults for keys the document omits: `y` absent, zero radius, one-shot.
	void TBD_AudioEmitterStruct()
	{
		y = TBD_AudioEmitter.ABSENT;
		radiusM = 0;
		loop = false;
	}
}

//! One 2D music cue fired on a named mission event.
//! @contract mission.schema.json#/$defs/musicCue
class TBD_MusicCueStruct
{
	string id; //!< JSON `id`; unique across emitters and cues
	string cueEvent; //!< JSON `event` (renamed before the read); one of the `TBD_AudioEmitter.EV_*` names
	string track; //!< JSON `track`; an `SCR_SoundEvent` name
}

//! The `audio` block: the emitter and cue arrays.
//! @contract mission.schema.json#/$defs/audio
class TBD_AudioBlockStruct
{
	ref array<ref TBD_AudioEmitterStruct> emitters; //!< JSON `emitters`; allocated even when absent
	ref array<ref TBD_MusicCueStruct> musicCues; //!< JSON `musicCues`; allocated even when absent
}

//! The document root for the audio pass: declares `audio` and nothing else.
//! @contract mission.schema.json#/properties/audio
class TBD_AudioDocStruct
{
	ref TBD_AudioBlockStruct audio; //!< JSON `audio`; allocated even when absent
}
