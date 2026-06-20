// Top Command Strip (Ultra Plan §5.4; Eden docked shell). Left: menu stubs
// (File/Edit/View/Mission/Environment) + inline mission title. Center: Eden-style
// environment control — a time-of-day scrubber + weather. Right: Undo/Redo cluster,
// Mission Settings gear, and the (still-disabled) Export button. The Visual-Git timeline
// scrubber + working Export land in Phase 9.

import { useEffect, useReducer, useState } from 'react'
import { Download, Redo2, Settings2, Undo2 } from 'lucide-react'
import {
  setTitle,
  updateEnvironment,
  useMapStore,
  type MissionDoc,
  type MissionMeta,
  type UndoController,
} from '@/features/tactical-map'
import { cn } from '@/lib/utils'
import { overlayDocked } from './overlay'
import { MissionSettingsDialog } from './MissionSettingsDialog'

interface TopCommandStripProps {
  md: MissionDoc
  undo: UndoController
}

const MENUS = ['File', 'Edit', 'View', 'Mission', 'Environment']

const WEATHER: { value: MissionMeta['environment']['weather']; label: string }[] = [
  { value: 'clear', label: 'Clear' },
  { value: 'overcast', label: 'Overcast' },
  { value: 'heavy_rain', label: 'Heavy Rain' },
  { value: 'dense_fog', label: 'Dense Fog' },
]

const toMinutes = (hhmm: string) => {
  const [h, m] = hhmm.split(':').map(Number)
  return (h || 0) * 60 + (m || 0)
}
const toHHMM = (mins: number) =>
  `${String(Math.floor(mins / 60)).padStart(2, '0')}:${String(mins % 60).padStart(2, '0')}`

export function TopCommandStrip({ md, undo }: TopCommandStripProps) {
  const title = useMapStore((s) => s.meta?.title ?? '')
  const env = useMapStore((s) => s.meta?.environment)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [, bump] = useReducer((n: number) => n + 1, 0)
  // Re-render on undo-stack changes so Undo/Redo reflect canUndo/canRedo.
  useEffect(() => undo.subscribe(bump), [undo])

  const iconBtn =
    'rounded-md p-1.5 text-on-surface-variant transition-colors hover:bg-white/10 disabled:opacity-30 disabled:hover:bg-transparent'

  const time = env?.time ?? '06:00'

  return (
    <div className={cn(overlayDocked, 'flex h-full items-center gap-2 border-b border-white/10 px-3')}>
      <nav className="flex shrink-0 items-center">
        {MENUS.map((m) => (
          <button
            key={m}
            type="button"
            title={`${m} menu (soon)`}
            className="rounded-md px-2 py-1 text-label-md text-on-surface-variant transition-colors hover:bg-white/10"
          >
            {m}
          </button>
        ))}
      </nav>

      <span className="h-5 w-px bg-white/10" />

      <input
        value={title}
        onChange={(e) => setTitle(md, e.target.value)}
        placeholder="Untitled Mission"
        aria-label="Mission title"
        className="min-w-0 flex-1 bg-transparent text-label-md font-semibold text-on-surface outline-none placeholder:text-outline"
      />

      {/* Eden environment control: time scrubber + weather. */}
      <div className="flex shrink-0 items-center gap-2">
        <input
          type="range"
          min={0}
          max={1439}
          value={toMinutes(time)}
          onChange={(e) => updateEnvironment(md, { time: toHHMM(Number(e.target.value)) })}
          aria-label="Time of day"
          title="Time of day"
          className="h-1 w-28 cursor-pointer accent-primary"
        />
        <span className="w-10 font-mono text-code-md tabular-nums text-on-surface">{time}</span>
        <select
          value={env?.weather ?? 'clear'}
          onChange={(e) =>
            updateEnvironment(md, {
              weather: e.target.value as MissionMeta['environment']['weather'],
            })
          }
          aria-label="Weather"
          className="rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-1.5 py-1 text-label-sm text-on-surface outline-none focus:border-primary/60"
        >
          {WEATHER.map((w) => (
            <option key={w.value} value={w.value} className="bg-surface-container">
              {w.label}
            </option>
          ))}
        </select>
      </div>

      <span className="h-5 w-px bg-white/10" />

      <div className="flex shrink-0 items-center gap-0.5">
        <button
          type="button"
          className={iconBtn}
          onClick={undo.undo}
          disabled={!undo.canUndo()}
          aria-label="Undo"
        >
          <Undo2 className="size-4" />
        </button>
        <button
          type="button"
          className={iconBtn}
          onClick={undo.redo}
          disabled={!undo.canRedo()}
          aria-label="Redo"
        >
          <Redo2 className="size-4" />
        </button>
      </div>

      <button
        type="button"
        className={iconBtn}
        onClick={() => setSettingsOpen(true)}
        aria-label="Mission settings"
        title="Mission settings"
      >
        <Settings2 className="size-4" />
      </button>

      <button
        type="button"
        disabled
        title="The mission compiler lands in a later phase"
        className={cn(
          'inline-flex shrink-0 items-center gap-1.5 rounded-md bg-action/20 px-2.5 py-1 text-label-md text-on-surface-variant',
          'opacity-50',
        )}
      >
        <Download className="size-4" />
        Export
      </button>

      <MissionSettingsDialog md={md} open={settingsOpen} onOpenChange={setSettingsOpen} />
    </div>
  )
}
