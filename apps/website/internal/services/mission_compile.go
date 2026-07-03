package services

import (
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"regexp"
	"sort"
	"strings"

	"github.com/tbd-milsim/reforger-backend/internal/contract"
	"github.com/tbd-milsim/reforger-backend/internal/models"
)

// Mission compile flatten (T-092.2) — the Go twin of the frontend
// flattenModDocument.ts. It derives the CANONICAL mod mission document
// (mission.schema.json, string schemaVersion "1.1"/"1.2") from a mission row +
// its current version's editor payload, mirroring the TS traversal EXACTLY
// (same slot-id/kit/orbat/zone/default rules) so GET /missions/:id/compiled and
// the client-side flatten agree — the deriveOrbatFromEditor precedent, extended
// to the full document.
//
// Locked coordinate mapping (t092_spawn_transform_program.md):
//
//	editor position.x -> slot.x · position.y -> slot.z ·
//	position.z -> slot.y (optional, 1.2) · position.rotation -> headingDeg
//
// @contract mission.schema.json#/

// ModSlot is one flattened slots[] entry of the compiled document.
type ModSlot struct {
	ID            string   `json:"id"`
	Faction       string   `json:"faction"`
	GroupCallsign string   `json:"groupCallsign"`
	Role          string   `json:"role"`
	Kit           string   `json:"kit"`
	X             float64  `json:"x"`
	Z             float64  `json:"z"`
	Y             *float64 `json:"y,omitempty"`
	HeadingDeg    float64  `json:"headingDeg"`
}

// ModOrbatRole / ModOrbatGroup mirror mission.schema.json $defs/role + $defs/group.
type ModOrbatRole struct {
	Slot  string `json:"slot"`
	Kit   string `json:"kit"`
	Count int    `json:"count"`
}

type ModOrbatGroup struct {
	Callsign string         `json:"callsign"`
	Type     string         `json:"type"`
	Roles    []ModOrbatRole `json:"roles"`
}

type ModOrbatFaction struct {
	Groups []ModOrbatGroup `json:"groups"`
}

// ModZone models the synthesized spawn zones (circle shapes only here).
type ModZone struct {
	ID      string       `json:"id"`
	Type    string       `json:"type"`
	Faction string       `json:"faction,omitempty"`
	Shape   ModZoneShape `json:"shape"`
}

type ModZoneShape struct {
	Circle ModCircle `json:"circle"`
}

type ModCircle struct {
	X float64 `json:"x"`
	Z float64 `json:"z"`
	R float64 `json:"r"`
}

type ModFaction struct {
	Key         string `json:"key"`
	DisplayName string `json:"displayName"`
	PresetID    string `json:"presetId"`
	Tickets     int    `json:"tickets"`
}

type ModMeta struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Author      string `json:"author,omitempty"`
	Terrain     string `json:"terrain"`
	TemplateID  string `json:"templateId"`
	PlayerRange [2]int `json:"playerRange"`
}

type ModEnvironment struct {
	DateTime      string `json:"dateTime,omitempty"`
	WeatherPreset string `json:"weatherPreset,omitempty"`
}

type ModFlow struct {
	BriefingSeconds  int    `json:"briefingSeconds"`
	SafeStartSeconds int    `json:"safeStartSeconds"`
	TimeLimitSeconds int    `json:"timeLimitSeconds"`
	JIP              string `json:"jip"`
}

type ModWinConditions struct {
	Mode  string   `json:"mode"`
	EndOn []string `json:"endOn"`
}

// ModMissionDocument is the full compiled document body served to the game server.
type ModMissionDocument struct {
	SchemaVersion string                     `json:"schemaVersion"`
	Meta          ModMeta                    `json:"meta"`
	Environment   *ModEnvironment            `json:"environment,omitempty"`
	Factions      []ModFaction               `json:"factions"`
	Orbat         map[string]ModOrbatFaction `json:"orbat"`
	Slots         []ModSlot                  `json:"slots"`
	Zones         []ModZone                  `json:"zones"`
	Flow          ModFlow                    `json:"flow"`
	WinConditions ModWinConditions           `json:"winConditions"`
}

// ErrNoSlots marks a mission whose current version has no placed slots — a 1.1/1.2
// document requires slots[], so there is nothing valid to serve.
var ErrNoSlots = errors.New("mission version has no placed slots")

// compileEditorPayload mirrors the graph flattenModDocument.ts walks: editorPayload
// (mission_payload.go) plus the slot position/assetId fields the ORBAT derive skips.
type compileEditorPayload struct {
	Editor struct {
		Factions []struct {
			Key      string   `json:"key"`
			Name     string   `json:"name"`
			SquadIDs []string `json:"squadIds"`
		} `json:"factions"`
		Squads []struct {
			ID       string   `json:"id"`
			Callsign string   `json:"callsign"`
			Name     string   `json:"name"`
			SlotIDs  []string `json:"slotIds"`
		} `json:"squads"`
		Slots []struct {
			ID       string `json:"id"`
			Index    int    `json:"index"`
			Role     string `json:"role"`
			AssetID  string `json:"assetId"`
			Position struct {
				X        float64 `json:"x"`
				Y        float64 `json:"y"`
				Z        float64 `json:"z"`
				Rotation float64 `json:"rotation"`
			} `json:"position"`
		} `json:"slots"`
	} `json:"editor"`
}

