import { useMemo, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { toast } from 'sonner'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { MaterialIcon } from '@/components/MaterialIcon'
import { useCurrentModpack, useEvent, useMemberSearch, useOrbat } from '@/hooks/queries'
import {
  useAssignSlot,
  useRegisterMission,
  useReleaseSquad,
  useReserveSquad,
  useWithdrawMission,
} from '@/hooks/mutations'
import { useAuthStore } from '@/store/useAuthStore'
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
                  <MissionDossier key={m.event_mission_id} index={i + 1} m={m} />
                ))}
              </div>
            )}
          </div>
        )}
      </QueryState>
    </AuthGate>
  )
}

function MissionDossier({ index, m }: { index: number; m: EventMissionDossier }) {
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
        <div className="flex flex-col items-end gap-2">
          {m.my_state && (
            <span className="rounded bg-success-muted px-2 py-0.5 text-xs font-semibold text-success">
              {m.my_state.toUpperCase()}
            </span>
          )}
          <p className="text-sm text-on-surface-variant">{m.filled}/{m.total} slots filled</p>
          <button
            type="button"
            disabled
            title="2D mission planner — coming soon"
            className="flex cursor-not-allowed items-center gap-2 rounded-lg border border-border-subtle px-3 py-1.5 text-xs text-on-surface-variant opacity-50"
          >
            <MaterialIcon name="map" className="text-base" /> Mission Planner
          </button>
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

      {/* ORBAT is registered inline — no extra navigation. */}
      <div className="mt-4">
        <OrbatSelector emid={m.event_mission_id} myState={m.my_state} />
      </div>
    </OpsCard>
  )
}

// --- Reusable ORBAT split-pane selector ------------------------------------

