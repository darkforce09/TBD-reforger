package services

import "testing"

func TestParseOrbatTemplate_LegacyOrbatWins(t *testing.T) {
	// A payload carrying BOTH a top-level orbat and an editor block: the explicit
	// orbat must be used verbatim (derivation is skipped).
	payload := []byte(`{
		"orbat": [
			{"faction":"BLUFOR","callsign":"HQ","squad":"Command","slots":[
				{"role":"Commander","loadout":"","tag":""}
			]}
		],
		"editor": {
			"factions":[{"key":"OPFOR","squadIds":["s1"]}],
			"squads":[{"id":"s1","name":"Recon","slotIds":["x1"]}],
			"slots":[{"id":"x1","index":0,"role":"Sniper","tag":""}]
		}
	}`)
	got := ParseOrbatTemplate(payload)
	if len(got) != 1 || got[0].Faction != "BLUFOR" || got[0].Squad != "Command" {
		t.Fatalf("legacy orbat not used verbatim: %+v", got)
	}
	if len(got[0].Slots) != 1 || got[0].Slots[0].Role != "Commander" {
		t.Fatalf("legacy slot wrong: %+v", got[0].Slots)
	}
}

func TestParseOrbatTemplate_DerivesFromEditor(t *testing.T) {
	// Editor-only payload (NO orbat key) with two squads under one faction. Slot ids
	// are listed out of index order to prove the derivation sorts by index ascending.
	payload := []byte(`{
		"editor": {
			"factions":[{"key":"BLUFOR","squadIds":["sq-a","sq-b"]}],
			"squads":[
				{"id":"sq-a","callsign":"Alpha Actual","name":"Alpha 1-1","slotIds":["s2","s0","s1"]},
				{"id":"sq-b","name":"Bravo 1-1","slotIds":["b0"]}
			],
			"slots":[
				{"id":"s0","index":0,"role":"Squad Leader","tag":""},
				{"id":"s1","index":1,"role":"Combat Medic","tag":"MED"},
				{"id":"s2","index":2,"role":"Rifleman","tag":""},
				{"id":"b0","index":0,"role":"Team Leader","tag":""}
			]
		}
	}`)
	got := ParseOrbatTemplate(payload)
	if len(got) != 2 {
		t.Fatalf("expected 2 squads, got %d: %+v", len(got), got)
	}

	alpha := got[0]
	if alpha.Faction != "BLUFOR" || alpha.Callsign != "Alpha Actual" || alpha.Squad != "Alpha 1-1" {
		t.Fatalf("alpha squad header wrong: %+v", alpha)
	}
	if len(alpha.Slots) != 3 {
		t.Fatalf("alpha expected 3 slots, got %d", len(alpha.Slots))
	}
	wantRoles := []string{"Squad Leader", "Combat Medic", "Rifleman"}
	for i, want := range wantRoles {
		if alpha.Slots[i].Role != want {
			t.Fatalf("alpha slot %d role = %q, want %q (order not sorted by index?)", i, alpha.Slots[i].Role, want)
		}
		if alpha.Slots[i].Loadout != "" {
			t.Fatalf("derived loadout should be empty, got %q", alpha.Slots[i].Loadout)
		}
	}
	if alpha.Slots[1].Tag != "MED" {
		t.Fatalf("medic tag = %q, want MED", alpha.Slots[1].Tag)
	}

	bravo := got[1]
	if bravo.Squad != "Bravo 1-1" || bravo.Callsign != "" || len(bravo.Slots) != 1 || bravo.Slots[0].Role != "Team Leader" {
		t.Fatalf("bravo squad wrong: %+v", bravo)
	}
}

func TestParseOrbatTemplate_EmptyPayload(t *testing.T) {
	for _, p := range []string{`{}`, `{"editor":{}}`, ``, `not json`} {
		if got := ParseOrbatTemplate([]byte(p)); len(got) != 0 {
			t.Fatalf("payload %q: expected empty, got %+v", p, got)
		}
	}
}

func TestParseOrbatTemplate_SkipsMissingRefs(t *testing.T) {
	// squadIds / slotIds referencing absent entities are skipped, not panicked on.
	payload := []byte(`{
		"editor": {
			"factions":[{"key":"BLUFOR","squadIds":["sq-a","ghost"]}],
			"squads":[{"id":"sq-a","name":"Alpha","slotIds":["s0","ghost-slot"]}],
			"slots":[{"id":"s0","index":0,"role":"Rifleman","tag":""}]
		}
	}`)
	got := ParseOrbatTemplate(payload)
	if len(got) != 1 || got[0].Squad != "Alpha" || len(got[0].Slots) != 1 {
		t.Fatalf("missing-ref handling wrong: %+v", got)
	}
}