const (
	compileDateAnchor   = "1989-06-14" // editor only authors HH:MM
	spawnZoneRadiusM    = 150
	compileTemplateID   = "editor_v1"
	flowBriefingSec     = 600
	flowSafeStartSec    = 300
	flowTimeLimitSec    = 5400
	flowJIP             = "until_safestart_end"
	winMode             = "attrition"
	factionStubOpfor    = "opfor"
	factionStubBlufor   = "blufor"
	rifleSquadGroupType = "rifle_squad"
)

var nonSlugChars = regexp.MustCompile(`[^a-z0-9_]+`)
var nonAlnumChars = regexp.MustCompile(`[^a-z0-9]+`)

// slugKey lowercases into the schema's ^[a-z][a-z0-9_]*$ pattern (factionKey/terrain/templateId).
func slugKey(raw, fallback string) string {
	s := nonSlugChars.ReplaceAllString(strings.ToLower(raw), "_")
	s = strings.Trim(s, "_")
	if s == "" {
		return fallback
	}
	if s[0] < 'a' || s[0] > 'z' {
		s = "f_" + s
	}
	return s
}

// missionDocID reduces the mission UUID to the schema's ^msn_[a-z0-9]+$ id space.
func missionDocID(id string) string {
	hex := nonAlnumChars.ReplaceAllString(strings.ToLower(id), "")
	if hex == "" {
		hex = "editor"
	}
	return "msn_" + hex
}

func normalizeHeading(rotation float64) float64 {
	if math.IsNaN(rotation) || math.IsInf(rotation, 0) {
		return 0
	}
	return math.Mod(math.Mod(rotation, 360)+360, 360)
}

