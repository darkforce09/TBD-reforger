// Mission Settings modal (Ultra Plan §5.4): global environment + terrain, moved out
// of the right panel into a dedicated Dialog opened from the Top Command Strip. Built
// on the shared frosted `Dialog` primitive; edits flow to meta via updateEnvironment.

import {
  updateEnvironment,
  useMapStore,
  useMapStyle,
  setMapStyle,
  useClassToggles,
  setClassToggle,
  type MapStyle,
  type MissionDoc,
  type MissionMeta,
} from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import {
  Field,
  ReadonlyField,
  SelectField,
  SliderField,
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

/** Stored 0–1 hillshade blend → slider percent (0.1% steps); unset defaults to 40%. */
function hillshadePercent(env: MissionMeta['environment'] | undefined): number {
  return Math.round((env?.hillshadeOpacity ?? 0.4) * 1000) / 10
}

/** One map-style segment button (Satellite | Hybrid | Map, T-090.5.1 §4.3) — active styling
 *  keyed on the current style. */
function MapStyleButton({
  style,
  label,
  current,
}: {
  style: MapStyle
  label: string
  current: MapStyle
}) {
  return (
    <button
      type="button"
      onClick={() => setMapStyle(style)}
      className={`flex-1 rounded-md border px-2.5 py-1.5 text-label-md transition-colors ${
        current === style
          ? 'border-primary/60 bg-primary/10 text-on-surface'
          : 'border-outline-variant/40 text-outline hover:bg-surface-container-lowest/60'
      }`}
    >
      {label}
    </button>
  )
}

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
  const hillshadeOn = env?.showHillshade === true
  // Map style is a per-user pref (localStorage), not mission meta — it travels with the
  // user, not the mission (N8). Hybrid = dimmed satellite under (future) full vectors.
  const mapStyle = useMapStyle()
  const classToggles = useClassToggles()

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent title="Mission Settings" description="Global environment for this mission.">
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

          <Field label="Map style">
            <div className="flex gap-2">
              <MapStyleButton style="satellite" label="Satellite" current={mapStyle} />
              <MapStyleButton style="hybrid" label="Hybrid" current={mapStyle} />
              <MapStyleButton style="map" label="Map" current={mapStyle} />
            </div>
          </Field>

          <ToggleField
            label="Show grid"
            checked={env?.showGrid !== false}
            onChange={(showGrid) => updateEnvironment(md, { showGrid })}
          />

          <ToggleField
            label="Show hillshade"
            checked={hillshadeOn}
            onChange={(showHillshade) => updateEnvironment(md, { showHillshade })}
          />

          <SliderField
            label="Hillshade strength"
            value={hillshadePercent(env)}
            step={0.1}
            disabled={!hillshadeOn}
            onChange={(pct) =>
              updateEnvironment(md, { hillshadeOpacity: Math.round(pct * 10) / 1000 })
            }
          />

          <Field label="World layers">
            <div className="flex flex-col gap-2">
              <ToggleField
                label="Fences"
                checked={classToggles.fences}
                onChange={(on) => setClassToggle('fences', on)}
              />
              <ToggleField
                label="Airfield"
                checked={classToggles.airfield}
                onChange={(on) => setClassToggle('airfield', on)}
              />
            </div>
          </Field>
        </div>
      </DialogContent>
    </Dialog>
  )
}
