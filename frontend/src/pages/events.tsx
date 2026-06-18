import { useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { toast } from 'sonner'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { MaterialIcon } from '@/components/MaterialIcon'
import { useCurrentModpack, useEvent, useOrbat } from '@/hooks/queries'
import { useRegisterMission, useWithdrawMission } from '@/hooks/mutations'
import { DEFAULT_AVATAR } from '@/lib/avatar'
import {
  countdownLabel,
  formatLocalDateTime,
  gameModeLabel,
  terrainLabel,
} from '@/lib/format'
import type { EventMissionDossier } from '@/types/api'
import { cn } from '@/lib/utils'

// --- Event Hub -------------------------------------------------------------

export function EventHubPage() {
  const { id } = useParams<{ id: string }>()
  const { data: event, isLoading, isError, error } = useEvent(id)
  const { data: modpack } = useCurrentModpack()
  const missions = event?.missions ?? []

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        {event && (
          <div className="mx-auto w-full max-w-5xl">
            <Link to="/events" className="mb-4 inline-flex items-center gap-1 text-sm text-primary hover:underline">
              <MaterialIcon name="chevron_left" className="text-base" /> All Operations
            </Link>

            {/* Hero */}
            <section
              className="relative mb-8 overflow-hidden rounded-xl border border-border-subtle bg-surface-container p-8"
              style={
                event.banner_image_url
                  ? {
                      backgroundImage: `linear-gradient(rgba(13,19,34,0.82),rgba(13,19,34,0.95)), url(${event.banner_image_url})`,
                      backgroundSize: 'cover',
                      backgroundPosition: 'center',
                    }
                  : undefined
              }
            >
              <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-primary/10 to-transparent" />
              <div className="relative flex flex-col gap-3">
                <span className="text-xs font-semibold uppercase tracking-widest text-on-surface-variant">
                  Operation Hub
                </span>
                <h1 className="text-3xl font-bold md:text-4xl">
                  {event.name_override || 'Untitled Operation'}
                </h1>
                <div className="text-2xl font-semibold tracking-widest text-primary">
                  T-MINUS {countdownLabel(event.start_time)}
                </div>
                <p className="text-on-surface-variant">{formatLocalDateTime(event.start_time)}</p>
                {event.briefing && (
                  <p className="max-w-2xl whitespace-pre-line text-on-surface-variant">{event.briefing}</p>
                )}
                <div className="mt-2 flex flex-wrap gap-3 text-sm">
                  <span className="flex items-center gap-2 rounded-lg border border-border-subtle bg-surface-container-high px-3 py-2">
                    <MaterialIcon name="headset_mic" className="text-primary" /> TS3: ts.tbdevent.eu
                  </span>
                  {modpack && (
                    <a
                      href={modpack.workshop_url ?? '#'}
                      target="_blank"
                      rel="noreferrer"
                      className="flex items-center gap-2 rounded-lg border border-border-subtle bg-surface-container-high px-3 py-2 hover:border-primary/40"
                    >
                      <MaterialIcon name="extension" className="text-primary" /> {modpack.name} v
                      {modpack.version}
                    </a>
                  )}
                </div>
              </div>
            </section>

            <h2 className="mb-4 text-lg font-semibold">Mission Dossiers</h2>
            {missions.length === 0 ? (
              <p className="text-on-surface-variant">No missions have been added to this operation yet.</p>
            ) : (
              <div className="flex flex-col gap-6">
                {missions.map((m, i) => (
                  <MissionDossier key={m.event_mission_id} eventId={event.id} index={i + 1} m={m} />
                ))}
              </div>
            )}
          </div>
        )}
      </QueryState>
    </AuthGate>
  )
}