// FlattenToModDocument builds the compiled mod mission document from the mission row
// and its current version payload. Fields the editor never authors (zones, flow,
// winConditions, templateId, playerRange, presetId) are synthesized with the same
// defaults as flattenModDocument.ts. Returns ErrNoSlots when the editor graph holds no
// placed slots. The optional slot y is emitted only for a finite, non-zero editor
// position.z (pre-DEM missions carry z=0 placeholders that must fall back to surface Y).
//
//nolint:gocognit,gocyclo,cyclop,funlen // single authoritative traversal mirroring the TS flatten one-for-one; splitting it would desync the twins
func FlattenToModDocument(m *models.Mission, payload []byte) (*ModMissionDocument, error) {
	aliases, err := contract.LoadKitAliases()
	if err != nil {
		return nil, err
	}

	var p compileEditorPayload
	if err := json.Unmarshal(payload, &p); err != nil {
		return nil, fmt.Errorf("parse mission version payload: %w", err)
	}
	ed := p.Editor

	type squadRow struct {
		callsign string
		name     string
		slotIDs  []string
	}
	squadsByID := make(map[string]squadRow, len(ed.Squads))
	for _, sq := range ed.Squads {
		squadsByID[sq.ID] = squadRow{sq.Callsign, sq.Name, sq.SlotIDs}
	}
	type slotRow = struct {
		ID       string `json:"id"`
		Index    int    `json:"index"`
		Role     string `json:"role"`
		AssetID  string `json:"assetId"`
		Position struct {
			X        float64 `json:"x"`
			Y        float64 `json:"y"`
			Z        float64 `json:"z"`
			Rotation float64 `json:"rotation"`
		} `json:"position"`
	}
	slotsByID := make(map[string]slotRow, len(ed.Slots))
	for _, sl := range ed.Slots {
		slotsByID[sl.ID] = sl
	}

	doc := &ModMissionDocument{
		SchemaVersion: "1.1",
		Orbat:         make(map[string]ModOrbatFaction, len(ed.Factions)),
	}
	type centroid struct {
		sx, sz float64
		n      int
	}
	centroids := make(map[string]*centroid)
	centroidOrder := make([]string, 0, len(ed.Factions))
	anyY := false

	for _, f := range ed.Factions {
		factionKey := slugKey(f.Key, "faction")
		defaultKit, preset := aliases.FactionDefault(factionKey)
		groups := make([]ModOrbatGroup, 0, len(f.SquadIDs))

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
			if len(rows) == 0 {
				continue
			}
			sort.SliceStable(rows, func(i, j int) bool { return rows[i].Index < rows[j].Index })

			callsign := sq.callsign
			if callsign == "" {
				callsign = sq.name
			}

			roleCounters := make(map[string]int)
			roleIndex := make(map[string]int)
			roles := make([]ModOrbatRole, 0)

			for _, sl := range rows {
				occurrence := roleCounters[sl.Role]
				roleCounters[sl.Role] = occurrence + 1

				kit, mapped := aliases.KitForResource(sl.AssetID)
				if !mapped {
					kit = defaultKit
				}

				if idx, seen := roleIndex[sl.Role]; seen {
					roles[idx].Count++
				} else {
					roleIndex[sl.Role] = len(roles)
					roles = append(roles, ModOrbatRole{Slot: sl.Role, Kit: kit, Count: 1})
				}

				x := sl.Position.X
				z := sl.Position.Y // editor y (map north axis) -> mod z
				var yPtr *float64
				elev := sl.Position.Z // editor z (elevation) -> mod y (optional)
				if elev != 0 && !math.IsNaN(elev) && !math.IsInf(elev, 0) {
					e := elev
					yPtr = &e
					anyY = true
				}

				doc.Slots = append(doc.Slots, ModSlot{
					ID:            fmt.Sprintf("%s:%s:%s:%d", factionKey, callsign, sl.Role, occurrence),
					Faction:       factionKey,
					GroupCallsign: callsign,
					Role:          sl.Role,
					Kit:           kit,
					X:             x,
					Z:             z,
					Y:             yPtr,
					HeadingDeg:    normalizeHeading(sl.Position.Rotation),
				})

				c, ok := centroids[factionKey]
				if !ok {
					c = &centroid{}
					centroids[factionKey] = c
					centroidOrder = append(centroidOrder, factionKey)
				}
				c.sx += x
				c.sz += z
				c.n++
			}

			groups = append(groups, ModOrbatGroup{Callsign: callsign, Type: rifleSquadGroupType, Roles: roles})
		}

		if len(groups) > 0 {
			doc.Orbat[factionKey] = ModOrbatFaction{Groups: groups}
		}
		displayName := f.Name
		if displayName == "" {
			displayName = factionKey
		}
		doc.Factions = append(doc.Factions, ModFaction{
			Key:         factionKey,
			DisplayName: displayName,
			PresetID:    preset,
			Tickets:     0,
		})
	}

	if len(doc.Slots) == 0 {
		return nil, ErrNoSlots
	}
	if anyY {
		doc.SchemaVersion = "1.2"
	}

	// Schema requires >= 2 factions; pad a stub opposing faction for single-faction drafts.
	if len(doc.Factions) < 2 {
		stub := factionStubOpfor
		for _, f := range doc.Factions {
			if f.Key == factionStubOpfor {
				stub = factionStubBlufor
			}
		}
		_, preset := aliases.FactionDefault(stub)
		doc.Factions = append(doc.Factions, ModFaction{
			Key:         stub,
			DisplayName: strings.ToUpper(stub),
			PresetID:    preset,
			Tickets:     0,
		})
	}

	for _, factionKey := range centroidOrder {
		c := centroids[factionKey]
		doc.Zones = append(doc.Zones, ModZone{
			ID:      "z_spawn_" + factionKey,
			Type:    "spawn",
			Faction: factionKey,
			Shape: ModZoneShape{Circle: ModCircle{
				X: math.Round(c.sx/float64(c.n)*10) / 10,
				Z: math.Round(c.sz/float64(c.n)*10) / 10,
				R: spawnZoneRadiusM,
			}},
		})
	}

	maxPlayers := m.MaxPlayers
	if maxPlayers < 1 {
		maxPlayers = len(doc.Slots)
	}
	if maxPlayers < 1 {
		maxPlayers = 1
	}

	terrain := string(m.Terrain)
	if terrain == "custom" && m.CustomTerrainName != "" {
		terrain = m.CustomTerrainName
	}

	doc.Meta = ModMeta{
		ID:          missionDocID(m.ID.String()),
		Name:        m.Title,
		Author:      m.AuthorID,
		Terrain:     slugKey(terrain, "everon"),
		TemplateID:  compileTemplateID,
		PlayerRange: [2]int{1, maxPlayers},
	}
	if m.Title == "" {
		doc.Meta.Name = "Untitled Mission"
	}

	env := &ModEnvironment{WeatherPreset: string(m.Weather)}
	if m.TimeOfDay != "" {
		// Row time_of_day may come back as HH:MM or HH:MM:SS — keep exactly HH:MM.
		t := m.TimeOfDay
		if len(t) > 5 {
			t = t[:5]
		}
		env.DateTime = compileDateAnchor + "T" + t + ":00Z"
	}
	doc.Environment = env

	doc.Flow = ModFlow{
		BriefingSeconds:  flowBriefingSec,
		SafeStartSeconds: flowSafeStartSec,
		TimeLimitSeconds: flowTimeLimitSec,
		JIP:              flowJIP,
	}
	doc.WinConditions = ModWinConditions{Mode: winMode, EndOn: []string{"time_limit", "faction_eliminated"}}

	return doc, nil
}
