// T-090.5.2 — World-glyph atlas, loaded once per tab (glyphs spec deliverable 4). The build
// step (`make map-glyphs-build`) bakes packages/map-assets/glyphs/svg/* into one webp texture
// + a Deck-ready IconLayer mapping; this module fetches the mapping and hands both to icon
// layer builders. Missing/broken atlas degrades per plan risk R5: warn once, return null,
// icon layers simply don't mount — never a crash, mass layers unaffected.

export interface WorldGlyphRect {
  x: number
  y: number
  width: number
  height: number
  anchorX: number
  anchorY: number
  /** Deck IconLayer mask flag: true = tintable via getColor (manifest `tintable`). */
  mask: boolean
}

export interface WorldGlyphAtlas {
  atlasUrl: string
  iconMapping: Record<string, WorldGlyphRect>
}

const ATLAS_IMAGE_URL = '/map-assets/glyphs/atlas/world-glyphs.webp'
const ATLAS_MAPPING_URL = '/map-assets/glyphs/atlas/world-glyphs.json'

let cached: WorldGlyphAtlas | null = null
let pending: Promise<WorldGlyphAtlas | null> | null = null
let warned = false

const warnOnce = (msg: string) => {
  if (!warned) {
    warned = true
    console.warn(`[worldmap] glyph atlas unavailable — badges/glyph layers off (${msg})`)
  }
}

/** Kick (or join) the one atlas mapping fetch. Resolves null on any failure. */
export function loadWorldGlyphAtlas(): Promise<WorldGlyphAtlas | null> {
  if (cached) return Promise.resolve(cached)
  pending ??= (async () => {
    try {
      const res = await fetch(ATLAS_MAPPING_URL)
      const type = res.headers.get('content-type') ?? ''
      if (!res.ok || type.includes('text/html')) {
        warnOnce(`GET ${ATLAS_MAPPING_URL} → ${res.status}`)
        return null
      }
      const mapping = (await res.json()) as { icons?: Record<string, WorldGlyphRect> }
      if (!mapping.icons || Object.keys(mapping.icons).length === 0) {
        warnOnce('mapping has no icons — run make map-glyphs-build')
        return null
      }
      cached = { atlasUrl: ATLAS_IMAGE_URL, iconMapping: mapping.icons }
      return cached
    } catch (e) {
      warnOnce(e instanceof Error ? e.message : String(e))
      return null
    } finally {
      pending = null
    }
  })()
  return pending
}

/** Synchronous view for layer assembly: the atlas once loaded, else null. */
export function getWorldGlyphAtlas(): WorldGlyphAtlas | null {
  return cached
}
