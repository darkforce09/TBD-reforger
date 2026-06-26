// Attributes modal (Ultra Plan §5.2) — opened by double-clicking a unit (Eden paradigm).
// Phase 3.5: editable Transform/Identity/States/Arsenal tabs, replacing the old right-panel
// SlotInspector (the Asset Palette now stays docked). T-068.4: Arsenal tab is now a working
// dumb loadout picker (four gear dropdowns + JSON download); the smart Forge / paper-doll is T-068.10.

import { memo, useMemo, useState } from 'react'
import { Download } from 'lucide-react'
import {
  updateSlot,
  updateSlotPosition,
  useMapStore,
  type MissionDoc,
  type Slot,
} from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { cn } from '@/lib/utils'
import { useRegistry } from '@/hooks/queries'
import type { RegistryItemKind } from '@/types/models/registry'
import {
  buildLoadoutExport,
  downloadLoadoutJson,
  type LoadoutGear,
} from '../loadout/loadoutExport'
import {
  Field,
  NumberField,
  ReadonlyField,
  SelectField,
  TextField,
  ToggleField,
} from './RightInspector/fields'

const STANCE = [
  { value: 'stand', label: 'Standing' },
  { value: 'crouch', label: 'Crouched' },
  { value: 'prone', label: 'Prone' },
]

const TABS = ['Transform', 'Identity', 'States', 'Arsenal'] as const
type Tab = (typeof TABS)[number]

function AttributesModalInner({
  md,
  slotId,
  onOpenChange,
}: {
  md: MissionDoc
  slotId: string | null
  onOpenChange: (open: boolean) => void
}) {
  const slot = useMapStore((s) => (slotId ? s.slotsById[slotId] : undefined))
  const squadName = useMapStore((s) =>
    slot ? (s.squadsById[slot.squadId]?.name ?? '—') : '—',
  )
  const [tab, setTab] = useState<Tab>('Identity')

  return (
    <Dialog open={slotId != null} onOpenChange={onOpenChange}>
      <DialogContent
        title="Attributes"
        description={slot ? `${slot.role || 'Slot'} · slot #${slot.index + 1}` : 'Entity'}
      >
        {slot && (
          <div className="flex flex-col gap-4">
            <div className="flex gap-1 rounded-lg bg-surface-container-lowest/50 p-1">
              {TABS.map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setTab(t)}
                  className={cn(
                    'flex-1 rounded-md px-2 py-1.5 text-label-md transition-colors',
                    tab === t
                      ? 'bg-primary/20 text-primary'
                      : 'text-on-surface-variant hover:bg-white/5',
                  )}
                >
                  {t}
                </button>
              ))}
            </div>

            {tab === 'Transform' && (
              <TransformTab md={md} slot={slot} />
            )}
            {tab === 'Identity' && (
              <IdentityTab md={md} slot={slot} squadName={squadName} />
            )}
            {tab === 'States' && <StatesTab />}
            {tab === 'Arsenal' && <ArsenalTab slot={slot} />}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}

function TransformTab({ md, slot }: { md: MissionDoc; slot: Slot }) {
  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-3 gap-3">
        <NumberField label="X" value={slot.position.x} onCommit={(x) => updateSlotPosition(md, slot.id, { x })} />
        <NumberField label="Y" value={slot.position.y} onCommit={(y) => updateSlotPosition(md, slot.id, { y })} />
        <NumberField label="Z" value={slot.position.z} onCommit={(z) => updateSlotPosition(md, slot.id, { z })} />
      </div>
      <NumberField
        label="Rotation"
        value={slot.position.rotation}
        suffix="°"
        onCommit={(rotation) => updateSlotPosition(md, slot.id, { rotation })}
      />
      <SelectField
        label="Stance"
        value={slot.stance}
        options={STANCE}
        onChange={(stance) => updateSlot(md, slot.id, { stance: stance as Slot['stance'] })}
      />
      <p className="text-label-sm normal-case text-outline">
        Drag on the map or edit coordinates above. Z is manual until terrain elevation (DEM) ships.
      </p>
    </div>
  )
}

