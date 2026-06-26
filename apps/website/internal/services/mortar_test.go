package services

import (
	"math"
	"testing"
)

func TestSolveAzimuthCardinals(t *testing.T) {
	cases := []struct {
		name   string
		dx, dy float64
		wantAz float64
	}{
		{"north", 0, 100, 0},
		{"east", 100, 0, 90},
		{"south", 0, -100, 180},
		{"west", -100, 0, 270},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			sol, ok := SolveFireMission("M252 81mm", 0, 0, tc.dx, tc.dy)
			if !ok {
				t.Fatalf("expected in range")
			}
			if math.Abs(sol.AzimuthDeg-tc.wantAz) > 0.1 {
				t.Errorf("azimuth = %v, want %v", sol.AzimuthDeg, tc.wantAz)
			}
		})
	}
}

func TestSolveDistanceAndHighAngle(t *testing.T) {
	sol, ok := SolveFireMission("M252 81mm", 0, 0, 0, 1240)
	if !ok {
		t.Fatal("expected in range")
	}
	if sol.DistanceM != 1240 {
		t.Errorf("distance = %d, want 1240", sol.DistanceM)
	}
	// High-angle mortar fire: elevation must be above 800 mils (>45°).
	if sol.ElevationMils <= 800 || sol.ElevationMils >= 1600 {
		t.Errorf("elevation = %d mils, want high-angle (800-1600)", sol.ElevationMils)
	}
	if sol.TimeOfFlightS <= 0 {
		t.Errorf("time of flight = %v, want > 0", sol.TimeOfFlightS)
	}
}

func TestSolveLowerChargeForShorterRange(t *testing.T) {
	near, _ := SolveFireMission("M252 81mm", 0, 0, 0, 300)
	far, _ := SolveFireMission("M252 81mm", 0, 0, 0, 1500)
	if near.Charge > far.Charge {
		t.Errorf("closer target should not need a higher charge: near=%d far=%d", near.Charge, far.Charge)
	}
}

func TestSolveOutOfRange(t *testing.T) {
	if _, ok := SolveFireMission("M252 81mm", 0, 0, 0, 100000); ok {
		t.Fatal("expected out of range for 100km")
	}
}

func TestSolveUnknownWeaponFallsBack(t *testing.T) {
	sol, ok := SolveFireMission("Potato Launcher", 0, 0, 0, 500)
	if !ok || sol.WeaponSystem != DefaultMortar {
		t.Fatalf("expected fallback to %s, got %q ok=%v", DefaultMortar, sol.WeaponSystem, ok)
	}
}
