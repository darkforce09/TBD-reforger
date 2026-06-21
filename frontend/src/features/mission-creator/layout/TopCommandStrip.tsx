// Top Command Strip (Ultra Plan §5.4; Eden docked shell). Left: menu stubs
// (File/Edit/View/Mission/Environment) + inline mission title with an unsaved-changes dot.
// Center: Eden-style environment control — a time scrubber + weather. Right: a Visual-Git
// history stub, Undo/Redo, Mission Settings gear, Save Version (semver snapshot → server),
// and Export (download the camelCase mod JSON). Persistence wired in Phase 9.

import { useEffect, useReducer, useState } from 'react'
import { Download, History, Redo2, Save, Settings2, Undo2 } from 'lucide-react'
import {
  setTitle,
  updateEnvironment,
  useMapStore,
  type MissionDoc,
  type MissionMeta,
  type UndoController,
} from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { cn } from '@/lib/utils'
import { overlayDocked } from './overlay'
import { MissionSettingsDialog } from './MissionSettingsDialog'
import type { SaveResult } from '../hooks/useMissionEditor'

interface TopCommandStripProps {
  md: MissionDoc
  undo: UndoController
  dirty: boolean
  suggestedSemver: string
  onExport: () => void
  onSaveVersion: (semver: string, notes?: string) => Promise<SaveResult>
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

export function TopCommandStrip({
  md,
  undo,
  dirty,
  suggestedSemver,
  onExport,
  onSaveVersion,
}: TopCommandStripProps) {
  const title = useMapStore((s) => s.meta?.title ?? '')
  const env = useMapStore((s) => s.meta?.environment)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [saveOpen, setSaveOpen] = useState(false)
  const [semver, setSemver] = useState(suggestedSemver)
  const [notes, setNotes] = useState('')
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [, bump] = useReducer((n: number) => n + 1, 0)
  // Re-render on undo-stack changes so Undo/Redo reflect canUndo/canRedo.
  useEffect(() => undo.subscribe(bump), [undo])

  const openSave = () => {
    setSemver(suggestedSemver)
    setNotes('')
    setSaveError(null)
    setSaveOpen(true)
  }
  const submitSave = async () => {
    setSaving(true)
    const res = await onSaveVersion(semver, notes)
    setSaving(false)
    if (res.ok) setSaveOpen(false)
    else setSaveError(res.error ?? 'Could not save')
  }

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

      <div className="flex min-w-0 flex-1 items-center gap-1.5">
        <input
          value={title}
          onChange={(e) => setTitle(md, e.target.value)}
          placeholder="Untitled Mission"
          aria-label="Mission title"
          className="min-w-0 flex-1 bg-transparent text-label-md font-semibold text-on-surface outline-none placeholder:text-outline"
        />
        {dirty && (
          <span
            title="Unsaved changes"
            aria-label="Unsaved changes"
            className="size-1.5 shrink-0 rounded-full bg-tactical-yellow"
          />
        )}
      </div>

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

      <button
        type="button"
        disabled
        title="Version history / Visual-Git (soon)"
        aria-label="Version history"
        className={iconBtn}
      >
        <History className="size-4" />
      </button>

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
        onClick={openSave}
        title="Save a new version snapshot to the server"
        className={cn(
          'inline-flex shrink-0 items-center gap-1.5 rounded-md px-2.5 py-1 text-label-md transition-colors',
          dirty
            ? 'bg-primary/20 text-primary hover:bg-primary/30'
            : 'text-on-surface-variant hover:bg-white/10',
        )}
      >
        <Save className="size-4" />
        Save
      </button>

      <button
        type="button"
        onClick={onExport}
        title="Download the mission JSON"
        className="inline-flex shrink-0 items-center gap-1.5 rounded-md bg-action/20 px-2.5 py-1 text-label-md text-on-surface transition-colors hover:bg-action/30"
      >
        <Download className="size-4" />
        Export
      </button>

      <MissionSettingsDialog md={md} open={settingsOpen} onOpenChange={setSettingsOpen} />

      <Dialog open={saveOpen} onOpenChange={setSaveOpen}>
        <DialogContent
          title="Save Version"
          description="Create an immutable version snapshot on the server (semver must be unique)."
        >
          <div className="flex flex-col gap-4">
            <label className="flex flex-col gap-1">
              <span className="text-label-sm uppercase tracking-wider text-outline">Semver</span>
              <input
                value={semver}
                onChange={(e) => setSemver(e.target.value)}
                placeholder="0.1.0"
                className="w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 font-mono text-code-md text-on-surface outline-none focus:border-primary/60"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-label-sm uppercase tracking-wider text-outline">Notes (optional)</span>
              <input
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                placeholder="What changed in this version"
                className="w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none focus:border-primary/60"
              />
            </label>
            {saveError && <p className="text-label-sm normal-case text-error">{saveError}</p>}
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setSaveOpen(false)}
                className="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={submitSave}
                disabled={saving || !semver.trim()}
                className="rounded-md bg-primary/20 px-3 py-1.5 text-label-md text-primary transition-colors hover:bg-primary/30 disabled:opacity-40"
              >
                {saving ? 'Saving…' : 'Save Version'}
              </button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
