// Shared frosted-glass recipe for the editor's floating overlay panels. More
// transparent + stronger blur than the global `.glass` token (0.7 alpha) so the
// Deck.gl map is clearly visible panning underneath — the "macOS Tactical" look.
// Aegis tokens only (no slate/blue); bg-surface-container-lowest/55 mirrors the
// translucent pattern already used in components/ui/split-pane.tsx.

export const overlayPanel =
  'pointer-events-auto rounded-xl border border-white/10 ' +
  'bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl'

// Flush-docked variant for the Eden shell's edge panels (Phase 3.5): same glass, but
// squared off against the viewport edge (no outer rounding) with a single inner edge
// border, so the left/right sidebars dock flush and the map sits between them.
export const overlayDocked =
  'pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl'
