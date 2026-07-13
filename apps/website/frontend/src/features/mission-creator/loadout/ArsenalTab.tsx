// Attributes → Arsenal tab (T-068.4; smart Forge T-068.10; paper-doll T-068.10.7).
// The ACE layout: LEFT the active region's item list (click to equip), CENTER the clickable
// SVG soldier (every loadout part is a hotspot, incl. the optic/magazine on the rifle),
// RIGHT the contextual column (attachment quick-lists / container / item detail). Thin
// Slot-doc adapter: migrates v1 on read, persists v2 picks via updateSlotLoadout (one undo
// step per pick), validates against the compat worker, downloads the v2 loadout-export.
// Character slots only; worker-down degrades to kind lists (edge regions explain themselves).

import { useMemo, useState } from 'react'
import { Download, Loader2 } from 'lucide-react'
import { toast } from 'sonner'
import { updateSlotLoadout, type MissionDoc, type Slot } from '@/features/tactical-map'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
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
import { RAIL_REGIONS, formatLoadoutWeight, loadoutWeight } from './arsenalDollModel'
import { buildLoadoutExport, downloadLoadoutJson } from './loadoutExport'
import { itemDetail } from './itemDetail'
import { ContainerPanel, ItemDetailPane } from './ItemDetailPane'
import { migrateLoadout } from './migrateLoadout'
import { SlotItemList } from './SlotItemList'
import { SlotRail } from './SlotRail'
import { SoldierModel3D } from './SoldierModel3D'
import { SoldierSilhouette } from './SoldierSilhouette'
import { useArsenalValidation } from './useArsenalValidation'

/** Right-column quick-list: the compat feed for one rifle attachment, click to equip
 *  (the ACE behavior — swapping the optic without leaving the weapon view). */
function AttachmentQuickList({
  title,
  feed,
  current,
  catalogByName,
  onEquip,
}: {
  title: string
  feed: readonly string[]
  current: string
  catalogByName: ReadonlyMap<string, RegistryItem>
  onEquip: (value: string) => void
}) {
  const values = useMemo(() => {
    const visible = feed.filter((v) => {
      const item = catalogByName.get(v)
      // Same picker semantics as rowValues: templates/variants hide, a live pick never blanks.
      if (item?.abstract === true || item?.variant_of) return v === current
      return true
    })
    const name = (v: string) => catalogByName.get(v)?.display_name ?? v
    return visible.sort((a, b) => name(a).localeCompare(name(b)))
  }, [feed, current, catalogByName])

  return (
    <div className="flex flex-col gap-1">
      <p className="text-label-sm uppercase tracking-wide text-outline">{title}</p>
      {values.length === 0 && (
        <p className="text-label-sm normal-case text-outline">Nothing compatible.</p>
      )}
      {current && (
        <button
          type="button"
          onClick={() => onEquip('')}
          className="rounded px-1.5 py-1 text-left text-label-sm normal-case text-on-surface-variant transition-colors hover:bg-white/5"
        >
          — None —
        </button>
      )}
      {values.map((v) => (
        <button
          key={v}
          type="button"
          onClick={() => onEquip(v)}
          className={cn(
            'truncate rounded px-1.5 py-1 text-left text-label-sm normal-case transition-colors',
            v === current
              ? 'bg-primary/20 text-primary'
              : 'text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
          )}
        >
          {catalogByName.get(v)?.display_name ?? v}
        </button>
      ))}
    </div>
  )
}

/** Right column: rifle active → attachment quick-lists; container equipped → capacity
 *  panel; otherwise the equipped item's detail (variant links hop via onInspect). */
function ContextColumn({
  quickLists,
  picks,
  sets,
  detail,
  catalogByName,
  onPick,
  onInspect,
}: {
  quickLists: boolean
  picks: Record<LoadoutKey, string>
  sets: CompatSets
  detail: ReturnType<typeof itemDetail>
  catalogByName: ReadonlyMap<string, RegistryItem>
  onPick: (key: LoadoutKey, value: string) => void
  onInspect: (resourceName: string) => void
}) {
  if (quickLists) {
    return (
      <div className="flex h-full flex-col gap-4 rounded-lg border border-outline-variant/20 bg-surface-container-lowest/30 p-3">
        <h4 className="text-title-sm text-on-surface">
          {catalogByName.get(picks.primary)?.display_name ?? 'Primary'}
        </h4>
        <AttachmentQuickList
          title="Optic"
          feed={sets.edgeItems.optic ?? []}
          current={picks.optic}
          catalogByName={catalogByName}
          onEquip={(v) => onPick('optic', v)}
        />
        <AttachmentQuickList
          title="Magazine"
          feed={sets.edgeItems.magazine ?? []}
          current={picks.magazine}
          catalogByName={catalogByName}
          onEquip={(v) => onPick('magazine', v)}
        />
      </div>
    )
  }
  if (detail?.isContainer) return <ContainerPanel detail={detail} />
  return <ItemDetailPane detail={detail} onInspect={onInspect} />
}