function IdentityTab({
  md,
  slot,
  squadName,
}: {
  md: MissionDoc
  slot: Slot
  squadName: string
}) {
  return (
    <div className="flex flex-col gap-4">
      <TextField
        label="Role"
        value={slot.role}
        onChange={(role) => updateSlot(md, slot.id, { role })}
        placeholder="Rifleman"
      />
      <TextField
        label="Tag"
        value={slot.tag ?? ''}
        onChange={(tag) => updateSlot(md, slot.id, { tag })}
        placeholder="MED · ENG · SL…"
      />
      <ReadonlyField label="Squad" value={squadName} />
    </div>
  )
}

function StatesTab() {
  return (
    <div className="flex flex-col gap-3">
      <p className="text-label-sm normal-case text-outline">
        Unit traits — wired to the compiler in a later phase.
      </p>
      <ToggleField label="Medic (soon)" checked={false} onChange={() => {}} />
      <ToggleField label="Engineer (soon)" checked={false} onChange={() => {}} />
    </div>
  )
}

// Dumb loadout picker (T-068.4): four gear dropdowns from the registry + JSON download.
// Character slots only — props/vehicles get an empty state. '' = empty pick → null on export.
const GEAR_ROWS: { key: keyof LoadoutGear; label: string; kind: RegistryItemKind }[] = [
  { key: 'primary', label: 'Primary', kind: 'gear_primary' },
  { key: 'uniform', label: 'Uniform', kind: 'gear_uniform' },
  { key: 'vest', label: 'Vest', kind: 'gear_vest' },
  { key: 'helmet', label: 'Helmet', kind: 'gear_helmet' },
]

function ArsenalTab({ slot }: { slot: Slot }) {
  const { data } = useRegistry()
  const [gear, setGear] = useState<Record<keyof LoadoutGear, string>>({
    primary: '',
    uniform: '',
    vest: '',
    helmet: '',
  })

  const isCharacter = useMemo(() => {
    if (!data || !slot.assetId) return false
    return data.data.some((i) => i.resource_name === slot.assetId && i.kind === 'character')
  }, [data, slot.assetId])

  const optionsByKind = useMemo(() => {
    const map = {} as Record<RegistryItemKind, { value: string; label: string }[]>
    for (const row of GEAR_ROWS) {
      const items = (data?.data ?? []).filter((i) => i.kind === row.kind)
      map[row.kind] = [
        { value: '', label: '— None —' },
        ...items.map((i) => ({ value: i.resource_name, label: i.display_name })),
      ]
    }
    return map
  }, [data])

  if (!isCharacter) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-label-sm normal-case text-outline">
          Loadout applies to placed characters.
        </p>
      </div>
    )
  }

  const onDownload = () => {
    const norm = (v: string): string | null => (v === '' ? null : v)
    const payload = buildLoadoutExport(
      {
        primary: norm(gear.primary),
        uniform: norm(gear.uniform),
        vest: norm(gear.vest),
        helmet: norm(gear.helmet),
      },
      data?.modpack_id ?? '',
    )
    downloadLoadoutJson(payload)
  }

  return (
    <div className="flex flex-col gap-4">
      {GEAR_ROWS.map((row) => (
        <SelectField
          key={row.key}
          label={row.label}
          value={gear[row.key]}
          options={optionsByKind[row.kind]}
          onChange={(v) => setGear((g) => ({ ...g, [row.key]: v }))}
        />
      ))}
      <Field label="Export">
        <button
          type="button"
          onClick={onDownload}
          className="inline-flex items-center justify-center gap-2 rounded-lg border border-primary/40 bg-primary/15 px-3 py-2 text-label-md text-primary transition-colors hover:bg-primary/25"
        >
          <Download className="size-4" />
          Download loadout JSON
        </button>
      </Field>
    </div>
  )
}

// Memoized (T-057) with a stabilized onOpenChange so unrelated page renders don't re-render it.
export const AttributesModal = memo(AttributesModalInner)
