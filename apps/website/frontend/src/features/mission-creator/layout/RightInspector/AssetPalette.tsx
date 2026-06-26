// Right-panel Asset Palette (Ultra Plan §5.2; locked in ux_spec.md) —
// docked flush right, ALWAYS visible (it never swaps to an inspector; properties edit via
// the double-click Attributes modal instead). A tab strip switches the catalog the user
// drags from. Phase 3.5: Factions = the live Asset Browser tree; the other tabs are stubs
// until the registry feed (GET /api/v1/registry) and the markers/objectives tools land.

import { memo, useState } from 'react'
import { cn } from '@/lib/utils'
import { overlayDocked } from '../overlay'
import { AssetBrowser } from './AssetBrowser'

const TABS = ['Factions', 'Vehicles', 'Markers', 'Objectives'] as const
type Tab = (typeof TABS)[number]

function AssetPaletteInner() {
  const [tab, setTab] = useState<Tab>('Factions')

  return (
    <div className={cn(overlayDocked, 'flex h-full w-full flex-col overflow-hidden border-l border-white/10')}>
      <div className="custom-scrollbar flex shrink-0 gap-0.5 overflow-x-auto border-b border-white/10 p-2">
        {TABS.map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={cn(
              'shrink-0 whitespace-nowrap rounded-md px-2 py-1.5 text-label-sm uppercase tracking-wide transition-colors',
              tab === t
                ? 'bg-primary/15 text-primary'
                : 'text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
            )}
          >
            {t}
          </button>
        ))}
      </div>

      <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-3">
        {tab === 'Factions' ? (
          <AssetBrowser />
        ) : (
          <p className="px-2 py-6 text-center text-label-sm normal-case break-words text-outline">
            {tab} catalog arrives with the asset registry feed.
          </p>
        )}
      </div>
    </div>
  )
}

export const AssetPalette = memo(AssetPaletteInner)