/** One-line readout under the doll — the doll itself carries no text (T-068.10.8). */
function DollCaption({
  activeKey,
  picks,
  catalogByName,
}: {
  activeKey: LoadoutKey
  picks: Record<LoadoutKey, string>
  catalogByName: ReadonlyMap<string, RegistryItem>
}) {
  const label = RAIL_REGIONS.find((r) => r.key === activeKey)?.label ?? activeKey
  const rn = picks[activeKey]
  return (
    <p className="pt-1 text-center text-label-sm normal-case text-on-surface-variant">
      <span className="text-outline">{label} — </span>
      {rn ? (catalogByName.get(rn)?.display_name ?? rn) : 'empty'}
    </p>
  )
}

function ValidationBadge({ valid, errorCount }: { valid: boolean; errorCount: number }) {
  if (valid) return <Badge variant="success">Loadout valid</Badge>
  return (
    <Badge variant="error">
      {errorCount} incompatible pick{errorCount === 1 ? '' : 's'}
    </Badge>
  )
}

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

  // The doll selection: which region the left list + right column look at.
  const [activeKey, setActiveKey] = useState<LoadoutKey>('primary')
  // Variant-link hops inside the detail pane override the derived detail target.
  const [inspected, setInspected] = useState<string>('')
  // T-154: wgpu doll is primary; engine-create failure falls back to the SVG silhouette.
  const [dollUnavailable, setDollUnavailable] = useState(false)

  const { status, sets } = useArsenalValidation(isCharacter, data?.modpack_id, picks)
  const validation = useMemo(() => validateLoadout(picks, sets), [picks, sets])
  const weight = useMemo(() => loadoutWeight(picks, catalogByName), [picks, catalogByName])

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
  const effectiveSets = smart ? sets : degradeSets
  const onSelectRegion = (key: LoadoutKey) => {
    setActiveKey(key)
    setInspected('')
  }
  const onPick = (key: LoadoutKey, value: string) => {
    setInspected('') // show the freshly equipped item, not a stale variant hop
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

  // Right column: rifle active → attachment quick-lists; container equipped → capacity
  // panel; otherwise the equipped item's detail (variant links hop via `inspected`).
  const quickLists = smart && activeKey === 'primary' && Boolean(picks.primary)
  const detailRn = inspected || picks[activeKey]
  const detail = detailRn ? itemDetail(detailRn, catalog, catalogByName) : null
  const errorCount = Object.keys(validation.errors).length

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        {smart ? (
          <Badge variant="success">Compat active</Badge>
        ) : (
          <Badge variant="warning">Compat unavailable — full catalog</Badge>
        )}
        {/* Honest weight: known sum + how many equipped items ship no weight data. */}
        <span className="font-mono text-label-sm tabular-nums text-on-surface-variant">
          {formatLoadoutWeight(weight)}
        </span>
      </div>

      {/* ACE paper-doll (T-068.10.8): slot rail | list | soldier | context. */}
      <div className="grid min-h-0 flex-1 grid-cols-[44px_260px_1fr_240px] gap-3">
        <SlotRail
          picks={picks}
          activeKey={activeKey}
          onSelect={onSelectRegion}
          catalogByName={catalogByName}
        />

        <div className="min-h-0 rounded-lg border border-outline-variant/20 bg-surface-container-lowest/30 p-2">
          <SlotItemList
            key={activeKey}
            activeKey={activeKey}
            picks={picks}
            onPick={onPick}
            catalog={catalog}
            catalogByName={catalogByName}
            sets={effectiveSets}
            smart={smart}
          />
        </div>

        <div className="flex min-h-0 flex-col">
          <div className="min-h-0 flex-1">
            {dollUnavailable ? (
              <SoldierSilhouette
                picks={picks}
                activeKey={activeKey}
                onSelect={onSelectRegion}
                catalogByName={catalogByName}
              />
            ) : (
              <SoldierModel3D
                picks={picks}
                activeKey={activeKey}
                onSelect={onSelectRegion}
                onUnavailable={() => setDollUnavailable(true)}
              />
            )}
          </div>
          <DollCaption activeKey={activeKey} picks={picks} catalogByName={catalogByName} />
        </div>

        <div className="min-h-0 overflow-y-auto">
          <ContextColumn
            quickLists={quickLists}
            picks={picks}
            sets={sets}
            detail={detail}
            catalogByName={catalogByName}
            onPick={onPick}
            onInspect={setInspected}
          />
        </div>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2">
        {smart ? <ValidationBadge valid={validation.valid} errorCount={errorCount} /> : <span />}
        <button
          type="button"
          onClick={onDownload}
          disabled={smart && !validation.valid}
          className="inline-flex items-center justify-center gap-2 rounded-lg border border-primary/40 bg-primary/15 px-3 py-2 text-label-md text-primary transition-colors hover:bg-primary/25 disabled:cursor-not-allowed disabled:border-outline-variant/30 disabled:bg-surface-container-lowest/40 disabled:text-outline"
        >
          <Download className="size-4" />
          Download loadout JSON
        </button>
      </div>

      {pendingKindsWithData.length > 0 && (
        <p className="text-label-sm normal-case text-outline">
          Equipment items (binoculars, radios, medical, glasses) get their rows with the
          equipment/cargo slices — the registry already classifies them.
        </p>
      )}
    </div>
  )
}
