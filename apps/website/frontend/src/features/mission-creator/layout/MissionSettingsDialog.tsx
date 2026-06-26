// Mission Settings modal (Ultra Plan §5.4): global environment + terrain, moved out
// of the right panel into a dedicated Dialog opened from the Top Command Strip. Built
// on the shared frosted `Dialog` primitive; edits flow to meta via updateEnvironment.

import {
  updateEnvironment,
  useMapStore,
  type MissionDoc,
  type MissionMeta,
} from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import {
  ReadonlyField,
  SelectField,
  TextField,
  ToggleField,
} from './RightInspector/fields'

type Weather = MissionMeta['environment']['weather']

const WEATHER = [
  { value: 'clear', label: 'Clear' },
  { value: 'overcast', label: 'Overcast' },
  { value: 'heavy_rain', label: 'Heavy Rain' },
  { value: 'dense_fog', label: 'Dense Fog' },
]

export function MissionSettingsDialog({
  md,
  open,
  onOpenChange,
}: {
  md: MissionDoc
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const meta = useMapStore((s) => s.meta)
  const env = meta?.environment

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        title="Mission Settings"
        description="Global environment for this mission."
      >
        <div className="flex flex-col gap-4">
          <ReadonlyField label="Terrain" value={meta?.terrain ?? 'everon'} />

          <div className="grid grid-cols-2 gap-3">
            <TextField
              label="Time"
              type="time"
              value={env?.time ?? '06:00'}
              onChange={(time) => updateEnvironment(md, { time })}
            />
            <TextField
              label="View Distance (m)"
              type="number"
              value={env?.viewDistance ?? 1600}
              onChange={(v) => updateEnvironment(md, { viewDistance: Number(v) || 0 })}
            />
          </div>

          <SelectField
            label="Weather"
            value={env?.weather ?? 'clear'}
            options={WEATHER}
            onChange={(weather) => updateEnvironment(md, { weather: weather as Weather })}
          />

          <ToggleField
            label="Thermals enabled"
            checked={env?.thermals ?? false}
            onChange={(thermals) => updateEnvironment(md, { thermals })}
          />
        </div>
      </DialogContent>
    </Dialog>
  )
}
