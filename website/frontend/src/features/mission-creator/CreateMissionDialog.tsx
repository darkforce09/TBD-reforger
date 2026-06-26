import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { toast } from 'sonner'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { useCreateMission } from '@/hooks/mutations'
import { terrainLabel } from '@/lib/format'
import { cn } from '@/lib/utils'

interface CreateMissionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Defaults to navigating to the 2D editor at /missions/:id/edit. */
  onCreated?: (missionId: string) => void
}

// macOS pill controls — match the Event Manager create dialog (admin.tsx).
const PILL =
  'w-full rounded-full bg-white/5 px-5 py-3 text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none transition focus:ring-1 focus:ring-primary/50'

const DEFAULTS = {
  title: '',
  terrain: 'everon',
  gameMode: 'pve_coop',
  weather: 'clear',
  timeOfDay: '14:00',
  maxPlayers: 64,
}

/**
 * Transient "New Mission" dialog launched from the Mission Library (T-048) —
 * replaces the old full-page /missions/create wizard. Define environment, POST,
 * then jump into the 2D editor. Form resets on every close (clean slate on reopen).
 */
export function CreateMissionDialog({ open, onOpenChange, onCreated }: CreateMissionDialogProps) {
  const navigate = useNavigate()
  const create = useCreateMission()
  const [title, setTitle] = useState(DEFAULTS.title)
  const [terrain, setTerrain] = useState(DEFAULTS.terrain)
  const [gameMode, setGameMode] = useState(DEFAULTS.gameMode)
  const [weather, setWeather] = useState(DEFAULTS.weather)
  const [timeOfDay, setTimeOfDay] = useState(DEFAULTS.timeOfDay)
  const [maxPlayers, setMaxPlayers] = useState(DEFAULTS.maxPlayers)

  const resetForm = () => {
    setTitle(DEFAULTS.title)
    setTerrain(DEFAULTS.terrain)
    setGameMode(DEFAULTS.gameMode)
    setWeather(DEFAULTS.weather)
    setTimeOfDay(DEFAULTS.timeOfDay)
    setMaxPlayers(DEFAULTS.maxPlayers)
  }

  // Reset to a clean slate whenever the dialog closes (macOS Mail pattern).
  const handleOpenChange = (next: boolean) => {
    if (!next) resetForm()
    onOpenChange(next)
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!title.trim()) {
      toast.error('Title is required')
      return
    }
    create.mutate(
      {
        title: title.trim(),
        terrain,
        game_mode: gameMode,
        weather,
        time_of_day: timeOfDay,
        max_players: maxPlayers,
      },
      {
        onSuccess: (data: { id?: string }) => {
          toast.success('Mission created')
          handleOpenChange(false)
          if (data?.id) {
            if (onCreated) onCreated(data.id)
            else navigate(`/missions/${data.id}/edit`)
          }
        },
        onError: (e: unknown) =>
          toast.error(
            (e as { response?: { data?: { error?: string } } }).response?.data?.error ??
              'Failed to create mission',
          ),
      },
    )
  }

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent
        title="New Mission"
        description="Define terrain and environment before opening the 2D editor."
        className="max-w-lg"
      >
        <form onSubmit={handleSubmit} className="space-y-5">
          <div>
            <label className="mb-2 block text-label-md text-on-surface-variant">
              Operation Designation
            </label>
            <input
              type="text"
              placeholder="Enter operation designation..."
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              autoFocus
              className={PILL}
            />
          </div>

          <div>
            <p className="mb-2 text-label-md text-on-surface-variant">Terrain</p>
            <div className="grid gap-3 sm:grid-cols-2">
              {(['everon', 'arland'] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setTerrain(t)}
                  className={cn(
                    'rounded-xl border p-4 text-left text-label-md font-semibold transition',
                    terrain === t
                      ? 'border-primary bg-primary/10 text-on-surface'
                      : 'border-white/10 bg-white/5 text-on-surface-variant hover:bg-white/10',
                  )}
                >
                  {terrainLabel(t)}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="mb-2 block text-label-md text-on-surface-variant">Game Mode</label>
            <select value={gameMode} onChange={(e) => setGameMode(e.target.value)} className={PILL}>
              <option value="pve_coop">Co-op PvE</option>
              <option value="pvp">PvP</option>
              <option value="zeus">Zeus</option>
            </select>
          </div>

          <div>
            <label className="mb-2 block text-label-md text-on-surface-variant">Insertion Time</label>
            <input
              type="time"
              value={timeOfDay}
              onChange={(e) => setTimeOfDay(e.target.value)}
              className={PILL}
            />
          </div>

          <div>
            <label className="mb-2 block text-label-md text-on-surface-variant">Weather</label>
            <select value={weather} onChange={(e) => setWeather(e.target.value)} className={PILL}>
              <option value="clear">Clear (Default)</option>
              <option value="overcast">Overcast</option>
              <option value="heavy_rain">Heavy Rain</option>
              <option value="dense_fog">Dense Fog</option>
            </select>
          </div>

          <div>
            <label className="mb-2 block text-label-md text-on-surface-variant">Max Players</label>
            <select
              value={maxPlayers}
              onChange={(e) => setMaxPlayers(Number(e.target.value))}
              className={PILL}
            >
              {[16, 32, 48, 64, 96, 128].map((n) => (
                <option key={n} value={n}>
                  {n} Operators
                </option>
              ))}
            </select>
          </div>

          <button
            type="submit"
            disabled={create.isPending}
            className="w-full rounded-full bg-primary py-3 text-label-md font-semibold text-on-primary transition hover:bg-primary/90 disabled:opacity-50"
          >
            {create.isPending ? 'Creating…' : 'Create Mission Draft'}
          </button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
