// Attributes modal (Ultra Plan §5.2) — STUB. In Eden, double-clicking a unit opens its
// attributes (position, skills like medic/engineer, and the full Arsenal launcher). This
// phase ships the frosted dialog shell + read-only identity; the editable sections + the
// Arsenal land in later phases.

import { useMapStore } from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'

const SECTIONS = [
  { title: 'Position & Stance', hint: 'Fine X/Y/Z, rotation, stance' },
  { title: 'Skills', hint: 'Medic, Engineer, rank, accuracy' },
  { title: 'Arsenal', hint: 'Full loadout editor (paper-doll + registry)' },
]

export function AttributesModal({
  slotId,
  onOpenChange,
}: {
  slotId: string | null
  onOpenChange: (open: boolean) => void
}) {
  const slot = useMapStore((s) => (slotId ? s.slotsById[slotId] : undefined))

  return (
    <Dialog open={slotId != null} onOpenChange={onOpenChange}>
      <DialogContent
        title="Attributes"
        description={slot ? `${slot.role} · slot #${slot.index + 1}` : 'Entity'}
      >
        <div className="flex flex-col gap-3">
          {SECTIONS.map((s) => (
            <div
              key={s.title}
              className="rounded-lg border border-dashed border-outline-variant/30 px-3 py-3"
            >
              <div className="flex items-center justify-between">
                <span className="text-label-md font-semibold text-on-surface">{s.title}</span>
                <span className="text-label-sm uppercase tracking-wider text-outline">Soon</span>
              </div>
              <p className="mt-1 text-label-sm normal-case text-outline">{s.hint}</p>
            </div>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  )
}
