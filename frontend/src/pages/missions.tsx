import { useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { toast } from 'sonner'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { useMission, useMissions } from '@/hooks/queries'
import { useCreateMission } from '@/hooks/mutations'
import { gameModeLabel, terrainLabel } from '@/lib/format'
import { cn } from '@/lib/utils'

const SCOPES = [
  { label: 'Global', value: 'global' },
  { label: 'My Missions', value: 'mine' },
  { label: 'Bookmarked', value: 'bookmarked' },
] as const

const TERRAINS = ['', 'everon', 'arland'] as const

export function MissionLibraryPage() {
  const [scopeIdx, setScopeIdx] = useState(0)
  const [q, setQ] = useState('')
  const [terrain, setTerrain] = useState('')
  const scope = SCOPES[scopeIdx]?.value ?? 'global'
  const filters: Record<string, string> = {}
  if (terrain) filters.terrain = terrain
  if (q) filters.q = q

  const { data, isLoading, isError, error } = useMissions(scope, filters)
  const missions = data?.data ?? []

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-6xl">
          <PageHeader title="Mission Library" subtitle="Browse, filter, and deploy active operations." />
          <div className="mb-6 flex flex-wrap gap-2">
            {SCOPES.map((tab, i) => (
              <button
                key={tab.value}
                type="button"
                onClick={() => setScopeIdx(i)}
                className={cn(
                  'rounded-lg px-4 py-2 text-sm font-medium',
                  i === scopeIdx ? 'bg-primary text-on-primary' : 'bg-surface-container-high text-on-surface-variant',
                )}
              >
                {tab.label}
              </button>
            ))}
          </div>
          <OpsCard className="mb-6 flex flex-wrap gap-3 bg-surface-container-high">
            <input
              type="search"
              placeholder="Search operations..."
              value={q}
              onChange={(e) => setQ(e.target.value)}
              className="min-w-[200px] flex-1 rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
            <select
              value={terrain}
              onChange={(e) => setTerrain(e.target.value)}
              className="rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            >
              {TERRAINS.map((t) => (
                <option key={t || 'all'} value={t}>
                  {t ? terrainLabel(t) : 'All Terrains'}
                </option>
              ))}
            </select>
          </OpsCard>
          {missions.length === 0 ? (
            <p className="text-on-surface-variant">No missions found.</p>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {missions.map((m) => (
                <OpsCard key={m.id} className="bg-surface-container-high">
                  <div className="mb-2 flex flex-wrap gap-1">
                    <span className="rounded bg-primary/15 px-2 py-0.5 text-xs text-primary">
                      {gameModeLabel(m.game_mode)}
                    </span>
                    <span className="rounded bg-surface-container-highest px-2 py-0.5 text-xs text-on-surface-variant">
                      {m.status}
                    </span>
                  </div>
                  <h3 className="text-lg font-semibold">{m.title}</h3>
                  <p className="mt-1 text-sm text-on-surface-variant">
                    {m.author_name} — {terrainLabel(m.terrain)} — {m.max_players} max
                  </p>
                  <Link
                    to={`/missions/${m.id}`}
                    className="mt-4 inline-block text-sm font-medium text-primary hover:underline"
                  >
                    View Operation Intel
                  </Link>
                </OpsCard>
              ))}
            </div>
          )}
        </div>
      </QueryState>
    </AuthGate>
  )
}

