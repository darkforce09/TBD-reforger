// Attributes modal (Ultra Plan §5.2) — opened by double-clicking a unit (Eden paradigm).
// Phase 3.5: editable Transform/Identity/States/Arsenal tabs, replacing the old right-panel
// SlotInspector (the Asset Palette now stays docked). Position editing (Transform X/Y) and
// the real Arsenal land in later phases (7b / 6); this ships role/tag/stance editing + stubs.

import { useState } from 'react'
import { Shield } from 'lucide-react'
import { updateSlot, useMapStore, type MissionDoc, type Slot } from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { cn } from '@/lib/utils'
import {
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

export function AttributesModal({
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
            {tab === 'Arsenal' && <ArsenalTab />}
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
        <ReadonlyField label="X" value={Math.round(slot.position.x).toString()} />
        <ReadonlyField label="Y" value={Math.round(slot.position.y).toString()} />
        <ReadonlyField label="Z" value={Math.round(slot.position.z).toString()} />
      </div>
      <ReadonlyField label="Rotation" value={`${Math.round(slot.position.rotation)}°`} />
      <SelectField
        label="Stance"
        value={slot.stance}
        options={STANCE}
        onChange={(stance) => updateSlot(md, slot.id, { stance: stance as Slot['stance'] })}
      />
      <p className="text-label-sm normal-case text-outline">
        Drag the unit on the map to reposition it (coming in a later phase). Z is derived
        from terrain elevation once the DEM lands.
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

function ArsenalTab() {
  return (
    <div className="flex flex-col gap-3">
      <p className="text-label-sm normal-case text-outline">
        The visual Loadout Forge (paper-doll + registry) arrives with Phase 6.
      </p>
      <button
        type="button"
        disabled
        title="The Arsenal lands in a later phase"
        className="inline-flex items-center justify-center gap-2 rounded-lg border border-outline-variant/30 bg-surface-container-high/40 px-3 py-2 text-label-md text-outline"
      >
        <Shield className="size-4" />
        Open Loadout Forge (soon)
      </button>
    </div>
  )
}
