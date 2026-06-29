package services

import (
	"encoding/json"
	"sort"
)

// OrbatSlotTemplate is one ordered, distinct slot in a squad: a role with its
// loadout and an optional specialization tag (e.g. "MED" / "ENG").
type OrbatSlotTemplate struct {
	Role    string `json:"role"`
	Loadout string `json:"loadout"`
	Tag     string `json:"tag"`
}

// OrbatSquadTemplate is a squad and its ordered slot list. The slot's position in
// the list is its 1-based number on the ORBAT.
type OrbatSquadTemplate struct {
	Faction  string              `json:"faction"`
	Callsign string              `json:"callsign"`
	Squad    string              `json:"squad"`
	Slots    []OrbatSlotTemplate `json:"slots"`
}

// ParseOrbatTemplate extracts the ORBAT squad list from a mission version payload —
// the source of automated event ORBAT (factions/squads/roles are derived from the
// uploaded mission.json rather than hand-created).
//
// Two sources, in priority order:
//  1. A top-level "orbat" array (legacy / explicit) — used verbatim when present.
//  2. Otherwise derived from the editor block. Save Version (T-062.1.1) omits the
//     redundant "orbat" to halve the payload, so the editor graph
//     (factions → squads → slots) is the only ORBAT source for those versions.
func ParseOrbatTemplate(payload []byte) []OrbatSquadTemplate {
	var p struct {
		Orbat []OrbatSquadTemplate `json:"orbat"`
	}
	//nolint:errcheck // best-effort: an absent/invalid top-level "orbat" intentionally falls back to the editor-derived ORBAT.
	_ = json.Unmarshal(payload, &p)
	if len(p.Orbat) > 0 {
		return p.Orbat
	}
	return deriveOrbatFromEditor(payload)
}

// editorPayload mirrors the normalized graph the frontend compiler writes under
// "editor" (compile.ts assemblePayload): factions/squads/slots linked by id arrays.
type editorPayload struct {
	Editor struct {
		Factions []struct {
			Key      string   `json:"key"`
			SquadIDs []string `json:"squadIds"`
		} `json:"factions"`
		Squads []struct {
			ID       string   `json:"id"`
			Callsign string   `json:"callsign"`
			Name     string   `json:"name"`
			SlotIDs  []string `json:"slotIds"`
		} `json:"squads"`
		Slots []struct {
			ID    string `json:"id"`
			Index int    `json:"index"`
			Role  string `json:"role"`
			Tag   string `json:"tag"`
		} `json:"slots"`
	} `json:"editor"`
}

// deriveOrbatFromEditor reconstructs the ORBAT template from the editor graph,
// mirroring compile.ts compileMission order EXACTLY so a derived ORBAT is identical
// to the one the frontend would have shipped in "orbat": iterate factions in array
// order → each faction.squadIds → resolve squad → each squad.slotIds → resolve slots
// → sort by slot.index ascending. loadout is always "" (the frontend emits "" until
// the Arsenal phase resolves real loadout names).
func deriveOrbatFromEditor(payload []byte) []OrbatSquadTemplate {
	var e editorPayload
	if err := json.Unmarshal(payload, &e); err != nil {
		return nil
	}
	ed := e.Editor
	if len(ed.Factions) == 0 {
		return nil
	}

	type slotRow struct {
		index int
		role  string
		tag   string
	}
	squadsByID := make(map[string]struct {
		callsign string
		name     string
		slotIDs  []string
	}, len(ed.Squads))
	for _, sq := range ed.Squads {
		squadsByID[sq.ID] = struct {
			callsign string
			name     string
			slotIDs  []string
		}{sq.Callsign, sq.Name, sq.SlotIDs}
	}
	slotsByID := make(map[string]slotRow, len(ed.Slots))
	for _, sl := range ed.Slots {
		slotsByID[sl.ID] = slotRow{sl.Index, sl.Role, sl.Tag}
	}

	out := make([]OrbatSquadTemplate, 0, len(ed.Squads))
	for _, f := range ed.Factions {
		for _, squadID := range f.SquadIDs {
			sq, ok := squadsByID[squadID]
			if !ok {
				continue
			}
			rows := make([]slotRow, 0, len(sq.slotIDs))
			for _, slotID := range sq.slotIDs {
				if sl, ok := slotsByID[slotID]; ok {
					rows = append(rows, sl)
				}
			}
			sort.SliceStable(rows, func(i, j int) bool { return rows[i].index < rows[j].index })
			slots := make([]OrbatSlotTemplate, len(rows))
			for i, r := range rows {
				slots[i] = OrbatSlotTemplate{Role: r.role, Loadout: "", Tag: r.tag}
			}
			out = append(out, OrbatSquadTemplate{
				Faction:  f.Key,
				Callsign: sq.callsign,
				Squad:    sq.name,
				Slots:    slots,
			})
		}
	}
	return out
}
