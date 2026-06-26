package services

import "math"

// FireSolution is the computed firing data for a mortar fire mission.
type FireSolution struct {
	WeaponSystem  string  `json:"weapon_system"`
	DistanceM     int     `json:"distance_m"`
	AzimuthDeg    float64 `json:"azimuth_deg"`
	AzimuthMils   int     `json:"azimuth_mils"`
	ElevationMils int     `json:"elevation_mils"`
	Charge        int     `json:"charge"`
	TimeOfFlightS float64 `json:"time_of_flight_s"`
}

// mortarCharges maps a weapon system to its per-ring muzzle velocities (m/s).
// These are representative values for a simplified projectile model — real
// deployments would replace this with the weapon's published firing tables.
var mortarCharges = map[string][]float64{
	"M252 81mm":  {70, 105, 150, 210, 270},
	"M821 81mm":  {70, 105, 150, 210, 270},
	"2B14 82mm":  {65, 100, 145, 200, 255},
	"M120 120mm": {110, 170, 230, 318},
}

const (
	gravity       = 9.80665
	milsPerCircle = 6400.0
)

// DefaultMortar is used when an unknown weapon is requested.
const DefaultMortar = "M252 81mm"

// SolveFireMission computes the high-angle firing solution from a firing
// position to a target, both in flat game-world meters (x = east, y = north).
// It selects the lowest charge that can reach the target. Returns ok=false if
// the target is beyond maximum range for every charge.
func SolveFireMission(weapon string, fpX, fpY, tgtX, tgtY float64) (FireSolution, bool) {
	charges, ok := mortarCharges[weapon]
	if !ok {
		weapon = DefaultMortar
		charges = mortarCharges[DefaultMortar]
	}

	dx := tgtX - fpX
	dy := tgtY - fpY
	rng := math.Hypot(dx, dy)

	// Grid azimuth: clockwise from north (+y) toward east (+x).
	azDeg := math.Atan2(dx, dy) * 180 / math.Pi
	if azDeg < 0 {
		azDeg += 360
	}

	sol := FireSolution{
		WeaponSystem: weapon,
		DistanceM:    int(math.Round(rng)),
		AzimuthDeg:   math.Round(azDeg*10) / 10,
		AzimuthMils:  int(math.Round(azDeg * milsPerCircle / 360)),
	}

	for ch, v := range charges {
		k := rng * gravity / (v * v) // = sin(2θ)
		if k <= 1 {
			// High-angle (mortar) root: 2θ = 180° − arcsin(k).
			theta := (math.Pi - math.Asin(k)) / 2
			sol.Charge = ch
			sol.ElevationMils = int(math.Round(theta * 180 / math.Pi * milsPerCircle / 360))
			sol.TimeOfFlightS = math.Round(2*v*math.Sin(theta)/gravity*10) / 10
			return sol, true
		}
	}
	return sol, false
}
