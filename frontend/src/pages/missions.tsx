import { useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { toast } from 'sonner'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { Badge } from '@/components/ui/badge'
import {
  Sheet,
  SheetContent,
  SheetClose,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet'
import { useMission, useMissions } from '@/hooks/queries'
import { useCreateMission } from '@/hooks/mutations'
import { gameModeLabel, terrainLabel } from '@/lib/format'
import type { MissionDetail } from '@/types/api'
import { cn } from '@/lib/utils'

const SCOPES = [
  { label: 'Global Missions', value: 'global' },
  { label: 'My Missions', value: 'mine' },
  { label: 'Bookmarked', value: 'bookmarked' },
] as const

const TERRAINS = ['', 'everon', 'arland'] as const
const MODES = ['', 'pve_coop', 'pvp', 'zeus'] as const
// `player_count` is parsed server-side as a "lo-hi" range (internal/handlers/missions.go).
const PLAYER_RANGES = [
  { label: 'All Players', value: '' },
  { label: '1–8', value: '1-8' },
  { label: '9–16', value: '9-16' },
  { label: '17–32', value: '17-32' },
  { label: '33–64', value: '33-64' },
] as const

const SELECT_CLASS =
  'rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60'

// Cinematic fallback art so cards/hero never render as empty grey blocks.
const PLACEHOLDER_ART =
  'https://lh3.googleusercontent.com/aida/AP1WRLtxuwSoyDyCrRuQu8gTHWuSmoOWZq8e7gw0bSjjZCmteU96TomvCGHto-cuqHYV_0gxNUjw_Lx2SWgiEl2W3vEi6aVH84DpTky5lG8-FKDJOzH96TrwAJwGJwE3DSwSN1gRC7miWds0X7kNvMAZRBgQPu_5g2iX9RtJ3WYUlgHbfVLYcmV7TaHPUvhZHvvvKenG2B3S2CRER15d2kdG5YNFbtFwtwgzEIeYG2jP4GubWd7SMO0bADPFFA'

export function MissionLibraryPage() {
  const [scopeIdx, setScopeIdx] = useState(0)
  const [q, setQ] = useState('')
  const [terrain, setTerrain] = useState('')
  const [mode, setMode] = useState('')
  const [players, setPlayers] = useState('')
  const [previewId, setPreviewId] = useState<string | null>(null)
  const scope = SCOPES[scopeIdx]?.value ?? 'global'
  const filters: Record<string, string> = {}
  if (terrain) filters.terrain = terrain
  if (mode) filters.mode = mode
  if (players) filters.player_count = players
  if (q) filters.q = q

  const { data, isLoading, isError, error } = useMissions(scope, filters)
  const missions = data?.data ?? []
  // The hero spotlights the most recent operation in view (list is ordered newest-first).
  const featured = missions[0]

  return (
    <AuthGate>
      {/* Full-bleed: edge-to-edge frosted glass over the topo map fills the viewport
          and scrolls internally — no floating rounded card with margins. */}
      <div className="relative h-full w-full overflow-hidden">
        {/* Global topo-map background */}
        <div className="bg-topo-map bg-grid-overlay absolute inset-0 z-0" />
        <div className="custom-scrollbar relative z-10 h-full w-full overflow-y-auto bg-surface-glass backdrop-blur-xl">
        <div className="p-6 md:p-8">
        {/* Header + macOS segmented control */}
        <header className="mb-6">
          <h1 className="text-4xl font-bold tracking-tight text-on-surface uppercase">Mission Library</h1>
          <p className="mt-1 text-body-md text-on-surface-variant">
            Browse, filter, and deploy active operations across the theater.
          </p>
          <div className="mt-5 inline-flex gap-1 rounded-full border border-white/5 bg-black/20 p-1">
            {SCOPES.map((tab, i) => (
              <button
                key={tab.value}
                type="button"
                onClick={() => setScopeIdx(i)}
                className={cn(
                  'rounded-full px-4 py-1.5 text-label-md font-medium transition-all',
                  i === scopeIdx
                    ? 'bg-surface-glass text-on-surface shadow-md'
                    : 'text-on-surface-variant hover:text-on-surface',
                )}
              >
                {tab.label}
              </button>
            ))}
          </div>
        </header>

        <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
          {/* Featured Operation — cinematic hero. "LIVE OPERATION" is a presentational
              flourish; the mission model has no live-status flag. */}
          {featured && (
            <section className="relative mb-8 flex min-h-[320px] flex-col overflow-hidden rounded-2xl border border-white/10 bg-black/30 lg:flex-row">
              {/* Text content — sits above the art on small screens, side-by-side on desktop. */}
              <div className="relative z-10 flex w-full flex-col justify-center gap-4 p-8 lg:w-3/5">
                <div className="flex items-center gap-2 font-mono text-label-sm tracking-widest text-error-alert uppercase">
                  <span className="relative flex h-2.5 w-2.5">
                    <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-error-alert opacity-60" />
                    <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-error-alert" />
                  </span>
                  Live Operation
                </div>
                <h2 className="text-4xl font-black tracking-tighter text-on-surface uppercase xl:text-5xl">
                  {featured.title}
                </h2>
                <p className="max-w-prose text-body-md text-on-surface-variant line-clamp-3">
                  {featured.briefing ||
                    'Command has flagged this operation as the priority deployment. Review the dossier for objectives, ORBAT, and the armory loadout before committing forces to the field.'}
                </p>
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="primary">{gameModeLabel(featured.game_mode)}</Badge>
                  <Badge variant="neutral">{terrainLabel(featured.terrain)}</Badge>
                  <Badge variant="tertiary">{featured.max_players} OPERATORS</Badge>
                </div>
                <div>
                  <button
                    type="button"
                    onClick={() => setPreviewId(featured.id)}
                    className="mt-2 rounded-lg bg-primary px-6 py-3 font-mono text-label-md font-semibold tracking-wider text-on-primary uppercase transition-transform hover:scale-[1.02]"
                  >
                    [ View Dossier ]
                  </button>
                </div>
              </div>
              {/* Cinematic art — layers behind the text on small screens (absolute),
                  sits side-by-side on desktop (relative), always object-cover. */}
              <div className="absolute inset-0 lg:relative lg:inset-auto lg:w-2/5">
                <img
                  src={featured.thumbnail_url || PLACEHOLDER_ART}
                  alt=""
                  className="h-full w-full object-cover opacity-60 mix-blend-luminosity"
                />
                <div className="absolute inset-0 bg-gradient-to-r from-surface to-transparent" />
              </div>
            </section>
          )}

          {/* Unified search + filter toolbar */}
          <div className="mb-6 flex flex-wrap items-center gap-2 rounded-2xl border border-white/5 bg-black/20 p-2 backdrop-blur-md">
            <input
              type="search"
              placeholder="Search operations..."
              value={q}
              onChange={(e) => setQ(e.target.value)}
              className="min-w-[200px] flex-1 rounded-lg border border-white/10 bg-black/30 px-4 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60"
            />
            <select value={terrain} onChange={(e) => setTerrain(e.target.value)} className={SELECT_CLASS}>
              {TERRAINS.map((t) => (
                <option key={t || 'all'} value={t}>
                  {t ? terrainLabel(t) : 'All Terrains'}
                </option>
              ))}
            </select>
            <select value={mode} onChange={(e) => setMode(e.target.value)} className={SELECT_CLASS}>
              {MODES.map((m) => (
                <option key={m || 'all'} value={m}>
                  {m ? gameModeLabel(m) : 'All Modes'}
                </option>
              ))}
            </select>
            <select value={players} onChange={(e) => setPlayers(e.target.value)} className={SELECT_CLASS}>
              {PLAYER_RANGES.map((p) => (
                <option key={p.value || 'all'} value={p.value}>
                  {p.label}
                </option>
              ))}
            </select>
          </div>

          {/* Mission grid */}
          {missions.length === 0 ? (
            <p className="py-12 text-center text-on-surface-variant">No missions found.</p>
          ) : (
            <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
              {missions.map((m) => (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => setPreviewId(m.id)}
                  className="group overflow-hidden rounded-2xl border border-white/10 bg-surface-container/60 text-left transition-all hover:-translate-y-0.5 hover:border-white/25 hover:shadow-xl"
                >
                  <div className="relative h-48 w-full overflow-hidden bg-surface-container-low">
                    <img
                      src={m.thumbnail_url || PLACEHOLDER_ART}
                      alt=""
                      className="h-48 w-full object-cover transition-transform duration-500 group-hover:scale-105"
                    />
                    <span className="absolute top-3 left-3">
                      <Badge variant="primary" className="border-white/10 bg-black/40 backdrop-blur-md">
                        {gameModeLabel(m.game_mode)}
                      </Badge>
                    </span>
                  </div>
                  <div className="p-4">
                    <div className="mb-3 flex items-center gap-2">
                      {m.author_avatar ? (
                        <img src={m.author_avatar} alt="" className="h-6 w-6 rounded-full object-cover" />
                      ) : (
                        <span className="flex h-6 w-6 items-center justify-center rounded-full bg-surface-container-high text-label-sm text-on-surface-variant">
                          {m.author_name?.charAt(0).toUpperCase() || '?'}
                        </span>
                      )}
                      <span className="text-label-md text-on-surface-variant">{m.author_name}</span>
                    </div>
                    <h3 className="text-headline-sm font-bold text-on-surface">{m.title}</h3>
                    <div className="mt-3 flex flex-wrap gap-2">
                      <span className="rounded-md border border-white/5 bg-black/30 px-2 py-0.5 font-mono text-label-sm text-on-surface-variant">
                        {terrainLabel(m.terrain)}
                      </span>
                      <span className="rounded-md border border-white/5 bg-black/30 px-2 py-0.5 font-mono text-label-sm text-on-surface-variant">
                        {m.max_players} MAX
                      </span>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </QueryState>
        </div>
        </div>
      </div>

      {/* Slide-over mission dossier (no full-page navigation) */}
      <Sheet open={previewId != null} onOpenChange={(o) => !o && setPreviewId(null)}>
        {previewId && <MissionDossierSheet id={previewId} />}
      </Sheet>
    </AuthGate>
  )
}

/**
 * "Slide-Over Dossier" drawer — a wide cinematic variant of the shared Sheet.
 * Uses `<SheetContent bleed>` so the frosted Popup/Backdrop, transitions, and a11y
 * wiring come from the design-system primitive, while this component owns the
 * edge-to-edge hero, scrollable body, and sticky two-button action footer.
 */
function MissionDossierSheet({ id }: { id: string }) {
  const navigate = useNavigate()
  const { data: mission, isLoading, isError, error } = useMission(id)

  return (
    <SheetContent side="right" bleed className="w-full max-w-none md:w-[60vw]">
      {/* Edge-to-edge cinematic hero header */}
      <div className="relative h-64 w-full shrink-0 md:h-80">
        <img
          src={mission?.thumbnail_url || PLACEHOLDER_ART}
          alt=""
          className="h-full w-full object-cover"
        />
        <div className="absolute inset-0 bg-gradient-to-t from-surface/90 to-transparent" />
        <SheetClose
          aria-label="Close"
          className="absolute top-5 right-5 flex h-10 w-10 items-center justify-center rounded-full border border-white/10 bg-black/30 text-on-surface backdrop-blur-md transition-colors hover:bg-black/50"
        >
          <span className="material-symbols-outlined">close</span>
        </SheetClose>
        <div className="absolute right-8 bottom-6 left-8">
          <SheetTitle className="text-4xl font-black tracking-tighter text-white uppercase">
            {mission?.title ?? 'Mission Dossier'}
          </SheetTitle>
          <SheetDescription className="mt-1 font-mono text-label-md text-on-surface-variant">
            {mission?.author_name ? `Authored by ${mission.author_name}` : 'Loading dossier…'}
          </SheetDescription>
        </div>
      </div>

      {/* Scrollable content — pb-32 clears the sticky footer */}
      <div className="custom-scrollbar flex-1 overflow-y-auto px-8 pt-6 pb-32">
        <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
          {mission && <MissionDossierBody mission={mission} />}
        </QueryState>
      </div>

      {/* Sticky action footer */}
      <div className="absolute right-0 bottom-0 left-0 flex">
        <button
          type="button"
          onClick={() => navigate('/missions/create')}
          className="flex-1 bg-action/90 py-5 font-bold tracking-wide text-on-action backdrop-blur-md transition-colors hover:bg-action"
        >
          [ OPEN IN MISSION CREATOR ]
        </button>
        <button
          type="button"
          onClick={() => toast('2D Tactical Planner — coming soon')}
          className="flex-1 border-t border-white/10 bg-surface-container-high/90 py-5 font-bold tracking-wide text-primary backdrop-blur-md transition-colors hover:bg-surface-container-highest"
        >
          [ LAUNCH TACTICAL PLANNER ]
        </button>
      </div>
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
    <div className="space-y-8">
      <div className="flex flex-wrap gap-2">
        <Badge variant="primary">{gameModeLabel(mission.game_mode)}</Badge>
        <Badge variant="neutral">{terrainLabel(mission.terrain)}</Badge>
        {mission.current_version && <Badge variant="tertiary">v{mission.current_version.semver}</Badge>}
      </div>

      <section>
        <h3 className="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
          Tactical Briefing
        </h3>
        <p className="whitespace-pre-wrap text-body-md leading-relaxed text-on-surface-variant">
          {mission.briefing || 'No briefing provided.'}
        </p>
      </section>

      <dl className="grid grid-cols-1 gap-8 md:grid-cols-2">
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
    <div className="rounded-xl border border-white/10 bg-white/5 p-4">
      <dt className="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">{label}</dt>
      <dd className="mt-1 text-headline-sm text-on-surface">{value}</dd>
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
  const [maxPlayers, setMaxPlayers] = useState(64)

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
          if (data?.id) navigate(`/missions/${data.id}/edit`)
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
            <label className="mb-2 block text-sm font-medium">Weather</label>
            <select
              value={weather}
              onChange={(e) => setWeather(e.target.value)}
              className="mb-4 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            >
              <option value="clear">Clear (Default)</option>
              <option value="overcast">Overcast</option>
              <option value="heavy_rain">Heavy Rain</option>
              <option value="dense_fog">Dense Fog</option>
            </select>
            <label className="mb-2 block text-sm font-medium">Max Players</label>
            <select
              value={maxPlayers}
              onChange={(e) => setMaxPlayers(Number(e.target.value))}
              className="mb-6 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            >
              {[16, 32, 48, 64, 96, 128].map((n) => (
                <option key={n} value={n}>
                  {n} Operators
                </option>
              ))}
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
