// Attributes → Arsenal tab (T-068.4; smart Forge T-068.10; picks panel extracted T-153).
// Thin Slot-doc adapter over ArsenalPicksPanel: migrates v1 docs on read, persists v2 picks
// via updateSlotLoadout (one undo step per pick), validates against the compat worker, and
// downloads the v2 loadout-export (with the derived legacy gear block). Character slots
// only; worker-down degrades to kind-only rows.

import { useMemo, useState } from 'react'
import { Download, Loader2 } from 'lucide-react'
import { toast } from 'sonner'
import { updateSlotLoadout, type MissionDoc, type Slot } from '@/features/tactical-map'
import { Badge } from '@/components/ui/badge'
import { useRegistry } from '@/hooks/queries'
import type { RegistryItem } from '@/types/models/registry'
import {
  PENDING_V2_KINDS,
  loadoutToPicks,
  picksToLoadout,
  validateLoadout,
  type CompatSets,
  type LoadoutKey,
} from './arsenalRules'
import { ArsenalPicksPanel } from './ArsenalPicksPanel'
import { buildLoadoutExport, downloadLoadoutJson } from './loadoutExport'
import { itemDetail } from './itemDetail'
import { ContainerPanel, ItemDetailPane } from './ItemDetailPane'
import { migrateLoadout } from './migrateLoadout'
import { useArsenalValidation } from './useArsenalValidation'
import { Field } from '../layout/RightInspector/fields'

export function ArsenalTab({ md, slot }: { md: MissionDoc; slot: Slot }) {
  const { data } = useRegistry()
  const catalog = useMemo(() => data?.data ?? [], [data])
  const catalogByName = useMemo(
    () => new Map<string, RegistryItem>(catalog.map((i) => [i.resource_name, i])),
    [catalog],
  )

  const isCharacter = useMemo(
    () => Boolean(slot.assetId && catalogByName.get(slot.assetId)?.kind === 'character'),
    [catalogByName, slot.assetId],
  )

  // v1 docs migrate on read (AreaType-aware vest routing); the editor writes v2 only.
  const loadoutV2 = useMemo(
    () => migrateLoadout(slot.loadout, catalogByName),
    [slot.loadout, catalogByName],
  )
  const picks = useMemo(() => loadoutToPicks(loadoutV2), [loadoutV2])
  // T-068.10.6 inspection: last-picked item, seeded from the first non-empty pick.
  const [inspected, setInspected] = useState<string>('')
  const inspectedRn = inspected || Object.values(picks).find(Boolean) || ''
  const detail = useMemo(
    () => (inspectedRn ? itemDetail(inspectedRn, catalog, catalogByName) : null),
    [inspectedRn, catalog, catalogByName],
  )
  const { status, sets } = useArsenalValidation(isCharacter, data?.modpack_id, picks)
  const validation = useMemo(() => validateLoadout(picks, sets), [picks, sets])

  if (!isCharacter) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-label-sm normal-case text-outline">
          Loadout applies to placed characters.
        </p>
      </div>
    )
  }

  if (status === 'loading') {
    return (
      <div className="flex items-center gap-2 py-6 text-label-sm text-outline">
        <Loader2 className="size-3.5 animate-spin" />
        Loading compatibility…
      </div>
    )
  }

  const smart = status === 'ready'
  const degradeSets: CompatSets = { edgeItems: {} }
  const onPick = (key: LoadoutKey, value: string) => {
    if (value) setInspected(value) // ACE model: picking an item shows it in the detail pane
    updateSlotLoadout(md, slot.id, picksToLoadout({ ...picks, [key]: value }, catalogByName))
  }
  const onDownload = () => {
    if (smart && !validation.valid) {
      toast.error('Loadout has incompatible picks — fix the flagged slots first.')
      return
    }
    downloadLoadoutJson(buildLoadoutExport(loadoutV2, data?.modpack_id ?? ''))
  }

  const pendingKindsWithData = PENDING_V2_KINDS.filter((k) =>
    catalog.some((i) => i.kind === k && i.abstract !== true),
  )

  const showContainer = detail?.isContainer === true

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        {smart ? (
          <Badge variant="success">Compat active</Badge>
        ) : (
          <Badge variant="warning">Compat unavailable — full catalog</Badge>
        )}
      </div>

      {/* ACE panes (T-068.10.6): detail left, pickers center, container right. */}
      <div
        className={
          showContainer
            ? 'grid max-h-[58vh] grid-cols-[230px_1fr_210px] gap-3'
            : 'grid max-h-[58vh] grid-cols-[230px_1fr] gap-3'
        }
      >
        <ItemDetailPane detail={detail} onInspect={setInspected} />
        <div className="min-h-0 overflow-y-auto pr-1">
          <ArsenalPicksPanel
            picks={picks}
            onPick={onPick}
            catalog={catalog}
            catalogByName={catalogByName}
            sets={smart ? sets : degradeSets}
            smart={smart}
          />
        </div>
        {showContainer && detail && <ContainerPanel detail={detail} />}
      </div>

      {pendingKindsWithData.length > 0 && (
        <p className="text-label-sm normal-case text-outline">
          Equipment items (binoculars, radios, medical, glasses) get their rows with the
          equipment/cargo slices — the registry already classifies them.
        </p>
      )}

      <Field label="Export">
        <button
          type="button"
          onClick={onDownload}
          disabled={smart && !validation.valid}
          className="inline-flex items-center justify-center gap-2 rounded-lg border border-primary/40 bg-primary/15 px-3 py-2 text-label-md text-primary transition-colors hover:bg-primary/25 disabled:cursor-not-allowed disabled:border-outline-variant/30 disabled:bg-surface-container-lowest/40 disabled:text-outline"
        >
          <Download className="size-4" />
          Download loadout JSON
        </button>
      </Field>
    </div>
  )
}
