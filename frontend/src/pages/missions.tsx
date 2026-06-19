import { useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { toast } from 'sonner'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { MaterialIcon } from '@/components/MaterialIcon'
import { Badge } from '@/components/ui/badge'
import { Sheet, SheetContent } from '@/components/ui/sheet'
import { useMission, useMissions } from '@/hooks/queries'
import { useCreateMission } from '@/hooks/mutations'
import { gameModeLabel, terrainLabel } from '@/lib/format'
import type { MissionDetail } from '@/types/api'
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
  const [previewId, setPreviewId] = useState<string | null>(null)
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
                  'rounded-lg px-4 py-2 text-label-md font-medium transition-colors',
                  i === scopeIdx ? 'bg-primary text-on-primary' : 'bg-surface-container-high text-on-surface-variant',
                )}
              >
                {tab.label}
              </button>
            ))}
          </div>
          <OpsCard glass className="mb-6 flex-row flex-wrap gap-3">
            <input
              type="search"
              placeholder="Search operations..."
              value={q}
              onChange={(e) => setQ(e.target.value)}
              className="min-w-[200px] flex-1 rounded-lg border border-outline-variant/40 bg-surface px-3 py-2 text-label-md outline-none focus:border-primary/60"
            />
            <select
              value={terrain}
              onChange={(e) => setTerrain(e.target.value)}
              className="rounded-lg border border-outline-variant/40 bg-surface px-3 py-2 text-label-md outline-none focus:border-primary/60"
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
                <button
                  key={m.id}
                  type="button"
                  onClick={() => setPreviewId(m.id)}
                  className="group overflow-hidden rounded-xl border border-outline-variant/30 bg-surface-container text-left transition-all hover:border-primary/40 hover:shadow-xl"
                >
                  <div className="relative h-36 w-full overflow-hidden bg-surface-container-low">
                    {m.thumbnail_url ? (
                      <img
                        src={m.thumbnail_url}
                        alt=""
                        className="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
                      />
                    ) : (
                      <div className="flex h-full w-full items-center justify-center">
                        <MaterialIcon name="map" className="text-4xl text-outline" />
                      </div>
                    )}
                    <span className="absolute top-3 left-3">
                      <Badge variant="primary">{gameModeLabel(m.game_mode)}</Badge>
                    </span>
                  </div>
                  <div className="p-4">
                    <h3 className="text-headline-sm text-on-surface">{m.title}</h3>
                    <p className="mt-1 text-label-md text-on-surface-variant">
                      {m.author_name} · {terrainLabel(m.terrain)} · {m.max_players} max
                    </p>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </QueryState>

      {/* Slide-over mission dossier (no full-page navigation) */}
      <Sheet open={previewId != null} onOpenChange={(o) => !o && setPreviewId(null)}>
        {previewId && <MissionDossierSheet id={previewId} />}
      </Sheet>
    </AuthGate>
  )
}

function MissionDossierSheet({ id }: { id: string }) {
  const { data: mission, isLoading, isError, error } = useMission(id)
  return (
    <SheetContent title={mission?.title ?? 'Mission Dossier'} description={mission?.author_name}>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        {mission && <MissionDossierBody mission={mission} />}
      </QueryState>
    </SheetContent>
  )
}

/** Shared dossier content — used by the library slide-over and the deep-link page. */
function MissionDossierBody({ mission }: { mission: MissionDetail }) {
  const [faction, setFaction] = useState<string | null>(null)
  const factions = [...new Set(mission.armory.map((a) => a.faction))]
  const activeFaction = faction ?? factions[0] ?? ''
  const armoryItems = mission.armory.filter((a) => a.faction === activeFaction)

  return (
    <div className="space-y-5">
      {mission.thumbnail_url && (
        <img src={mission.thumbnail_url} alt="" className="h-40 w-full rounded-lg object-cover" />
      )}
      <div className="flex flex-wrap gap-2">
        <Badge variant="primary">{gameModeLabel(mission.game_mode)}</Badge>
        <Badge variant="neutral">{terrainLabel(mission.terrain)}</Badge>
        {mission.current_version && <Badge variant="tertiary">v{mission.current_version.semver}</Badge>}
      </div>

      <section>
        <h3 className="mb-2 text-label-md text-on-surface-variant uppercase">Briefing</h3>
        <p className="whitespace-pre-wrap text-body-md text-on-surface-variant">
          {mission.briefing || 'No briefing provided.'}
        </p>
      </section>

      <dl className="grid grid-cols-2 gap-3">
        <Detail label="Weather" value={mission.weather} />
        <Detail label="Time" value={mission.time_of_day} />
        <Detail label="Max Players" value={String(mission.max_players)} />
        <Detail label="Status" value={mission.status} />
      </dl>

      {factions.length > 0 && (
        <section>
          <h3 className="mb-2 text-label-md text-on-surface-variant uppercase">The Armory</h3>
          <div className="mb-3 flex gap-2">
            {factions.map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => setFaction(f)}
                className={cn(
                  'rounded-lg px-3 py-1.5 text-label-md',
                  f === activeFaction ? 'bg-primary text-on-primary' : 'bg-surface-container text-on-surface-variant',
                )}
              >
                {f}
              </button>
            ))}
          </div>
          <div className="grid gap-2">
            {armoryItems.map((item) => (
              <div
                key={item.id}
                className="flex justify-between rounded-lg border border-outline-variant/30 bg-surface-container p-3 text-label-md"
              >
                <span className="text-on-surface">{item.item_name}</span>
                <span className="text-on-surface-variant">
                  {item.quantity != null ? `x${item.quantity}` : '∞'}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  )
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-outline-variant/30 bg-surface-variant/30 p-3">
      <dt className="text-label-sm text-on-surface-variant uppercase">{label}</dt>
      <dd className="mt-0.5 text-label-md text-on-surface">{value}</dd>
    </div>
  )
}

export function MissionOverviewPage() {
  const { id } = useParams<{ id: string }>()
  const { data: mission, isLoading, isError, error } = useMission(id)

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        {mission && (
          <div className="mx-auto w-full max-w-3xl">
            <PageHeader
              title={mission.title}
              subtitle={`by ${mission.author_name} — Terrain: ${terrainLabel(mission.terrain)}${mission.current_version ? ` — v${mission.current_version.semver}` : ''}`}
            />
            <OpsCard glass>
              <MissionDossierBody mission={mission} />
            </OpsCard>
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
