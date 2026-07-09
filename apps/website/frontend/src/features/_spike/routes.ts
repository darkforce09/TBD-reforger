// Lazy registration for spike harnesses. Vite code-splits so they never load for normal sessions.

import { lazy } from 'react'

// T-151 wgpu render-engine spike — reachable only at /_spike/wgpu.
export const WgpuSpikePage = lazy(() => import('./wgpu/WgpuCanvas'))
