// T-152.20 — Mission Settings "World layers" completeness manifest (audit A15 / operator gate O10).
// The `satisfies Record<keyof WorldClassToggles, string>` guard makes this the keyof-driven single
// source of truth for the section: adding a 13th class to WorldClassToggles without a label here
// fails tsc. MissionSettingsDialog renders the 7 world/map toggles (roads…sea) from these labels;
// the completeness test (worldLayerFields.test.ts) asserts the manifest covers every key (12/12) so
// O10 ("each pref off works") is executable for all classes, not just the 5 that were exposed.
import type { WorldClassToggles } from '@/features/tactical-map'

/** Display label for every WorldClassToggles key exposed in the Mission Settings World layers
 *  section. Order here is declaration order; the dialog decides row order (7 world/map toggles
 *  before the 5 label/overlay toggles). */
export const WORLD_LAYER_TOGGLE_LABELS = {
  roads: 'Roads',
  buildings: 'Buildings',
  forest: 'Forest mass',
  trees: 'Trees',
  props: 'Props',
  contours: 'Contours',
  sea: 'Sea',
  fences: 'Fences',
  airfield: 'Airfield',
  heights: 'Height labels',
  townLabels: 'Town labels',
  roadNames: 'Road names',
} satisfies Record<keyof WorldClassToggles, string>
