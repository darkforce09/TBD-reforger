// T-152.20 — the Mission Settings "World layers" section must expose a control for every
// WorldClassToggles key (audit A15 / operator gate O10). Two lines of defence, both future-proof:
//   1. compile-time — `satisfies Record<keyof WorldClassToggles, string>` in worldLayerFields.ts
//      fails tsc (npm run build) if a 13th class is added to the type without a label;
//   2. runtime (below) — the manifest key set must match the live pref schema (getClassToggles()),
//      so a class added to WorldClassToggles + DEFAULT_TOGGLES but not exposed here fails the test.
// getClassToggles() is the keyof-driven runtime enumerator: DEFAULT_TOGGLES is byte-locked to the
// interface (defaults test), so its keys are exactly `keyof WorldClassToggles`.
import { describe, it, expect } from 'vitest'
import { WORLD_LAYER_TOGGLE_LABELS } from './worldLayerFields'
import { getClassToggles } from '@/features/tactical-map/state/worldLayerPrefs'

describe('MissionSettingsDialog world-layer completeness (T-152.20 / A15 / O10)', () => {
  it('exposes a control for every WorldClassToggles key (12/12)', () => {
    const manifestKeys = Object.keys(WORLD_LAYER_TOGGLE_LABELS).sort()
    expect(manifestKeys).toHaveLength(12)
    // The manifest must match the live pref schema exactly — no missing key, no stray key.
    expect(manifestKeys).toEqual(Object.keys(getClassToggles()).sort())
  })

  it('every world-layer control has a non-empty label', () => {
    for (const label of Object.values(WORLD_LAYER_TOGGLE_LABELS)) {
      expect(label.trim().length).toBeGreaterThan(0)
    }
  })
})
