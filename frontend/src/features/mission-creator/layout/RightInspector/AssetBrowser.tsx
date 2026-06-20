// Right-panel default (Ultra Plan §5.2): the Asset Browser as a nested, collapsible
// Eden-style tree (Faction → Category → Class), NOT flat pills. Visual shell only:
// mock catalog + local selection; drag-leaf-onto-map and the registry-backed feed
// arrive in a later phase.

import { useState } from 'react'
import { TreeView } from '../tree/TreeView'
import { ASSET_CATALOG } from './assetCatalogMock'

export function AssetBrowser() {
  const [selectedId, setSelectedId] = useState<string | null>(null)

  return (
    <div className="flex flex-col gap-2">
      <header>
        <h2 className="text-headline-sm text-on-surface">Asset Browser</h2>
        <p className="text-label-sm normal-case text-outline">
          Drag an asset onto the map to place it.
        </p>
      </header>
      <TreeView nodes={ASSET_CATALOG} selectedId={selectedId} onSelect={setSelectedId} />
    </div>
  )
}
