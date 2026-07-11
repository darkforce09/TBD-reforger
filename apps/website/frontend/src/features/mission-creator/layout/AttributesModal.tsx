// Attributes modal (Ultra Plan §5.2) — opened by double-clicking a unit (Eden paradigm).
// Phase 3.5: editable Transform/Identity/States/Arsenal tabs, replacing the old right-panel
// SlotInspector (the Asset Palette now stays docked). T-068.10: the Arsenal tab is the smart
// Forge (compat-validated pickers persisted on the slot) in ../loadout/ArsenalTab.

import { memo, useState } from 'react'
import {
  updateSlot,
  updateSlotPosition,
  useMapStore,
  type MissionDoc,
  type Slot,
} from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { cn } from '@/lib/utils'
import { ArsenalTab } from '../loadout/ArsenalTab'
import {
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
  const squadName = useMapStore((s) => (slot ? (s.squadsById[slot.squadId]?.name ?? '—') : '—'))
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

            {tab === 'Transform' && <TransformTab md={md} slot={slot} />}
            {tab === 'Identity' && <IdentityTab md={md} slot={slot} squadName={squadName} />}
            {tab === 'States' && <StatesTab />}
            {tab === 'Arsenal' && <ArsenalTab md={md} slot={slot} />}
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
        <NumberField
          label="X"
          value={slot.position.x}
          onCommit={(x) => updateSlotPosition(md, slot.id, { x })}
        />
        <NumberField
          label="Y"
          value={slot.position.y}
          onCommit={(y) => updateSlotPosition(md, slot.id, { y })}
        />
        <NumberField
          label="Z"
          value={slot.position.z}
          onCommit={(z) => updateSlotPosition(md, slot.id, { z })}
        />
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

function IdentityTab({ md, slot, squadName }: { md: MissionDoc; slot: Slot; squadName: string }) {
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
      <ToggleField
        label="Medic (soon)"
        checked={false}
        onChange={() => {
          /* noop: trait is wired to the compiler in a later phase */
        }}
      />
      <ToggleField
        label="Engineer (soon)"
        checked={false}
        onChange={() => {
          /* noop: trait is wired to the compiler in a later phase */
        }}
      />
    </div>
  )
}

// Memoized (T-057) with a stabilized onOpenChange so unrelated page renders don't re-render it.
export const AttributesModal = memo(AttributesModalInner)
