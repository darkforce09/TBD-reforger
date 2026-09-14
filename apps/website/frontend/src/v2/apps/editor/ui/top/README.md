# Top Command Strip (`ui/top/`)

The top strip is split into two distinct 24px rows, adhering to the 48px height contract.

---

## Structure

1. **`primary_bar.md` (Row 1):** Eden main menu bar, editable mission title, save status indicator, quick time/weather scrubber, and settings gear.
2. **`secondary_bar.md` (Row 2):** Primary toolbelt buttons (Select, Move, Rotate, Ruler, LOS, Place), snapping toggles, grid toggles, and camera speed presets.

## Invariants
- Total combined height is strictly `48px` (`h-12`).
- Row 1 is a fixed `24px` (`h-6`).
- Row 2 takes the remaining `24px` (`h-6`).
