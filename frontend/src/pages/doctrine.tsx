import { useState } from 'react'
import { toast } from 'sonner'
import { MaterialIcon } from '@/components/MaterialIcon'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { useCurrentModpack, useModpacks, useVehicles, useWikiPages } from '@/hooks/queries'
import { useSolveFireMission } from '@/hooks/mutations'
import { formatBytes } from '@/lib/format'
import { cn } from '@/lib/utils'
import { SplitPane, SplitPaneEmpty } from '@/components/ui/split-pane'
import { ListDetailItem } from '@/components/ui/list-detail-item'
import { Badge } from '@/components/ui/badge'
import type { VehicleRow } from '@/types/api'

export function ModpacksPage() {
  const { data: current, isLoading: loadingCurrent } = useCurrentModpack()
  const { data: all, isLoading: loadingAll, isError, error } = useModpacks()
  const modpack = current ?? all?.find((m) => m.is_current) ?? all?.[0]

  return (
    <AuthGate>
      <QueryState
        isLoading={loadingCurrent || loadingAll}
        isError={isError}
        error={error as Error}
        isEmpty={!modpack}
        emptyMessage="No modpack configured."
      >
        {modpack && (
          <div className="mx-auto w-full max-w-3xl">
            <PageHeader
              title="Server Modpacks"
              subtitle="Required dependencies for deployment. Arma Reforger handles mod downloads automatically when you connect."
            />
            <OpsCard glass>
              <h2 className="text-xl font-semibold">
                {modpack.name} (v{modpack.version})
              </h2>
              <p className="mt-2 text-sm text-on-surface-variant">
                Total Size: {formatBytes(modpack.total_size_bytes)} — Mods Included:{' '}
                {modpack.mods.length}
              </p>
              <ul className="mt-6 space-y-2">
                {modpack.mods.map((m) => (
                  <li
                    key={m.id}
                    className="flex items-center gap-2 rounded-lg border border-outline-variant/30 bg-surface-container px-3 py-2 text-label-md"
                  >
                    <MaterialIcon name="extension" className="text-primary" />
                    {m.name}
                    {m.is_key_dependency && (
                      <span className="ml-auto"><Badge variant="warning">Required</Badge></span>
                    )}
                  </li>
                ))}
              </ul>
              <div className="mt-6 flex flex-col gap-2 sm:flex-row">
                <button
                  type="button"
                  className="flex-1 rounded-lg bg-primary py-3 text-sm font-medium text-on-primary"
                  onClick={() => toast.message('Launch requires the Reforger client')}
                >
                  Launch Game &amp; Auto-Download
                </button>
                {modpack.workshop_url && (
                  <a
                    href={modpack.workshop_url}
                    target="_blank"
                    rel="noreferrer"
                    className="flex-1 rounded-lg border border-border-subtle py-3 text-center text-sm hover:bg-surface-container"
                  >
                    View Collection in Reforger Workshop
                  </a>
                )}
              </div>
            </OpsCard>
          </div>
        )}
      </QueryState>
    </AuthGate>
  )
}