function OrbatSelector({ emid, myState }: { emid: string; myState?: string }) {
  const { data: squads, isLoading } = useOrbat(emid)
  const register = useRegisterMission(emid)
  const withdraw = useWithdrawMission(emid)
  const reserve = useReserveSquad(emid)
  const release = useReleaseSquad(emid)

  const user = useAuthStore((s) => s.user)
  const isLeader = useAuthStore((s) => s.hasMinRole('leader'))
  const isAdmin = useAuthStore((s) => s.hasMinRole('admin'))

  const factions = useMemo(() => [...new Set((squads ?? []).map((s) => s.faction))], [squads])
  const [faction, setFaction] = useState<string | null>(null)
  const activeFaction = faction ?? factions[0] ?? null
  const factionSquads = (squads ?? []).filter((s) => s.faction === activeFaction)

  const [squadKey, setSquadKey] = useState<string | null>(null)
  const activeSquad = factionSquads.find((s) => s.squad === squadKey) ?? factionSquads[0] ?? null

  const [selectedSlot, setSelectedSlot] = useState<string | null>(null)
  const [assigning, setAssigning] = useState<string | null>(null) // slot id with picker open

  const pickSquad = (squad: string) => {
    setSquadKey(squad)
    setSelectedSlot(null)
    setAssigning(null)
  }

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

  if (isLoading) {
    return <p className="text-sm text-on-surface-variant">Loading ORBAT…</p>
  }
  if (!squads || squads.length === 0) {
    return <p className="text-sm text-on-surface-variant">No ORBAT slots defined for this mission.</p>
  }

  const reservedBy = activeSquad?.reserved_by
  const iAmReserver = Boolean(reservedBy && reservedBy === user?.discord_id)
  const canManage = Boolean(activeSquad) && (isAdmin || iAmReserver) // may assign slots
  const lockedForMe = Boolean(reservedBy) && !canManage // held by someone else
  const selfRegister = Boolean(activeSquad) && !canManage && !lockedForMe // open self-claim

  return (
    <div className="grid overflow-hidden rounded-xl border border-border-subtle md:grid-cols-[240px_1fr]">
      {/* Left: navigation sidebar */}
      <aside className="border-b border-border-subtle bg-surface-container p-4 md:border-b-0 md:border-r">
        {factions.length > 1 && (
          <div className="mb-4 flex rounded-lg bg-surface p-1">
            {factions.map((f) => (
              <button
                key={f}
                type="button"
                onClick={() => {
                  setFaction(f)
                  pickSquad('')
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
                onClick={() => pickSquad(s.squad)}
                className={cn(
                  'flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition-colors',
                  activeSquad?.squad === s.squad
                    ? 'bg-primary/10 text-on-surface'
                    : 'text-on-surface-variant hover:bg-surface-container-high',
                )}
              >
                <span className="flex items-center gap-1.5">
                  {s.reserved_by && <MaterialIcon name="lock" className="text-sm text-on-surface-variant" />}
                  <span className="font-medium text-on-surface">{s.squad}</span>
                  {s.callsign && <span className="ml-1 text-xs">{s.callsign}</span>}
                </span>
                <span className={cn('text-xs', s.filled >= s.total ? 'text-error' : 'text-on-surface-variant')}>
                  {s.filled}/{s.total}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </aside>

      {/* Right: slot detail pane */}
      <section className="flex min-h-[18rem] flex-col bg-surface-container-high">
        <div className="flex-1 p-4">
          {activeSquad ? (
            <>
              <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                <h4 className="font-semibold">
                  {activeSquad.squad}
                  {activeSquad.callsign && (
                    <span className="text-sm font-normal text-on-surface-variant"> | {activeSquad.callsign}</span>
                  )}
                </h4>
                <div className="flex items-center gap-2">
                  {reservedBy ? (
                    <>
                      <span className="flex items-center gap-1 rounded bg-surface-container-highest px-2 py-0.5 text-xs text-on-surface-variant">
                        <MaterialIcon name="lock" className="text-sm" />
                        Reserved by {activeSquad.reserved_by_name || 'a leader'}
                      </span>
                      {(iAmReserver || isAdmin) && (
                        <button
                          type="button"
                          onClick={() =>
                            release.mutate(activeSquad.squad, {
                              onSuccess: () => toast.success('Squad released'),
                              onError: () => toast.error('Could not release squad'),
                            })
                          }
                          disabled={release.isPending}
                          className="rounded-lg border border-border-subtle px-3 py-1 text-xs text-on-surface-variant disabled:opacity-50"
                        >
                          Release
                        </button>
                      )}
                    </>
                  ) : (
                    isLeader && (
                      <button
                        type="button"
                        onClick={() =>
                          reserve.mutate(activeSquad.squad, {
                            onSuccess: () => toast.success(`Reserved ${activeSquad.squad}`),
                            onError: () => toast.error('Could not reserve squad'),
                          })
                        }
                        disabled={reserve.isPending}
                        className="flex items-center gap-1 rounded-lg bg-primary px-3 py-1 text-xs font-medium text-on-primary disabled:opacity-50"
                      >
                        <MaterialIcon name="lock" className="text-sm" /> Reserve Squad
                      </button>
                    )
                  )}
                </div>
              </div>

              <ul className="overflow-hidden rounded-lg border border-border-subtle divide-y divide-border-subtle">
                {activeSquad.slots.map((slot) => {
                  const taken = Boolean(slot.assigned_to)
                  const selected = selectedSlot === slot.id
                  const clickable = selfRegister && !taken
                  return (
                    <li key={slot.id}>
                      <div
                        onClick={() => clickable && setSelectedSlot(selected ? null : slot.id)}
                        className={cn(
                          'flex items-center justify-between gap-3 px-4 py-2 text-sm',
                          clickable && 'cursor-pointer',
                          selected && 'bg-primary/10',
                          clickable && !selected && 'hover:bg-surface-container',
                        )}
                      >
                        <span className="flex items-center gap-2">
                          <span className="text-on-surface-variant tabular-nums">{slot.number}:</span>
                          <span className="font-medium">{slot.role}</span>
                          {slot.loadout && (
                            <span className="text-on-surface-variant">({slot.loadout})</span>
                          )}
                          {slot.tag && (
                            <span className="rounded bg-surface-container-highest px-1.5 py-0.5 text-[10px] font-semibold text-on-surface-variant">
                              {slot.tag}
                            </span>
                          )}
                        </span>
                        <span className="shrink-0">
                          {taken ? (
                            <span className="flex items-center gap-2 text-on-surface-variant">
                              <img src={DEFAULT_AVATAR} alt="" className="h-6 w-6 rounded-full" />
                              {slot.assigned_name || slot.assigned_to}
                            </span>
                          ) : canManage ? (
                            <button
                              type="button"
                              onClick={() => setAssigning(assigning === slot.id ? null : slot.id)}
                              className="rounded-lg border border-border-subtle px-3 py-1 text-xs text-primary"
                            >
                              {assigning === slot.id ? 'Cancel' : 'Assign'}
                            </button>
                          ) : lockedForMe ? (
                            <span className="text-xs text-on-surface-variant">Reserved</span>
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
                        </span>
                      </div>
                      {canManage && !taken && assigning === slot.id && (
                        <AssignPicker emid={emid} slotId={slot.id} onClose={() => setAssigning(null)} />
                      )}
                    </li>
                  )
                })}
              </ul>
            </>
          ) : (
            <p className="text-on-surface-variant">Select a squad to view its slots.</p>
          )}
        </div>

        {/* Footer action bar */}
        <div className="flex items-center justify-between gap-3 border-t border-border-subtle bg-surface-container p-4">
          <div className="text-sm text-on-surface-variant">
            {myState
              ? `You are ${myState} for this mission.`
              : lockedForMe
                ? 'This squad is reserved by a leader.'
                : canManage
                  ? 'Assign members to fill this squad.'
                  : 'Select an open slot to deploy.'}
          </div>
          <div className="flex gap-2">
            {myState && (
              <button
                type="button"
                onClick={handleWithdraw}
                disabled={withdraw.isPending}
                className="rounded-lg border border-error/50 px-4 py-2 text-sm text-error disabled:opacity-50"
              >
                Withdraw
              </button>
            )}
            {selfRegister && (
              <button
                type="button"
                onClick={handleRegister}
                disabled={!selectedSlot || register.isPending}
                className="rounded-lg bg-primary px-6 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
              >
                Register for Deployment
              </button>
            )}
          </div>
        </div>
      </section>
    </div>
  )
}

// AssignPicker is the leader's inline member typeahead for filling a reserved
// squad's slot.
function AssignPicker({
  emid,
  slotId,
  onClose,
}: {
  emid: string
  slotId: string
  onClose: () => void
}) {
  const [q, setQ] = useState('')
  const { data: members } = useMemberSearch(q)
  const assign = useAssignSlot(emid)

  return (
    <div className="border-t border-border-subtle bg-surface p-2">
      <input
        autoFocus
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="Search members…"
        className="w-full rounded-lg border border-border-subtle bg-surface-container px-3 py-1.5 text-sm"
      />
      <ul className="mt-2 max-h-40 overflow-y-auto">
        {(members ?? []).map((m) => (
          <li key={m.discord_id}>
            <button
              type="button"
              onClick={() =>
                assign.mutate(
                  { slotId, discordId: m.discord_id },
                  {
                    onSuccess: () => {
                      toast.success(`Assigned ${m.username}`)
                      onClose()
                    },
                    onError: () => toast.error('Could not assign member'),
                  },
                )
              }
              className="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-surface-container-high"
            >
              <img src={m.avatar_url || DEFAULT_AVATAR} alt="" className="h-5 w-5 rounded-full" />
              {m.username}
            </button>
          </li>
        ))}
        {members && members.length === 0 && (
          <li className="px-2 py-1 text-xs text-on-surface-variant">No matching members.</li>
        )}
      </ul>
    </div>
  )
}

// --- Standalone ORBAT route (deep-link / direct URL) -----------------------

export function OrbatSelectionPage() {
  const { id, emid } = useParams<{ id: string; emid: string }>()
  const { data: event, isLoading, isError, error } = useEvent(id)
  const dossier = event?.missions.find((m) => m.event_mission_id === emid)

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
          {emid && <OrbatSelector emid={emid} myState={dossier?.my_state} />}
        </div>
      </QueryState>
    </AuthGate>
  )
}