function MissionDossier({
  eventId,
  index,
  m,
}: {
  eventId: string
  index: number
  m: EventMissionDossier
}) {
  return (
    <OpsCard className="bg-surface-container-high">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <span className="text-xs font-semibold uppercase tracking-widest text-on-surface-variant">
            Mission {index}
          </span>
          <h3 className="mt-1 text-xl font-semibold">{m.title}</h3>
          <p className="mt-1 text-sm text-on-surface-variant">
            {terrainLabel(m.terrain)} • {gameModeLabel(m.game_mode)} • {formatLocalDateTime(m.start_time)}
          </p>
        </div>
        <div className="text-right">
          {m.my_state && (
            <span className="rounded bg-success-muted px-2 py-0.5 text-xs font-semibold text-success">
              {m.my_state.toUpperCase()}
            </span>
          )}
          <p className="mt-1 text-sm text-on-surface-variant">{m.filled}/{m.total} slots filled</p>
        </div>
      </div>

      {m.briefing && (
        <p className="mt-2 whitespace-pre-line text-sm text-on-surface-variant">{m.briefing}</p>
      )}

      {m.armory_by_faction.length > 0 && (
        <div className="mt-3 grid gap-3 md:grid-cols-2">
          {m.armory_by_faction.map((fac) => (
            <div key={fac.faction} className="rounded-lg border border-border-subtle bg-surface-container p-3">
              <h4 className="mb-2 text-sm font-semibold">{fac.faction}</h4>
              <ul className="space-y-1 text-sm">
                {fac.items.map((it) => (
                  <li key={it.id} className="flex justify-between text-on-surface-variant">
                    <span>{it.item_name}</span>
                    <span>{it.quantity == null ? '∞' : `x${it.quantity}`}</span>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      )}

      <div className="mt-4 flex flex-wrap gap-3">
        <button
          type="button"
          disabled
          title="2D mission planner — coming soon"
          className="flex cursor-not-allowed items-center gap-2 rounded-lg border border-border-subtle px-4 py-2 text-sm text-on-surface-variant opacity-50"
        >
          <MaterialIcon name="map" /> Open in Mission Planner
        </button>
        <Link
          to={`/events/${eventId}/missions/${m.event_mission_id}/orbat`}
          className="flex items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary"
        >
          <MaterialIcon name="groups" /> Open ORBAT
        </Link>
      </div>
    </OpsCard>
  )
}

// --- ORBAT Selection (macOS split-pane) ------------------------------------

export function OrbatSelectionPage() {
  const { id, emid } = useParams<{ id: string; emid: string }>()
  const { data: event } = useEvent(id)
  const { data: squads, isLoading, isError, error } = useOrbat(emid)
  const register = useRegisterMission(emid ?? '')
  const withdraw = useWithdrawMission(emid ?? '')

  const dossier = event?.missions.find((m) => m.event_mission_id === emid)

  const factions = useMemo(
    () => [...new Set((squads ?? []).map((s) => s.faction))],
    [squads],
  )
  const [faction, setFaction] = useState<string | null>(null)
  const activeFaction = faction ?? factions[0] ?? null
  const factionSquads = (squads ?? []).filter((s) => s.faction === activeFaction)

  const [squadKey, setSquadKey] = useState<string | null>(null)
  const activeSquad = factionSquads.find((s) => s.squad === squadKey) ?? factionSquads[0] ?? null

  const [selectedSlot, setSelectedSlot] = useState<string | null>(null)

  const handleRegister = () => {
    if (!selectedSlot) return
    register.mutate(selectedSlot, {
      onSuccess: () => {
        toast.success('Registered for deployment')
        setSelectedSlot(null)
      },
      onError: () => toast.error('Could not claim that slot'),
    })
  }

  const handleWithdraw = () => {
    withdraw.mutate(undefined, {
      onSuccess: () => toast.success('Withdrawn from mission'),
      onError: () => toast.error('Could not withdraw'),
    })
  }

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-5xl">
          <Link
            to={`/events/${id}`}
            className="mb-4 inline-flex items-center gap-1 text-sm text-primary hover:underline"
          >
            <MaterialIcon name="chevron_left" className="text-base" /> {event?.name_override ?? 'Operation'}
          </Link>
          <PageHeader
            title={dossier?.title ?? 'Order of Battle'}
            subtitle="Select your faction, squad, and slot, then register for deployment."
          />

          <div className="grid overflow-hidden rounded-xl border border-border-subtle md:grid-cols-[260px_1fr]">
            {/* Left: navigation sidebar */}
            <aside className="border-b border-border-subtle bg-surface-container-high p-4 md:border-b-0 md:border-r">
              {factions.length > 1 && (
                <div className="mb-4 flex rounded-lg bg-surface p-1">
                  {factions.map((f) => (
                    <button
                      key={f}
                      type="button"
                      onClick={() => {
                        setFaction(f)
                        setSquadKey(null)
                        setSelectedSlot(null)
                      }}
                      className={cn(
                        'flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors',
                        f === activeFaction ? 'bg-primary text-on-primary' : 'text-on-surface-variant',
                      )}
                    >
                      {f}
                    </button>
                  ))}
                </div>
              )}

              <ul className="space-y-1">
                {factionSquads.map((s) => (
                  <li key={s.squad}>
                    <button
                      type="button"
                      onClick={() => {
                        setSquadKey(s.squad)
                        setSelectedSlot(null)
                      }}
                      className={cn(
                        'flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition-colors',
                        activeSquad?.squad === s.squad
                          ? 'bg-primary/10 text-on-surface'
                          : 'text-on-surface-variant hover:bg-surface-container',
                      )}
                    >
                      <span>
                        <span className="font-medium text-on-surface">{s.squad}</span>
                        {s.callsign && <span className="ml-1 text-xs">{s.callsign}</span>}
                      </span>
                      <span className={cn('text-xs', s.filled >= s.total ? 'text-error' : 'text-on-surface-variant')}>
                        {s.filled}/{s.total}
                      </span>
                    </button>
                  </li>
                ))}
                {factionSquads.length === 0 && (
                  <li className="px-3 py-2 text-sm text-on-surface-variant">No squads in this ORBAT.</li>
                )}
              </ul>
            </aside>

            {/* Right: slot detail pane */}
            <section className="flex min-h-[24rem] flex-col bg-surface-container">
              <div className="flex-1 p-4">
                {activeSquad ? (
                  <>
                    <div className="mb-3 flex items-center justify-between">
                      <h3 className="font-semibold">
                        {activeSquad.squad}
                        {activeSquad.callsign && (
                          <span className="text-sm font-normal text-on-surface-variant"> — {activeSquad.callsign}</span>
                        )}
                      </h3>
                      <span className="text-xs uppercase text-on-surface-variant">{activeFaction}</span>
                    </div>
                    <div className="overflow-hidden rounded-lg border border-border-subtle">
                      <table className="w-full text-sm">
                        <thead className="bg-surface-container-high text-xs font-semibold uppercase text-on-surface-variant">
                          <tr>
                            <th className="px-4 py-2 text-left">Role</th>
                            <th className="px-4 py-2 text-left">Status</th>
                          </tr>
                        </thead>
                        <tbody className="divide-y divide-border-subtle">
                          {activeSquad.slots.map((slot) => {
                            const taken = Boolean(slot.assigned_to)
                            const selected = selectedSlot === slot.id
                            return (
                              <tr
                                key={slot.id}
                                onClick={() => !taken && setSelectedSlot(selected ? null : slot.id)}
                                className={cn(
                                  !taken && 'cursor-pointer',
                                  selected && 'bg-primary/10',
                                  !taken && !selected && 'hover:bg-surface-container-high',
                                )}
                              >
                                <td className="px-4 py-2 font-medium">{slot.role}</td>
                                <td className="px-4 py-2">
                                  {taken ? (
                                    <span className="flex items-center gap-2 text-on-surface-variant">
                                      <img src={DEFAULT_AVATAR} alt="" className="h-6 w-6 rounded-full" />
                                      {slot.assigned_name || slot.assigned_to}
                                    </span>
                                  ) : (
                                    <span
                                      className={cn(
                                        'flex items-center gap-2',
                                        selected ? 'text-primary' : 'text-success',
                                      )}
                                    >
                                      <span className="h-2 w-2 rounded-full bg-current" />
                                      {selected ? 'Selected' : 'Available'}
                                    </span>
                                  )}
                                </td>
                              </tr>
                            )
                          })}
                        </tbody>
                      </table>
                    </div>
                  </>
                ) : (
                  <p className="text-on-surface-variant">Select a squad to view its slots.</p>
                )}
              </div>

              {/* Footer action bar */}
              <div className="flex items-center justify-between gap-3 border-t border-border-subtle bg-surface-container-high p-4">
                <div className="text-sm text-on-surface-variant">
                  {dossier?.my_state
                    ? `You are ${dossier.my_state} for this mission.`
                    : 'You are not registered for this mission.'}
                </div>
                <div className="flex gap-2">
                  {dossier?.my_state && (
                    <button
                      type="button"
                      onClick={handleWithdraw}
                      disabled={withdraw.isPending}
                      className="rounded-lg border border-error/50 px-4 py-2 text-sm text-error disabled:opacity-50"
                    >
                      Withdraw
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={handleRegister}
                    disabled={!selectedSlot || register.isPending}
                    className="rounded-lg bg-primary px-6 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                  >
                    Register for Deployment
                  </button>
                </div>
              </div>
            </section>
          </div>
        </div>
      </QueryState>
    </AuthGate>
  )
}