export function WikiPage() {
  const { data: pages, isLoading: loadingPages } = useWikiPages()
  const { data: vehicles, isLoading: loadingVehicles, isError, error } = useVehicles()
  const categories = [...new Set((pages ?? []).map((p) => p.category))]
  const [category, setCategory] = useState<string | null>(null)
  const activeCategory = category ?? categories[0] ?? 'Vehicle Database'

  return (
    <AuthGate>
      <QueryState
        isLoading={loadingPages || loadingVehicles}
        isError={isError}
        error={error as Error}
      >
        <div className="mx-auto flex w-full max-w-6xl gap-6">
          {categories.length > 0 && (
            <aside className="hidden w-56 shrink-0 lg:block">
              <p className="mb-3 text-xs font-semibold tracking-widest text-on-surface-variant uppercase">
                SOP Categories
              </p>
              <ul className="space-y-1">
                {categories.map((c) => (
                  <li key={c}>
                    <button
                      type="button"
                      onClick={() => setCategory(c)}
                      className={cn(
                        'w-full rounded-lg px-3 py-2 text-left text-sm',
                        c === activeCategory
                          ? 'bg-primary/15 text-primary'
                          : 'text-on-surface-variant hover:bg-surface-container-high',
                      )}
                    >
                      {c}
                    </button>
                  </li>
                ))}
              </ul>
            </aside>
          )}
          <div className="min-w-0 flex-1">
            <PageHeader
              title="Vehicle Database &amp; IFF"
              subtitle="Learn strengths, weaknesses, and identification markers of armored assets."
            />
            <OpsCard className="mb-6 border-warning/30 bg-warning/10">
              <p className="text-sm text-warning">
                <strong>CRITICAL:</strong> Do NOT engage armored contacts unless you have positively
                identified them per IFF protocol.
              </p>
            </OpsCard>
            {!vehicles?.length ? (
              <p className="text-on-surface-variant">No vehicles in database.</p>
            ) : (
              <div className="mb-6 overflow-hidden rounded-xl border border-border-subtle">
                <table className="w-full text-sm">
                  <thead className="bg-surface-container-high text-xs font-semibold uppercase text-on-surface-variant">
                    <tr>
                      <th className="px-4 py-3 text-left">Vehicle</th>
                      <th className="px-4 py-3 text-left">Faction</th>
                      <th className="px-4 py-3 text-left">Class</th>
                      <th className="px-4 py-3 text-left">Threat</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border-subtle bg-surface-container">
                    {vehicles.map((v) => (
                      <tr key={v.id}>
                        <td className="px-4 py-3 font-medium">{v.name}</td>
                        <td className="px-4 py-3">{v.faction}</td>
                        <td className="px-4 py-3">{v.armor_type}</td>
                        <td className="px-4 py-3 text-on-surface-variant">
                          {v.primary_threat ?? '—'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      </QueryState>
    </AuthGate>
  )
}

function threatVariant(threat?: string): 'error' | 'warning' | 'success' | 'neutral' {
  if (!threat) return 'neutral'
  if (/high|heavy/i.test(threat)) return 'error'
  if (/med/i.test(threat)) return 'warning'
  if (/low/i.test(threat)) return 'success'
  return 'neutral'
}

export function VehicleDatabasePage() {
  const { data: vehicles, isLoading, isError, error } = useVehicles()
  const [q, setQ] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)

  const rows = vehicles ?? []
  const filtered = q
    ? rows.filter((v) => `${v.name} ${v.faction}`.toLowerCase().includes(q.toLowerCase()))
    : rows
  const selected = filtered.find((v) => v.id === selectedId) ?? filtered[0] ?? null

  // Group filtered vehicles by faction for the master list.
  const byFaction = filtered.reduce<Record<string, VehicleRow[]>>((acc, v) => {
    ;(acc[v.faction] ??= []).push(v)
    return acc
  }, {})

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <SplitPane
          masterWidth="20rem"
          masterHeader={
            <input
              type="search"
              placeholder="Search assets..."
              value={q}
              onChange={(e) => setQ(e.target.value)}
              className="w-full rounded-lg border border-outline-variant/40 bg-surface-container px-3 py-1.5 text-label-md outline-none focus:border-primary/60"
            />
          }
          master={
            filtered.length === 0 ? (
              <p className="px-1 py-4 text-label-md text-on-surface-variant">No vehicles in database.</p>
            ) : (
              Object.entries(byFaction).map(([faction, list]) => (
                <div key={faction} className="mb-2">
                  <p className="px-2 py-1 text-label-sm text-outline uppercase">{faction}</p>
                  {list.map((v) => (
                    <ListDetailItem
                      key={v.id}
                      active={selected?.id === v.id}
                      onClick={() => setSelectedId(v.id)}
                      title={v.name}
                      preview={v.armor_type}
                    />
                  ))}
                </div>
              ))
            )
          }
          detail={
            selected ? (
              <VehicleDossier vehicle={selected} />
            ) : (
              <SplitPaneEmpty
                icon={<MaterialIcon name="directions_car" className="text-4xl" />}
                message="Select an asset to view its dossier."
              />
            )
          }
        />
      </QueryState>
    </AuthGate>
  )
}

function VehicleDossier({ vehicle }: { vehicle: VehicleRow }) {
  return (
    <div>
      <div className="relative h-64 w-full overflow-hidden border-b border-outline-variant/30">
        {vehicle.profile_image_url ? (
          <img src={vehicle.profile_image_url} alt="" className="h-full w-full object-cover" />
        ) : (
          <div className="flex h-full w-full items-center justify-center bg-surface-container-low">
            <MaterialIcon name="directions_car" className="text-6xl text-outline" />
          </div>
        )}
        <div className="absolute inset-0 bg-gradient-to-t from-surface to-transparent" />
        <div className="absolute right-6 bottom-6 left-6">
          <div className="mb-2 flex flex-wrap items-center gap-2">
            <Badge variant="neutral">{vehicle.faction}</Badge>
            <Badge variant="primary">{vehicle.armor_type}</Badge>
            {vehicle.primary_threat && (
              <Badge variant={threatVariant(vehicle.primary_threat)}>
                Threat: {vehicle.primary_threat}
              </Badge>
            )}
          </div>
          <h1 className="text-headline-lg text-on-surface">{vehicle.name}</h1>
        </div>
      </div>
      <div className="grid gap-4 p-8 sm:grid-cols-2">
        <Spec label="Faction" value={vehicle.faction} />
        <Spec label="Classification" value={vehicle.armor_type} />
        <Spec label="Primary Threat" value={vehicle.primary_threat ?? '—'} />
        <Spec label="Amphibious" value={vehicle.amphibious ?? '—'} />
      </div>
    </div>
  )
}

function Spec({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-outline-variant/30 bg-surface-variant/30 p-4">
      <span className="text-label-sm text-on-surface-variant uppercase">{label}</span>
      <p className="mt-1 text-body-md text-on-surface">{value}</p>
    </div>
  )
}

export function MortarCalculatorPage() {
  const solve = useSolveFireMission()
  const [fpX, setFpX] = useState(1000)
  const [fpY, setFpY] = useState(2000)
  const [tgtX, setTgtX] = useState(2200)
  const [tgtY, setTgtY] = useState(1800)
  const solution = solve.data

  const handleSolve = () => {
    solve.mutate(
      { fp_x: fpX, fp_y: fpY, tgt_x: tgtX, tgt_y: tgtY, weapon_system: 'm252_81mm' },
      {
        onError: () => toast.error('Could not compute firing solution'),
      },
    )
  }

  return (
    <AuthGate>
      <div className="mx-auto w-full max-w-6xl">
        <PageHeader title="Mortar Calculator" subtitle="Enter grid coordinates for M252 81mm solution." />
        <OpsCard glass className="mb-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <label className="text-sm">
            FP X
            <input
              type="number"
              value={fpX}
              onChange={(e) => setFpX(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
          <label className="text-sm">
            FP Y
            <input
              type="number"
              value={fpY}
              onChange={(e) => setFpY(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
          <label className="text-sm">
            TGT X
            <input
              type="number"
              value={tgtX}
              onChange={(e) => setTgtX(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
          <label className="text-sm">
            TGT Y
            <input
              type="number"
              value={tgtY}
              onChange={(e) => setTgtY(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
        </OpsCard>
        <button
          type="button"
          onClick={handleSolve}
          disabled={solve.isPending}
          className="mb-4 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
        >
          {solve.isPending ? 'Computing…' : 'Calculate Solution'}
        </button>
        <div className="relative h-[calc(100vh-16rem)] overflow-hidden rounded-xl border border-border-subtle bg-surface-container-lowest">
          <div
            className="absolute inset-0 opacity-30"
            style={{
              backgroundImage:
                'linear-gradient(rgba(59,130,246,0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(59,130,246,0.08) 1px, transparent 1px)',
              backgroundSize: '40px 40px',
            }}
          />
          <div
            className="absolute top-1/4 left-1/3 h-4 w-4 rounded-full border-2 border-success bg-success/30"
            title="Fire Position"
          />
          <div
            className="absolute top-1/2 left-2/3 h-4 w-4 rounded-full border-2 border-error bg-error/30"
            title="Target"
          />
          <OpsCard glass className="absolute right-4 bottom-4 w-72 border-t-2 border-tertiary">
            <h2 className="text-sm font-semibold text-primary">
              Firing Solution — {solution?.weapon_system ?? 'M252 81mm'}
            </h2>
            {solution ? (
              <dl className="mt-3 space-y-2 font-mono text-sm">
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">Distance</dt>
                  <dd>{Math.round(solution.distance_m).toLocaleString()} m</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">Azimuth</dt>
                  <dd>{solution.azimuth_deg.toFixed(1)}°</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">Elevation</dt>
                  <dd className="text-primary">{solution.elevation_mils} mils</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">TOF</dt>
                  <dd>{solution.time_of_flight_s.toFixed(1)} s</dd>
                </div>
              </dl>
            ) : (
              <p className="mt-3 text-xs text-on-surface-variant">
                Enter coordinates and calculate to see solution.
              </p>
            )}
          </OpsCard>
        </div>
      </div>
    </AuthGate>
  )
}