export function MissionOverviewPage() {
  const { id } = useParams<{ id: string }>()
  const { data: mission, isLoading, isError, error } = useMission(id)
  const [faction, setFaction] = useState<string | null>(null)

  const factions = [...new Set((mission?.armory ?? []).map((a) => a.faction))]
  const activeFaction = faction ?? factions[0] ?? ''
  const armoryItems = (mission?.armory ?? []).filter((a) => a.faction === activeFaction)

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        {mission && (
          <div className="mx-auto w-full max-w-6xl">
            <PageHeader
              title={mission.title}
              subtitle={`by ${mission.author_name} — Terrain: ${terrainLabel(mission.terrain)}${mission.current_version ? ` — v${mission.current_version.semver}` : ''}`}
            />
            <div className="grid gap-6 lg:grid-cols-3">
              <div className="space-y-6 lg:col-span-2">
                <OpsCard className="bg-surface-container-high">
                  <h2 className="mb-3 text-lg font-semibold">Mission Briefing</h2>
                  <p className="text-on-surface-variant whitespace-pre-wrap">
                    {mission.briefing || 'No briefing provided.'}
                  </p>
                </OpsCard>
                {mission.thumbnail_url && (
                  <OpsCard className="bg-surface-container-high">
                    <h2 className="mb-3 text-lg font-semibold">Topographic Preview</h2>
                    <img
                      src={mission.thumbnail_url}
                      alt=""
                      className="h-48 w-full rounded-lg object-cover"
                    />
                  </OpsCard>
                )}
                {factions.length > 0 && (
                  <OpsCard className="bg-surface-container-high">
                    <h2 className="mb-3 text-lg font-semibold">The Armory</h2>
                    <div className="mb-4 flex gap-2">
                      {factions.map((f) => (
                        <button
                          key={f}
                          type="button"
                          onClick={() => setFaction(f)}
                          className={cn(
                            'rounded-lg px-3 py-1.5 text-sm',
                            f === activeFaction
                              ? 'bg-primary text-on-primary'
                              : 'bg-surface-container text-on-surface-variant',
                          )}
                        >
                          {f}
                        </button>
                      ))}
                    </div>
                    <div className="grid gap-2 sm:grid-cols-2">
                      {armoryItems.map((item) => (
                        <div
                          key={item.id}
                          className="rounded-lg border border-border-subtle bg-surface-container p-3 text-sm"
                        >
                          {item.item_name} —{' '}
                          {item.quantity != null ? `x${item.quantity} Available` : 'Unlimited'}
                        </div>
                      ))}
                    </div>
                  </OpsCard>
                )}
              </div>
              <div className="space-y-6">
                <OpsCard className="bg-surface-container-high">
                  <h2 className="mb-3 text-lg font-semibold">Mission Details</h2>
                  <dl className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <dt className="text-on-surface-variant">Mode</dt>
                      <dd>{gameModeLabel(mission.game_mode)}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="text-on-surface-variant">Weather</dt>
                      <dd>{mission.weather}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="text-on-surface-variant">Time</dt>
                      <dd>{mission.time_of_day}</dd>
                    </div>
                    <div className="flex justify-between">
                      <dt className="text-on-surface-variant">Max Players</dt>
                      <dd>{mission.max_players}</dd>
                    </div>
                  </dl>
                </OpsCard>
              </div>
            </div>
          </div>
        )}
      </QueryState>
    </AuthGate>
  )
}

export function MissionCreatorPage() {
  const navigate = useNavigate()
  const create = useCreateMission()
  const [title, setTitle] = useState('')
  const [terrain, setTerrain] = useState('everon')
  const [gameMode, setGameMode] = useState('pve_coop')
  const [weather, setWeather] = useState('clear')
  const [timeOfDay, setTimeOfDay] = useState('14:00')

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
      },
      {
        onSuccess: (data: { id?: string }) => {
          toast.success('Mission created')
          if (data?.id) navigate(`/missions/${data.id}`)
        },
        onError: () => toast.error('Failed to create mission'),
      },
    )
  }

  return (
    <AuthGate>
      <div className="mx-auto w-full max-w-2xl">
        <PageHeader
          title="Initialize New Mission"
          subtitle="Define the environment parameters before launching the 2D Editor Canvas."
        />
        <OpsCard className="bg-surface-container-high">
          <form onSubmit={handleSubmit}>
            <label className="mb-2 block text-sm font-medium">Operation Designation</label>
            <input
              type="text"
              placeholder="Enter operation designation..."
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className="mb-6 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
            <p className="mb-3 text-sm font-medium">Terrain</p>
            <div className="mb-6 grid gap-3 sm:grid-cols-2">
              {(['everon', 'arland'] as const).map((t) => (
                <button
                  key={t}
                  type="button"
                  onClick={() => setTerrain(t)}
                  className={cn(
                    'rounded-lg border p-4 text-left',
                    terrain === t
                      ? 'border-primary bg-primary/10'
                      : 'border-border-subtle bg-surface-container',
                  )}
                >
                  <span className="font-semibold">{terrainLabel(t)}</span>
                </button>
              ))}
            </div>
            <label className="mb-2 block text-sm font-medium">Game Mode</label>
            <select
              value={gameMode}
              onChange={(e) => setGameMode(e.target.value)}
              className="mb-4 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            >
              <option value="pve_coop">Co-op PvE</option>
              <option value="pvp">PvP</option>
              <option value="zeus">Zeus</option>
            </select>
            <label className="mb-2 block text-sm font-medium">Insertion Time</label>
            <input
              type="time"
              value={timeOfDay}
              onChange={(e) => setTimeOfDay(e.target.value)}
              className="mb-4 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
            <select
              value={weather}
              onChange={(e) => setWeather(e.target.value)}
              className="mb-6 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            >
              <option value="clear">Clear (Default)</option>
              <option value="overcast">Overcast</option>
              <option value="rain">Light Rain</option>
            </select>
            <button
              type="submit"
              disabled={create.isPending}
              className="w-full rounded-lg bg-primary py-3 text-sm font-medium text-on-primary disabled:opacity-50"
            >
              {create.isPending ? 'Creating…' : 'Create Mission Draft'}
            </button>
          </form>
        </OpsCard>
      </div>
    </AuthGate>
  )
}
