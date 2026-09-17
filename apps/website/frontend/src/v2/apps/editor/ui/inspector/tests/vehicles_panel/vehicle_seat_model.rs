//! Tests the vehicles panel subject.

    use super::{seat_model, DEFAULT_CARGO_SEATS, VEHICLE_CARGO_KINDS};

    /// T-076 — the generic seat model the crew list draws: three fixed stations
    /// (driver/gunner/commander) then N cargo seats, `cargo1`…`cargoN`. Pins the seat_ids (the doc
    /// keys written into `vehicle.crew`) and the default cargo count, so a rename or a reorder that
    /// would silently orphan an already-authored `vehicle.crew` entry fails here first.
    #[test]
    fn seat_model_is_driver_gunner_commander_then_cargo() {
        let seats = seat_model(DEFAULT_CARGO_SEATS);
        let ids: Vec<&str> = seats.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "driver",
                "gunner",
                "commander",
                "cargo1",
                "cargo2",
                "cargo3",
                "cargo4"
            ],
            "generic seat ids + default {DEFAULT_CARGO_SEATS} cargo seats, in order"
        );
        // The labels are display copy, but the fixed three must read as their station names.
        assert_eq!(seats[0].1, "Driver");
        assert_eq!(seats[1].1, "Gunner");
        assert_eq!(seats[2].1, "Commander");
        assert_eq!(
            seats[3].1, "Cargo 1",
            "cargo seats are 1-indexed for the operator"
        );

        // Capacity drives the cargo-seat count (the branch the registry will feed once T-205 lands):
        // zero cargo seats leaves exactly the three fixed stations, no `cargoN`.
        let none = seat_model(0);
        assert_eq!(none.len(), 3, "no cargo capacity ⇒ only the fixed stations");
        assert!(
            none.iter().all(|(id, _)| !id.starts_with("cargo")),
            "no cargo seats emitted at capacity 0"
        );
    }

    /// A vehicle's cargo picker must never offer a person or another vehicle. `character` rows are
    /// crews (ORBAT slots, not freight) and nesting a vehicle inside a vehicle is not something the
    /// engine's storage does — either would author a document whose only failure mode is silence.
    #[test]
    fn vehicle_cargo_picker_excludes_people_and_vehicles() {
        for banned in ["character", "vehicle", "vehicle_weapon", "other"] {
            assert!(
                !VEHICLE_CARGO_KINDS.contains(&banned),
                "{banned} must not be offered as vehicle cargo"
            );
        }
        // …and it is a genuine superset of the worn-garment list, which is the whole reason it is a
        // separate constant rather than a reuse of `arsenal::CARGO_ADD_KINDS`.
        for expected in ["magazine", "ammo", "gear_primary", "gear_backpack"] {
            assert!(
                VEHICLE_CARGO_KINDS.contains(&expected),
                "{expected} is exactly what a resupply vehicle carries"
            );
        }
    }

    /// T-668 — the placed-vehicle header row wears HOVER_FILL (the one chrome hover fill), not the
    /// weaker ad-hoc `hover:bg-white/5` it used to. The panel is wasm-only so a native test cannot
    /// render it; this reads the source. The needle is assembled so this test's own prose can't
    /// satisfy the absence check.
    #[test]
    fn header_row_uses_hover_fill_not_the_weak_ad_hoc_fill() {
        use crate::v2::core::test_support::class_r_scrub::live_code;
        let code = live_code(include_str!("vehicles_panel.rs"));
        assert!(
            code.contains("HOVER_FILL"),
            "the header row must consume HOVER_FILL"
        );
        let weak = ["hover:bg-", "white/5"].concat();
        assert!(
            !code.contains(&weak),
            "T-668: the ad-hoc `hover:bg-white/5` header fill must be gone (use HOVER_FILL)"
        );
    }
