// Lazy registration for the T-145 Phase 3.0.d doc-core spike harness. Keeping the React.lazy
// boundary inside the feature module lets Vite code-split the deck-heavy page + wasm doc core so it
// never loads for normal sessions (it is reachable only at /_spike/doc-core, not in the nav).

import { lazy } from 'react'

export const DocCoreSpikePage = lazy(() => import('./DocCoreSpikePage'))

// T-151 wgpu render-engine spike — same code-split rationale; reachable only at
// /_spike/wgpu (the wgpu wasm pkg never loads for normal sessions).
export const WgpuSpikePage = lazy(() => import('./wgpu/WgpuCanvas'))
