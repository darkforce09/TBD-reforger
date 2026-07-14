// T-152.3 — TS oracle for landmark building glyph policy (compose lives in Rust).

/** Normative building classes → `building-{class}` center glyphs. */
export const BUILDING_CLASSES = [
  'residential',
  'civic',
  'agricultural',
  'industrial',
  'commercial',
  'hangar',
  'bunker',
  'tower',
  'military',
  'bridge',
  'castle',
  'lighthouse',
  'shed',
  'container',
  'tent',
  'ruin',
  'garage',
  'generic',
] as const

export type BuildingClass = (typeof BUILDING_CLASSES)[number]

const BUILDING_CLASS_SET = new Set<string>(BUILDING_CLASSES)

/** `building-{class}` footprint glyph key. */
export function buildingIconKey(buildingClass: string): string | null {
  return BUILDING_CLASS_SET.has(buildingClass) ? `building-${buildingClass}` : null
}

/** `building-badge-*` overlay for military / tower / bunker. */
export function badgeIconKey(buildingClass: string): string | null {
  switch (buildingClass) {
    case 'military':
      return 'building-badge-military'
    case 'tower':
      return 'building-badge-tower'
    case 'bunker':
      return 'building-badge-bunker'
    default:
      return null
  }
}

/** Badge overlay wins for military/tower/bunker; else footprint glyph. */
export function landmarkGlyphIconKey(buildingClass: string): string | null {
  return badgeIconKey(buildingClass) ?? buildingIconKey(buildingClass)
}
