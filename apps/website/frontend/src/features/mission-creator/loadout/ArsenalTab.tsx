// Attributes → Arsenal tab (T-068.4; smart Forge T-068.10). A dumb render loop over
// LOADOUT_ROWS: options + validation come from arsenalRules (pure) fed by useArsenalValidation
// (compat worker bridge); picks live on the slot itself (`Slot.loadout` via updateSlotLoadout —
// one undo step per pick, persisted through Save Version / Export / IDB / copy-paste).
// Character slots only; compat-unavailable degrades to the Phase 1 full-catalog pickers.

import { useMemo } from 'react'
import { Download, Loader2 } from 'lucide-react'
import { toast } from 'sonner'
import { updateSlotLoadout, type MissionDoc, type Slot } from '@/features/tactical-map'
import { Badge } from '@/components/ui/badge'
import { useRegistry } from '@/hooks/queries'
import type { RegistryItem } from '@/types/models/registry'
import {
  LOADOUT_ROWS,
  buildRowOptions,
  loadoutToPicks,
  picksToLoadout,
  validateLoadout,
  type CompatSets,
  type LoadoutKey,
} from './arsenalRules'
import { buildLoadoutExport, downloadLoadoutJson, slotLoadoutToGear } from './loadoutExport'
import { useArsenalValidation } from './useArsenalValidation'
import { Field, SelectField } from '../layout/RightInspector/fields'

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

  const picks = useMemo(() => loadoutToPicks(slot.loadout), [slot.loadout])
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
  const onPick = (key: LoadoutKey, value: string) => {
    updateSlotLoadout(md, slot.id, picksToLoadout({ ...picks, [key]: value }, catalogByName))
  }
  const onDownload = () => {
    if (smart && !validation.valid) {
      toast.error('Loadout has incompatible picks — fix the flagged slots first.')
      return
    }
    downloadLoadoutJson(buildLoadoutExport(slotLoadoutToGear(slot.loadout), data?.modpack_id ?? ''))
  }

  // Degrade (documented in the T-068.10 verify log): worker down → Phase 1 pickers, no edge
  // rows. Clothing rows are always the full catalog — compat constrains weapon families only.
  const rows = smart ? LOADOUT_ROWS : LOADOUT_ROWS.filter((r) => r.source.type === 'kind')
  const degradeSets: CompatSets = { edgeItems: {} }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-2">
        {smart ? (
          <Badge variant="success">Compat active</Badge>
        ) : (
          <Badge variant="warning">Compat unavailable — full catalog</Badge>
        )}
      </div>

      {rows.map((row) => {
        const disabledDep =
          smart && row.source.type === 'edge' && !picks[row.source.dependsOn]
            ? row.source.dependsOn
            : null
        const error = smart ? validation.errors[row.key] : undefined
        return (
          <div key={row.key} className="flex flex-col gap-1">
            {disabledDep ? (
              <Field label={row.label}>
                <div className="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 text-label-md text-outline">
                  Pick a {disabledDep} first
                </div>
              </Field>
            ) : (
              <SelectField
                label={row.label}
                value={picks[row.key]}
                options={buildRowOptions(
                  row,
                  picks[row.key],
                  smart ? sets : degradeSets,
                  catalog,
                  catalogByName,
                )}
                onChange={(v) => onPick(row.key, v)}
              />
            )}
            {error && <Badge variant="error">{error}</Badge>}
          </div>
        )
      })}

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
