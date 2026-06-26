import { useMemo, useState } from 'react'
import { toast } from 'sonner'
import { useQueryClient } from '@tanstack/react-query'
import { api } from '@/api/client'
import { AdminGate } from '@/components/AdminGate'
import { QueryState } from '@/components/QueryState'
import { MaterialIcon } from '@/components/MaterialIcon'
import { SplitPane, SplitPaneEmpty } from '@/components/ui/split-pane'
import { ListDetailItem } from '@/components/ui/list-detail-item'
import { Badge } from '@/components/ui/badge'
import { Switch } from '@/components/ui/switch'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import {
  useApprovals,
  useAuditLogs,
  useEvents,
  useMissions,
  usePersonnel,
} from '@/hooks/queries'
import {
  useApproveMission,
  useBanUser,
  useCreateEvent,
  useDeleteEvent,
  usePublishAnnouncement,
  useRejectMission,
  useUpdateUserRole,
} from '@/hooks/mutations'
import { formatLocalDateTime, formatShortDate, terrainLabel } from '@/lib/format'
import type { ApprovalRow, AuditLog, RosterRow } from '@/types/api'
import { cn } from '@/lib/utils'

const ROLE_OPTIONS = [
  { value: 'enlisted', label: 'Enlisted' },
  { value: 'leader', label: 'Leader' },
  { value: 'mission_maker', label: 'Mission Maker' },
  { value: 'admin', label: 'Admin' },
]

const inputClass =
  'w-full rounded-lg border border-outline-variant/40 bg-surface px-3 py-2 text-label-md outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/40 disabled:opacity-50'

const MONTH_NAMES = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
]
const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

/** Local YYYY-MM-DD key (avoids UTC drift from toISOString). */
function dayKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

/** Combine a calendar day with an "HH:MM" time string into a local Date. */
function combineDateTime(date: Date, time: string): Date {
  const [h, m] = time.split(':').map(Number)
  const d = new Date(date)
  d.setHours(h || 0, m || 0, 0, 0)
  return d
}

interface StagedMission {
  id: string
  title: string
}

export function EventManagerPage() {
  const { data: eventsData } = useEvents('all')
  const { data: missionsData } = useMissions('global')
  const createEvent = useCreateEvent()
  const deleteEvent = useDeleteEvent()
  const qc = useQueryClient()

  const today = useMemo(() => new Date(), [])
  const [viewMonth, setViewMonth] = useState(
    () => new Date(today.getFullYear(), today.getMonth(), 1),
  )
  const [selectedDate, setSelectedDate] = useState<Date>(() => new Date())

  // Schedule-operation form state.
  const [name, setName] = useState('')
  const [time, setTime] = useState('19:00')
  const [open, setOpen] = useState(true) // registration open
  const [staged, setStaged] = useState<StagedMission[]>([])
  const [attachOpen, setAttachOpen] = useState(false)
  // Frosted-glass create form (replaces the old form-beside-calendar layout).
  const [formOpen, setFormOpen] = useState(false)
  // Which existing operation on the selected day is targeted by Delete.
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null)

  const missions = missionsData?.data ?? []
  const events = useMemo(() => eventsData?.data ?? [], [eventsData])

  // Group operations by local calendar day for the indicators + day panel.
  const eventsByDay = useMemo(() => {
    const map = new Map<string, typeof events>()
    for (const e of events) {
      const key = dayKey(new Date(e.start_time))
      const list = map.get(key) ?? []
      list.push(e)
      map.set(key, list)
    }
    return map
  }, [events])

  const dayOps = eventsByDay.get(dayKey(selectedDate)) ?? []
  const availableMissions = missions.filter((m) => !staged.some((s) => s.id === m.id))

  // Build the calendar cells: leading blanks + each day, padded to whole weeks.
  const cells = useMemo(() => {
    const year = viewMonth.getFullYear()
    const month = viewMonth.getMonth()
    const leading = new Date(year, month, 1).getDay()
    const daysInMonth = new Date(year, month + 1, 0).getDate()
    const out: (Date | null)[] = []
    for (let i = 0; i < leading; i++) out.push(null)
    for (let d = 1; d <= daysInMonth; d++) out.push(new Date(year, month, d))
    while (out.length % 7 !== 0) out.push(null)
    return out
  }, [viewMonth])

  const shiftMonth = (delta: number) =>
    setViewMonth((m) => new Date(m.getFullYear(), m.getMonth() + delta, 1))

  const selectDay = (d: Date) => {
    setSelectedDate(d)
    setAttachOpen(false)
    const ops = eventsByDay.get(dayKey(d)) ?? []
    setSelectedEventId(ops[0]?.id ?? null)
  }

  const handlePublish = async () => {
    if (!time) {
      toast.error('Start time is required')
      return
    }
    const startIso = combineDateTime(selectedDate, time).toISOString()
    try {
      const created = (await createEvent.mutateAsync({
        start_time: startIso,
        name_override: name || undefined,
        registration_locked: !open,
      })) as { id: string }
      if (staged.length && created?.id) {
        await Promise.all(
          staged.map((m) =>
            api.post(`/events/${created.id}/missions`, {
              mission_id: m.id,
              start_time: startIso,
            }),
          ),
        )
        qc.invalidateQueries({ queryKey: ['events'] })
      }
      toast.success(
        staged.length
          ? `Event published with ${staged.length} mission${staged.length === 1 ? '' : 's'}`
          : 'Event published',
      )
      setName('')
      setStaged([])
      setOpen(true)
      setFormOpen(false)
    } catch {
      toast.error('Failed to publish event')
    }
  }

  const handleDelete = () => {
    if (!selectedEventId) return
    if (!window.confirm('Delete this operation? This cannot be undone.')) return
    deleteEvent.mutate(selectedEventId, {
      onSuccess: () => {
        toast.success('Operation deleted')
        setSelectedEventId(null)
      },
      onError: () => toast.error('Failed to delete operation'),
    })
  }

  return (
    <AdminGate>
      <div className="mx-auto h-full w-full max-w-5xl">
        {/* Header — primary action opens the frosted create form over the calendar. */}
        <div className="mb-6 flex flex-wrap items-center justify-between gap-4">
          <div>
            <h1 className="text-headline-md tracking-tight text-on-surface">Operations Calendar</h1>
            <p className="mt-1 text-sm text-on-surface-variant">
              Schedule operations for any day. ORBATs generate from each attached mission.
            </p>
          </div>
          <button
            type="button"
            onClick={() => setFormOpen(true)}
            className="flex items-center gap-2 rounded-full bg-action px-6 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
          >
            <MaterialIcon name="add" className="text-[18px]" />
            Schedule Operation
          </button>
        </div>

        <div className="grid grid-cols-1 gap-8 lg:grid-cols-12">
          {/* ── Left: tactical calendar ─────────────────────────────── */}
          <div className="lg:col-span-8">
            <div className="mb-6 flex items-center justify-between">
              <h2 className="text-2xl font-bold tracking-tight text-white">
                {MONTH_NAMES[viewMonth.getMonth()]} {viewMonth.getFullYear()}
              </h2>
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() => shiftMonth(-1)}
                  aria-label="Previous month"
                  className="flex size-9 items-center justify-center rounded-full text-on-surface-variant transition hover:bg-white/5 hover:text-white"
                >
                  <MaterialIcon name="chevron_left" />
                </button>
                <button
                  type="button"
                  onClick={() => shiftMonth(1)}
                  aria-label="Next month"
                  className="flex size-9 items-center justify-center rounded-full text-on-surface-variant transition hover:bg-white/5 hover:text-white"
                >
                  <MaterialIcon name="chevron_right" />
                </button>
              </div>
            </div>

            <div className="mb-2 grid grid-cols-7 gap-1">
              {WEEKDAYS.map((w) => (
                <div
                  key={w}
                  className="py-2 text-center font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase"
                >
                  {w}
                </div>
              ))}
            </div>

            <div className="grid grid-cols-7 gap-1">
              {cells.map((cell, i) => {
                if (!cell) return <div key={`pad-${i}`} />
                const isSelected = dayKey(cell) === dayKey(selectedDate)
                const isToday = dayKey(cell) === dayKey(today)
                const ops = eventsByDay.get(dayKey(cell)) ?? []
                return (
                  <button
                    key={dayKey(cell)}
                    type="button"
                    onClick={() => selectDay(cell)}
                    className={cn(
                      'flex aspect-square flex-col items-center justify-center gap-1.5 rounded-xl text-sm transition',
                      isSelected
                        ? 'bg-action text-on-action shadow-[0_0_20px_rgba(59,130,246,0.4)]'
                        : 'text-on-surface hover:bg-white/5',
                      !isSelected && isToday && 'font-bold text-primary',
                    )}
                  >
                    <span>{cell.getDate()}</span>
                    <span className="flex h-1 items-center gap-0.5">
                      {ops.slice(0, 3).map((op) => (
                        <span
                          key={op.id}
                          className={cn(
                            'h-1 w-4 rounded-full',
                            isSelected ? 'bg-white/70' : 'bg-primary/50',
                          )}
                        />
                      ))}
                    </span>
                  </button>
                )
              })}
            </div>
          </div>

          {/* ── Right: scheduled operations for the selected day ─────── */}
          <div className="lg:col-span-4 lg:border-l lg:border-white/5 lg:pl-8">
            <p className="font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
              {selectedDate.toLocaleDateString(undefined, {
                weekday: 'short',
                month: 'short',
                day: 'numeric',
                year: 'numeric',
              })}
            </p>
            <h2 className="mt-1 mb-4 text-lg font-bold tracking-tight text-white">
              Scheduled Operations
            </h2>

            {dayOps.length === 0 ? (
              <p className="text-sm text-on-surface-variant">
                No operations scheduled.{' '}
                <button
                  type="button"
                  onClick={() => setFormOpen(true)}
                  className="text-primary hover:underline"
                >
                  Schedule one.
                </button>
              </p>
            ) : (
              <div className="space-y-2">
                {dayOps.map((op) => (
                  <button
                    key={op.id}
                    type="button"
                    onClick={() => setSelectedEventId(op.id)}
                    className={cn(
                      'flex w-full items-center justify-between gap-3 rounded-xl border px-4 py-3 text-left transition',
                      op.id === selectedEventId
                        ? 'border-primary/60 bg-primary/15'
                        : 'border-white/10 hover:bg-white/[0.03]',
                    )}
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium text-on-surface">
                        {op.name_override || 'Untitled Operation'}
                      </p>
                      <p className="mt-0.5 font-mono text-xs text-on-surface-variant">
                        {formatLocalDateTime(op.start_time)} · {op.mission_count} mission
                        {op.mission_count === 1 ? '' : 's'} · {op.filled}/{op.total_slots}
                      </p>
                    </div>
                    <Badge variant={op.registration_locked ? 'neutral' : 'success'}>
                      {op.registration_locked ? 'Locked' : 'Open'}
                    </Badge>
                  </button>
                ))}
              </div>
            )}

            {selectedEventId && (
              <button
                type="button"
                onClick={handleDelete}
                disabled={deleteEvent.isPending}
                className="mt-4 w-full rounded-full py-3 text-sm font-medium text-error-alert transition hover:bg-error-alert/10 disabled:cursor-not-allowed disabled:opacity-40"
              >
                Delete Selected Operation
              </button>
            )}
          </div>
        </div>

        {/* Frosted create form — overlays the calendar, preserving context. */}
        <Dialog open={formOpen} onOpenChange={setFormOpen}>
          <DialogContent
            title="Schedule Operation"
            description={selectedDate.toLocaleDateString(undefined, {
              weekday: 'long',
              month: 'long',
              day: 'numeric',
              year: 'numeric',
            })}
          >
            {/* Start time */}
            <label className="flex w-fit items-center gap-2 rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface focus-within:ring-1 focus-within:ring-primary/50">
              <MaterialIcon name="schedule" className="text-base text-on-surface-variant" />
              <input
                type="time"
                value={time}
                onChange={(e) => setTime(e.target.value)}
                className="bg-transparent text-on-surface outline-none [color-scheme:dark]"
              />
            </label>

            {/* Operation name */}
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Operation name (e.g. Twin Theaters)"
              className="mt-3 w-full rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50"
            />

            {/* Mission multi-select */}
            <div className="mt-6">
              <p className="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                Missions
              </p>
              <div className="space-y-2">
                {staged.map((m) => (
                  <div
                    key={m.id}
                    className="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3"
                  >
                    <MaterialIcon name="map" className="text-on-surface-variant" />
                    <span className="flex-1 text-sm text-on-surface">{m.title}</span>
                    <button
                      type="button"
                      onClick={() => setStaged((prev) => prev.filter((s) => s.id !== m.id))}
                      aria-label={`Remove ${m.title}`}
                      className="flex size-7 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
                    >
                      <MaterialIcon name="close" className="text-base" />
                    </button>
                  </div>
                ))}
                {staged.length === 0 && (
                  <p className="px-1 text-sm text-on-surface-variant/70">No missions attached yet.</p>
                )}
              </div>

              {/* + Attach Mission dropdown */}
              <div className="relative mt-2">
                <button
                  type="button"
                  onClick={() => setAttachOpen((o) => !o)}
                  className="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5"
                >
                  <MaterialIcon name="add" className="text-base" />
                  Attach Mission
                </button>
                {attachOpen && (
                  <div className="absolute z-10 mt-2 max-h-64 w-full overflow-y-auto rounded-xl border border-white/10 bg-surface-container-high/95 p-1 shadow-2xl backdrop-blur-xl">
                    {availableMissions.length === 0 ? (
                      <p className="px-3 py-2 text-sm text-on-surface-variant">
                        No more missions in the library.
                      </p>
                    ) : (
                      availableMissions.map((m) => (
                        <button
                          key={m.id}
                          type="button"
                          onClick={() => {
                            setStaged((prev) => [...prev, { id: m.id, title: m.title }])
                            setAttachOpen(false)
                          }}
                          className="flex w-full items-center justify-between gap-2 rounded-lg px-3 py-2 text-left text-sm text-on-surface transition hover:bg-white/5"
                        >
                          <span className="truncate">{m.title}</span>
                          <span className="shrink-0 font-mono text-xs text-on-surface-variant">
                            {terrainLabel(m.terrain)}
                          </span>
                        </button>
                      ))
                    )}
                  </div>
                )}
              </div>
            </div>

            {/* Registration status segmented control */}
            <div className="mt-6">
              <p className="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                Registration
              </p>
              <div className="inline-flex rounded-full bg-white/5 p-1">
                {([true, false] as const).map((isOpen) => (
                  <button
                    key={String(isOpen)}
                    type="button"
                    onClick={() => setOpen(isOpen)}
                    className={cn(
                      'rounded-full px-6 py-2 text-sm font-medium transition',
                      open === isOpen
                        ? isOpen
                          ? 'bg-success/20 text-success'
                          : 'bg-white/10 text-on-surface'
                        : 'text-on-surface-variant hover:text-on-surface',
                    )}
                  >
                    {isOpen ? 'Open' : 'Locked'}
                  </button>
                ))}
              </div>
            </div>

            {/* Publish */}
            <button
              type="button"
              onClick={handlePublish}
              disabled={createEvent.isPending}
              className="mt-8 w-full rounded-full bg-action py-4 text-base font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
            >
              {createEvent.isPending ? 'Publishing…' : 'Publish Event'}
            </button>
          </DialogContent>
        </Dialog>
      </div>
    </AdminGate>
  )
}

type ReviewStatus = 'pending' | 'approved' | 'rejected'

// Mock approved/rejected history so the audit trail + revoke flow can be built
// before list endpoints for those states exist. Pending stays on live data.
const MOCK_APPROVED: ApprovalRow[] = [
  {
    mission_id: 'apr-1',
    title: 'Operation Iron Veil',
    terrain: 'everon',
    author_id: 'u-mike',
    author_name: 'Mission Maker Mike',
    submitted_at: '2026-06-12T18:00:00Z',
  },
  {
    mission_id: 'apr-2',
    title: 'Dawnbreaker',
    terrain: 'arland',
    author_id: 'u-sam',
    author_name: 'Sam',
    submitted_at: '2026-06-08T12:30:00Z',
  },
]

const MOCK_REJECTED: ApprovalRow[] = [
  {
    mission_id: 'rej-1',
    title: 'Night of the Long Knives',
    terrain: 'everon',
    author_id: 'u-mike',
    author_name: 'Mission Maker Mike',
    submitted_at: '2026-06-05T09:15:00Z',
  },
]

export function MissionApprovalsPage() {
  const { data, isLoading, isError, error } = useApprovals()
  const approve = useApproveMission()
  const reject = useRejectMission()
  const pendingRows = data?.data ?? []

  const [tab, setTab] = useState<ReviewStatus>('pending')
  const [selectedId, setSelectedId] = useState<string | null>(null)

  const rows =
    tab === 'pending' ? pendingRows : tab === 'approved' ? MOCK_APPROVED : MOCK_REJECTED
  const selected = rows.find((r) => r.mission_id === selectedId) ?? rows[0] ?? null

  const TABS: { value: ReviewStatus; label: string; count?: number }[] = [
    { value: 'pending', label: 'Pending', count: pendingRows.length },
    { value: 'approved', label: 'Approved', count: MOCK_APPROVED.length },
    { value: 'rejected', label: 'Rejected', count: MOCK_REJECTED.length },
  ]

  return (
    <AdminGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <SplitPane
          masterHeader={
            <div className="flex w-full items-center rounded-full bg-white/5 p-1">
              {TABS.map((t) => (
                <button
                  key={t.value}
                  type="button"
                  onClick={() => {
                    setTab(t.value)
                    setSelectedId(null)
                  }}
                  className={cn(
                    'flex-1 rounded-full py-2 text-center text-label-sm font-medium whitespace-nowrap transition-all',
                    tab === t.value
                      ? 'bg-action text-on-action shadow-md'
                      : 'text-white/50 hover:text-white',
                  )}
                >
                  {t.label}
                  {t.count != null && t.count > 0 && (
                    <span className="ml-1.5 font-mono text-code-md opacity-70">({t.count})</span>
                  )}
                </button>
              ))}
            </div>
          }
          master={
            rows.length === 0 ? (
              <p className="px-1 py-4 text-label-md text-on-surface-variant">
                {tab === 'pending'
                  ? 'No pending approvals.'
                  : `No ${tab} missions.`}
              </p>
            ) : (
              rows.map((r) => {
                const active = selected?.mission_id === r.mission_id
                return (
                  <button
                    key={r.mission_id}
                    type="button"
                    onClick={() => setSelectedId(r.mission_id)}
                    className={cn(
                      'group w-full rounded-r-xl border-l-4 px-4 py-3 text-left transition-all duration-200',
                      active
                        ? 'border-primary bg-primary/15 shadow-[inset_0_0_18px_rgba(173,198,255,0.15)]'
                        : 'border-transparent hover:bg-white/[0.03]',
                    )}
                  >
                    <span
                      className={cn(
                        'font-mono text-code-md',
                        active ? 'text-primary' : 'text-outline',
                      )}
                    >
                      [{formatShortDate(r.submitted_at)}]
                    </span>
                    <h3
                      className={cn(
                        'mt-1 truncate text-label-md font-semibold',
                        active ? 'text-on-surface' : 'text-on-surface-variant group-hover:text-on-surface',
                      )}
                    >
                      {r.title}
                    </h3>
                    <p className="mt-0.5 truncate text-label-sm text-on-surface-variant">
                      By {r.author_name} · {terrainLabel(r.terrain)}
                    </p>
                  </button>
                )
              })
            )
          }
          detail={
            selected ? (
              <ReviewInspector
                key={`${tab}-${selected.mission_id}`}
                selected={selected}
                status={tab}
                approve={approve}
                reject={reject}
              />
            ) : (
              <SplitPaneEmpty
                icon={<MaterialIcon name="task_alt" className="text-4xl" />}
                message={
                  tab === 'pending'
                    ? 'Queue clear — no pending approvals.'
                    : `No ${tab} missions to show.`
                }
              />
            )
          }
        />
      </QueryState>
    </AdminGate>
  )
}

// ─── Review Inspector ──────────────────────────────────────────────────────
// GitHub-PR-meets-chat review surface for one pending mission. Cinematic header,
// briefing + stats, a Git-backed activity timeline, and a sticky approve/reject
// bar. The mission stats and timeline are mocked until the Git versioning
// backend (commits against mission.sqm) and a review-comments API land — only
// the approve/reject actions are wired to live mutations.

type FeedItem =
  | { kind: 'commit'; hash: string; branch: string; author: string; highlight?: boolean }
  | { kind: 'comment'; author: string; role: 'admin' | 'author'; body: string }

const MOCK_FEED: FeedItem[] = [
  { kind: 'commit', hash: '7f8a9b2', branch: 'main', author: 'Mission Maker Mike' },
  {
    kind: 'comment',
    author: 'Admin',
    role: 'admin',
    body: 'Hey Mike, OPFOR count is way too high in the northern town. Please thin them out.',
  },
  {
    kind: 'commit',
    hash: 'a3c4d5e',
    branch: 'main',
    author: 'Mission Maker Mike',
    highlight: true,
  },
  {
    kind: 'comment',
    author: 'Mission Maker Mike',
    role: 'author',
    body: 'Fixed the OPFOR spawns and reduced armor presence.',
  },
]

function ReviewInspector({
  selected,
  status,
  approve,
  reject,
}: {
  selected: ApprovalRow
  status: ReviewStatus
  approve: ReturnType<typeof useApproveMission>
  reject: ReturnType<typeof useRejectMission>
}) {
  const [feed, setFeed] = useState<FeedItem[]>(MOCK_FEED)
  const [draft, setDraft] = useState('')

  const postComment = () => {
    const body = draft.trim()
    if (!body) return
    setFeed((prev) => [...prev, { kind: 'comment', author: 'You', role: 'admin', body }])
    setDraft('')
  }

  return (
    <div className="flex min-h-full flex-col">
      {/* Cinematic header */}
      <div className="relative h-64 shrink-0 bg-topo-map bg-cover bg-center">
        <div className="absolute inset-0 bg-gradient-to-t from-surface-glass to-transparent" />
        <div className="absolute inset-x-0 bottom-0 p-8">
          <div className="mb-3 flex flex-wrap items-center gap-2">
            {status === 'approved' && (
              <span className="rounded-full border border-success/40 bg-success/20 px-3 py-1 text-label-sm font-medium text-success backdrop-blur-md">
                Published
              </span>
            )}
            {status === 'rejected' && (
              <span className="rounded-full border border-error-alert/40 bg-error-alert/20 px-3 py-1 text-label-sm font-medium text-error-alert backdrop-blur-md">
                Rejected
              </span>
            )}
            <span className="rounded-full bg-white/10 px-3 py-1 text-label-sm text-on-surface backdrop-blur-md">
              {terrainLabel(selected.terrain)}
            </span>
            <span className="rounded-full bg-white/10 px-3 py-1 text-label-sm text-on-surface backdrop-blur-md">
              {selected.author_name}
            </span>
            <span className="rounded-full bg-white/10 px-3 py-1 font-mono text-code-md text-on-surface backdrop-blur-md">
              {formatLocalDateTime(selected.submitted_at)}
            </span>
          </div>
          <h1 className="text-headline-lg text-on-surface drop-shadow-lg">{selected.title}</h1>
        </div>
      </div>

      {/* Briefing + stats */}
      <div className="px-8 py-7">
        <p className="text-body-md text-on-surface-variant">
          Combined-arms assault across contested farmland. BLUFOR pushes from the south to seize the
          northern town while a mechanized OPFOR garrison holds the objective. Review the latest push
          before approving for the live mission database.
        </p>
        <div className="mt-6 grid grid-cols-3 gap-3">
          <StatTile label="BLUFOR Slots" value="32" />
          <StatTile label="OPFOR Type" value="Mechanized" />
          <StatTile label="Est. Duration" value="~90 min" />
        </div>

        {/* Deep review */}
        <button
          type="button"
          onClick={() => toast.message('Tactical Planner (2D editor) is coming soon')}
          className="mt-6 flex w-full items-center justify-center gap-2 rounded-xl border border-primary/40 bg-primary/10 py-3.5 text-label-md font-medium text-primary transition hover:bg-primary/20"
        >
          <MaterialIcon name="search" className="text-[20px]" />
          Launch Tactical Planner for Deep Review
        </button>

        {/* Activity & version history */}
        <div className="mt-8">
          <h2 className="mb-4 text-label-md font-semibold tracking-wide text-on-surface uppercase">
            Activity &amp; Version History
          </h2>
          <div className="flex flex-col gap-3">
            {feed.map((item, i) =>
              item.kind === 'commit' ? (
                <CommitRow key={i} item={item} />
              ) : (
                <ChatBubble key={i} item={item} />
              ),
            )}
          </div>

          {/* Comment input */}
          <div className="mt-5 flex items-center gap-2 rounded-full border border-white/10 bg-white/5 py-1.5 pr-1.5 pl-5 backdrop-blur-md focus-within:border-primary/40">
            <input
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  postComment()
                }
              }}
              placeholder="Leave a review comment…"
              className="flex-1 bg-transparent text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none"
            />
            <button
              type="button"
              onClick={postComment}
              aria-label="Send comment"
              className="flex size-9 items-center justify-center rounded-full bg-primary text-on-primary transition hover:bg-primary/80"
            >
              <MaterialIcon name="arrow_upward" className="text-[20px]" />
            </button>
          </div>
        </div>
      </div>

      {/* Sticky action bar — adapts to mission status */}
      <div className="sticky bottom-0 mt-auto flex items-center justify-end gap-3 border-t border-white/5 bg-surface-container/40 p-6 backdrop-blur-xl">
        {status === 'approved' ? (
          // Already live — let admins yank it back off the server.
          <button
            type="button"
            onClick={() =>
              toast.success('Mission unpublished — pulled from the live server')
            }
            className="rounded-full border border-error-alert/50 bg-error-alert/10 px-7 py-3 text-label-md font-bold text-error-alert shadow-[0_0_20px_rgba(239,68,68,0.2)] transition hover:bg-error-alert/20"
          >
            Revoke Approval &amp; Unpublish
          </button>
        ) : status === 'rejected' ? (
          // Give a rejected mission a second chance.
          <button
            type="button"
            onClick={() => toast.success('Mission re-approved & published')}
            className="rounded-full bg-emerald-600 px-7 py-3 text-label-md font-bold text-white shadow-[0_0_20px_rgba(16,185,129,0.3)] transition hover:bg-emerald-500"
          >
            Approve &amp; Publish
          </button>
        ) : (
          <>
            <button
              type="button"
              disabled={reject.isPending}
              onClick={() =>
                reject.mutate(
                  { id: selected.mission_id },
                  {
                    onSuccess: () => toast.success('Changes requested — returned to author'),
                    onError: () => toast.error('Request failed'),
                  },
                )
              }
              className="rounded-full border border-tactical-yellow/40 bg-tactical-yellow/5 px-6 py-3 text-label-md font-medium text-tactical-yellow transition hover:bg-tactical-yellow/10 disabled:opacity-50"
            >
              Request Changes
            </button>
            <button
              type="button"
              disabled={approve.isPending}
              onClick={() =>
                approve.mutate(selected.mission_id, {
                  onSuccess: () => toast.success('Mission approved & published'),
                  onError: () => toast.error('Approval failed'),
                })
              }
              className="rounded-full bg-emerald-600 px-7 py-3 text-label-md font-bold text-white shadow-[0_0_20px_rgba(16,185,129,0.3)] transition hover:bg-emerald-500 disabled:opacity-50"
            >
              Approve &amp; Publish
            </button>
          </>
        )}
      </div>
    </div>
  )
}

function StatTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
      <p className="font-mono text-code-md text-on-surface-variant uppercase">{label}</p>
      <p className="mt-1 text-headline-sm text-on-surface">{value}</p>
    </div>
  )
}

/** A Git commit event in the timeline, with a View Diff affordance. */
function CommitRow({ item }: { item: Extract<FeedItem, { kind: 'commit' }> }) {
  return (
    <div
      className={cn(
        'flex items-center gap-3 rounded-lg border px-4 py-2.5',
        item.highlight
          ? 'border-success/30 bg-success/5 shadow-[0_0_16px_rgba(16,185,129,0.12)]'
          : 'border-transparent',
      )}
    >
      <MaterialIcon
        name="commit"
        className={cn('text-[18px]', item.highlight ? 'text-success' : 'text-outline')}
      />
      <p className="flex-1 font-mono text-code-md text-on-surface-variant">
        Commit{' '}
        <span className={item.highlight ? 'text-success' : 'text-on-surface'}>{item.hash}</span>{' '}
        pushed to <span className="text-on-surface">{item.branch}</span> by {item.author}
      </p>
      <button
        type="button"
        onClick={() => toast.message(`Diff for ${item.hash} — mission.sqm (coming soon)`)}
        className="font-code-md rounded bg-white/5 px-2 py-1 text-xs text-on-surface-variant transition hover:bg-white/10 hover:text-on-surface"
      >
        View Diff
      </button>
    </div>
  )
}

/** A chat-style review comment bubble. */
function ChatBubble({ item }: { item: Extract<FeedItem, { kind: 'comment' }> }) {
  const isAdmin = item.role === 'admin'
  return (
    <div className={cn('flex', isAdmin ? 'justify-end' : 'justify-start')}>
      <div className={cn('max-w-[80%]', isAdmin ? 'items-end' : 'items-start')}>
        <p
          className={cn(
            'mb-1 text-label-sm text-on-surface-variant',
            isAdmin ? 'text-right' : 'text-left',
          )}
        >
          {item.author}
        </p>
        <div
          className={cn(
            'rounded-2xl px-4 py-2.5 text-body-md',
            isAdmin
              ? 'rounded-br-md bg-primary/20 text-on-surface'
              : 'rounded-bl-md bg-white/5 text-on-surface',
          )}
        >
          {item.body}
        </div>
      </div>
    </div>
  )
}

/** Initials avatar fallback (RosterRow carries no avatar URL). */
function initials(name: string): string {
  return name
    .split(/[\s_.-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join('')
}

function Avatar({ name, className }: { name: string; className?: string }) {
  return (
    <span
      className={cn(
        'flex shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-primary/40 to-tertiary/30 font-semibold text-on-surface',
        className,
      )}
    >
      {initials(name) || '??'}
    </span>
  )
}

export function PersonnelRosterPage() {
  const [q, setQ] = useState('')
  const { data, isLoading, isError, error } = usePersonnel(q || undefined)
  const updateRole = useUpdateUserRole()
  const banUser = useBanUser()
  const users = data?.data ?? []
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const selected = users.find((u) => u.discord_id === selectedId) ?? null

  return (
    <AdminGate>
      <div className="flex h-full w-full flex-1 overflow-hidden bg-surface-glass backdrop-blur-xl">
        {/* ── Left: data table (70%) ──────────────────────────────── */}
        <div className="flex min-w-0 flex-[7] flex-col border-r border-white/10">
          {/* Header */}
          <div className="border-b border-white/5 p-6">
            <div className="flex flex-wrap items-center justify-between gap-4">
              <h1 className="text-headline-lg text-on-surface">Personnel Roster</h1>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => toast.message('Sort options coming soon')}
                  className="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                >
                  <MaterialIcon name="swap_vert" className="text-[18px]" />
                  Sort
                </button>
                <button
                  type="button"
                  onClick={() => toast.message('Filter options coming soon')}
                  className="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
                >
                  <MaterialIcon name="filter_list" className="text-[18px]" />
                  Filter
                </button>
              </div>
            </div>
            <div className="relative mt-4">
              <MaterialIcon
                name="search"
                className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-[18px] text-on-surface-variant"
              />
              <input
                type="search"
                placeholder="Search Discord ID or Arma Name…"
                value={q}
                onChange={(e) => setQ(e.target.value)}
                className="w-full max-w-md rounded-full border border-white/10 bg-black/20 py-2.5 pr-3 pl-9 text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:border-primary/50"
              />
            </div>
          </div>

          {/* Table */}
          <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto">
            <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
              {users.length === 0 ? (
                <p className="p-6 text-on-surface-variant">No users found.</p>
              ) : (
                <table className="w-full text-label-md">
                  <thead className="sticky top-0 z-10 bg-surface-container-high/80 text-label-sm text-on-surface-variant uppercase backdrop-blur-md">
                    <tr>
                      <th className="px-4 py-3 text-left font-medium">User</th>
                      <th className="px-4 py-3 text-left font-medium">Arma Character</th>
                      <th className="px-4 py-3 text-left font-medium">Rank</th>
                      <th className="px-4 py-3 text-right font-medium">Warnings</th>
                      <th className="px-4 py-3 text-right font-medium">Status</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-white/5">
                    {users.map((u) => {
                      const active = selected?.discord_id === u.discord_id
                      return (
                        <tr
                          key={u.discord_id}
                          onClick={() => setSelectedId(u.discord_id)}
                          className={cn(
                            'cursor-pointer transition-colors',
                            active ? 'bg-primary/15' : 'hover:bg-white/[0.03]',
                          )}
                        >
                          <td
                            className={cn(
                              'border-l-4 px-4 py-3',
                              active ? 'border-primary' : 'border-transparent',
                            )}
                          >
                            <div className="flex items-center gap-3">
                              <Avatar name={u.discord_handle || u.username} className="size-8 text-xs" />
                              <span className="truncate text-on-surface">
                                {u.discord_handle || u.username}
                              </span>
                            </div>
                          </td>
                          <td className="px-4 py-3 text-on-surface-variant">
                            {u.arma_character || u.arma_id || 'Unlinked'}
                          </td>
                          <td className="px-4 py-3">
                            <span className="text-label-sm text-on-surface-variant uppercase">
                              {u.role}
                            </span>
                          </td>
                          <td
                            className={cn(
                              'px-4 py-3 text-right font-mono',
                              u.warnings > 0 ? 'text-tactical-yellow' : 'text-on-surface-variant',
                            )}
                          >
                            {u.warnings}
                          </td>
                          <td className="px-4 py-3 text-right">
                            {u.is_banned ? (
                              <Badge variant="error">Banned</Badge>
                            ) : (
                              <Badge variant="success">Active</Badge>
                            )}
                          </td>
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              )}
            </QueryState>
          </div>
        </div>

        {/* ── Right: fixed dossier (30%) ──────────────────────────── */}
        <aside className="flex min-w-0 flex-[3] flex-col bg-surface-container-lowest/40">
          {selected ? (
            <PersonnelDossier
              key={selected.discord_id}
              user={selected}
              updateRole={updateRole}
              banUser={banUser}
            />
          ) : (
            <div className="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center text-on-surface-variant">
              <MaterialIcon name="badge" className="text-4xl opacity-50" />
              <p className="text-label-md">Select personnel to view dossier</p>
            </div>
          )}
        </aside>
      </div>
    </AdminGate>
  )
}

function PersonnelDossier({
  user,
  updateRole,
  banUser,
}: {
  user: RosterRow
  updateRole: ReturnType<typeof useUpdateUserRole>
  banUser: ReturnType<typeof useBanUser>
}) {
  const [role, setRole] = useState(user.role)
  const [editingRole, setEditingRole] = useState(false)
  const [banned, setBanned] = useState(user.is_banned)

  const handleRoleChange = (next: string) => {
    setRole(next)
    updateRole.mutate(
      { discordId: user.discord_id, role: next },
      {
        onSuccess: () => toast.success('Role updated'),
        onError: () => {
          toast.error('Failed to update role')
          setRole(user.role)
        },
      },
    )
  }

  const handleBan = () => {
    const reason = window.prompt('Ban reason (optional):')
    if (reason === null) return
    banUser.mutate(
      { discordId: user.discord_id, reason: reason || undefined },
      {
        onSuccess: () => {
          toast.success('Personnel banned')
          setBanned(true)
        },
        onError: () => toast.error('Ban failed'),
      },
    )
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-6">
        {/* Profile header */}
        <div className="flex flex-col items-center text-center">
          <Avatar name={user.discord_handle || user.username} className="size-20 text-xl" />
          <h2 className="mt-4 text-headline-sm text-on-surface">
            {user.discord_handle || user.username}
          </h2>
          <p className="mt-1 font-mono text-code-md text-on-surface-variant">{user.discord_id}</p>
          <p className="mt-2 text-label-md text-on-surface-variant">
            {user.arma_character || user.arma_id || 'Unlinked Arma identity'}
          </p>
        </div>

        {/* Service telemetry */}
        <div className="mt-6 grid grid-cols-2 gap-3">
          <Stat label="Deployments" value="—" />
          <Stat label="Current Rank" value={role.toUpperCase()} />
          <Stat label="Warnings" value={String(user.warnings)} />
          <Stat label="Status" value={banned ? 'Banned' : 'Active'} />
        </div>

        {/* Inline role editor */}
        {editingRole && (
          <div className="mt-4">
            <label className="mb-1 block text-label-sm text-on-surface-variant uppercase">Role</label>
            <select
              value={role}
              onChange={(e) => handleRoleChange(e.target.value)}
              className={inputClass}
            >
              {ROLE_OPTIONS.map((r) => (
                <option key={r.value} value={r.value}>
                  {r.label}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      {/* Docked actions */}
      <div className="flex flex-col gap-2 border-t border-white/10 p-6">
        <button
          type="button"
          onClick={() => setEditingRole((v) => !v)}
          className="flex items-center justify-center gap-2 rounded-lg border border-white/10 py-2.5 text-label-md text-on-surface transition hover:bg-white/5"
        >
          <MaterialIcon name="manage_accounts" className="text-[18px]" />
          Edit Roles
        </button>
        <button
          type="button"
          onClick={() => toast.message('Warning issued (mock)')}
          className="flex items-center justify-center gap-2 rounded-lg border border-tactical-yellow/30 py-2.5 text-label-md text-tactical-yellow transition hover:bg-tactical-yellow/10"
        >
          <MaterialIcon name="warning" className="text-[18px]" />
          Issue Warning
        </button>
        <button
          type="button"
          onClick={handleBan}
          disabled={banned || banUser.isPending}
          className="flex items-center justify-center gap-2 rounded-lg bg-error-alert/15 py-2.5 text-label-md font-medium text-error-alert transition hover:bg-error-alert/25 disabled:cursor-not-allowed disabled:opacity-40"
        >
          <MaterialIcon name="gavel" className="text-[18px]" />
          {banned ? 'Personnel Banned' : 'Ban Personnel'}
        </button>
      </div>
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-outline-variant/30 bg-surface-variant/30 p-3">
      <span className="text-label-sm text-on-surface-variant uppercase">{label}</span>
      <p className="mt-1 font-mono text-headline-sm text-on-surface">{value}</p>
    </div>
  )
}

// ─── Comms Broadcaster (CMS) ───────────────────────────────────────────────
// macOS-Notes-style CMS: a document list on the left, a distraction-free editor
// on the right. The document list is mocked until a CMS list endpoint exists;
// publishing an announcement-category post is wired to the live announcement
// mutation, while SOP/draft saves are local for now.

type DocCategory = 'announcement' | 'sop' | 'event' | 'modpack' | 'important'
type DocStatus = 'published' | 'draft'

interface ContentDoc {
  id: string
  title: string
  category: DocCategory
  status: DocStatus
  date: string
  body: string
}

const CATEGORY_OPTIONS: { value: DocCategory; label: string }[] = [
  { value: 'announcement', label: 'Announcement' },
  { value: 'sop', label: 'SOP' },
  { value: 'event', label: 'Community Event' },
  { value: 'modpack', label: 'Modpack Update' },
  { value: 'important', label: 'Important' },
]

// Map a doc category onto the announcement `tag` enum (SOP has no equivalent).
const CATEGORY_TAG: Partial<Record<DocCategory, string>> = {
  announcement: 'update',
  event: 'event',
  modpack: 'modpack_update',
  important: 'important',
}

const MOCK_DOCS: ContentDoc[] = [
  {
    id: 'd1',
    title: 'Operation Blue Storm Briefing',
    category: 'announcement',
    status: 'published',
    date: '2026-06-18',
    body: 'All units, Operation Blue Storm kicks off Saturday at 1900Z. BLUFOR will stage at the southern airfield...\n\nReview your ORBAT assignments and ensure your modpack is current.',
  },
  {
    id: 'd2',
    title: 'SOP: Armor Tactics',
    category: 'sop',
    status: 'published',
    date: '2026-06-12',
    body: '# Armor Doctrine\n\nNever advance armor without infantry support. Maintain hull-down positions where possible and...',
  },
  {
    id: 'd3',
    title: 'Modpack v2.4.1 Changelog',
    category: 'modpack',
    status: 'draft',
    date: '2026-06-20',
    body: 'Draft notes for the upcoming modpack bump:\n- Added RHS Status Quo\n- Removed deprecated optics pack',
  },
]

export function ContentManagerPage() {
  const publish = usePublishAnnouncement()
  const [docs, setDocs] = useState<ContentDoc[]>(MOCK_DOCS)
  const [selectedId, setSelectedId] = useState<string | null>(MOCK_DOCS[0]?.id ?? null)
  const selected = docs.find((d) => d.id === selectedId) ?? null

  const newPost = () => {
    const doc: ContentDoc = {
      id: `new-${Date.now()}`,
      title: 'Untitled Post',
      category: 'announcement',
      status: 'draft',
      date: new Date().toISOString().slice(0, 10),
      body: '',
    }
    setDocs((prev) => [doc, ...prev])
    setSelectedId(doc.id)
  }

  const saveDoc = (updated: ContentDoc) =>
    setDocs((prev) => prev.map((d) => (d.id === updated.id ? updated : d)))

  return (
    <AdminGate>
      <div className="relative h-full w-full overflow-hidden">
        {/* Global topo-map background */}
        <div className="bg-topo-map bg-grid-overlay absolute inset-0 z-0" />
        <div className="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
          <SplitPane
            transparent
            masterWidth="20rem"
            masterHeader={
              <>
                <h1 className="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                  Comms Broadcaster
                </h1>
                <button
                  type="button"
                  onClick={newPost}
                  className="flex shrink-0 items-center gap-1.5 rounded-full border border-white/10 px-3 py-1.5 text-label-sm text-on-surface transition hover:bg-white/5"
                >
                  <MaterialIcon name="add" className="text-[18px]" />
                  New
                </button>
              </>
            }
            master={docs.map((d) => (
              <ListDetailItem
                key={d.id}
                active={d.id === selected?.id}
                onClick={() => setSelectedId(d.id)}
                meta={d.date}
                title={d.title || 'Untitled Post'}
                trailing={
                  <Badge variant={d.status === 'published' ? 'success' : 'warning'}>
                    {d.status === 'published' ? 'Published' : 'Draft'}
                  </Badge>
                }
              />
            ))}
            detail={
              selected ? (
                <ContentEditor key={selected.id} doc={selected} publish={publish} onChange={saveDoc} />
              ) : (
                <SplitPaneEmpty
                  icon={<MaterialIcon name="edit_note" className="text-4xl" />}
                  message="Select a post or create a new one."
                />
              )
            }
          />
        </div>
      </div>
    </AdminGate>
  )
}

const MD_TOOLS: { icon: string; label: string }[] = [
  { icon: 'format_bold', label: 'Bold' },
  { icon: 'format_italic', label: 'Italic' },
  { icon: 'link', label: 'Link' },
  { icon: 'format_list_bulleted', label: 'List' },
  { icon: 'image', label: 'Image' },
]

function ContentEditor({
  doc,
  publish,
  onChange,
}: {
  doc: ContentDoc
  publish: ReturnType<typeof usePublishAnnouncement>
  onChange: (doc: ContentDoc) => void
}) {
  const [title, setTitle] = useState(doc.title)
  const [body, setBody] = useState(doc.body)
  const [category, setCategory] = useState<DocCategory>(doc.category)
  const [pushDiscord, setPushDiscord] = useState(true)

  const current = (status: DocStatus): ContentDoc => ({
    ...doc,
    title: title.trim() || 'Untitled Post',
    body,
    category,
    status,
    date: new Date().toISOString().slice(0, 10),
  })

  const saveDraft = () => {
    onChange(current('draft'))
    toast.success('Draft saved')
  }

  const handlePublish = () => {
    if (!title.trim() || !body.trim()) {
      toast.error('Title and body are required')
      return
    }
    const tag = CATEGORY_TAG[category]
    // SOPs have no announcement equivalent — publish locally only.
    if (!tag) {
      onChange(current('published'))
      toast.success('SOP published')
      return
    }
    publish.mutate(
      {
        title: title.trim(),
        body: body.trim(),
        tag,
        is_pinned: false,
        push_to_discord: pushDiscord,
        status: 'published',
      },
      {
        onSuccess: () => {
          onChange(current('published'))
          toast.success(pushDiscord ? 'Published & broadcast to Discord' : 'Published')
        },
        onError: () => toast.error('Publish failed'),
      },
    )
  }

  return (
    <div className="relative flex h-full min-w-0 flex-1 flex-col">
      {/* Title header */}
      <div className="flex items-start justify-between gap-4 p-8 pb-4">
        <input
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="Post Title"
          className="min-w-0 flex-1 bg-transparent text-4xl font-bold text-on-surface outline-none placeholder:text-outline"
        />
        <div className="flex shrink-0 items-center gap-2">
          <select
            value={category}
            onChange={(e) => setCategory(e.target.value as DocCategory)}
            className="rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface outline-none focus:border-primary/50"
          >
            {CATEGORY_OPTIONS.map((c) => (
              <option key={c.value} value={c.value}>
                {c.label}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={() => toast.message('Hero image upload coming soon')}
            className="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-label-sm text-on-surface transition hover:bg-white/5"
          >
            <MaterialIcon name="image" className="text-[18px]" />
            Add Hero Image
          </button>
        </div>
      </div>

      {/* Markdown toolbar */}
      <div className="sticky top-0 z-10 mx-8 flex items-center gap-1 rounded-xl border border-white/10 bg-surface-container/60 p-1 backdrop-blur-md">
        {MD_TOOLS.map((t) => (
          <button
            key={t.label}
            type="button"
            onClick={() => toast.message(`${t.label} (mock)`)}
            aria-label={t.label}
            title={t.label}
            className="flex size-9 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-white/10 hover:text-on-surface"
          >
            <MaterialIcon name={t.icon} className="text-[20px]" />
          </button>
        ))}
      </div>

      {/* Writing canvas */}
      <textarea
        value={body}
        onChange={(e) => setBody(e.target.value)}
        placeholder="Start writing… Markdown supported."
        className="w-full flex-1 resize-none bg-transparent p-8 text-lg leading-relaxed text-on-surface outline-none placeholder:text-outline"
      />

      {/* Publishing footer */}
      <div className="flex items-center justify-between gap-4 border-t border-white/10 bg-white/5 p-6 backdrop-blur-md">
        <label className="flex items-center gap-3">
          <Switch checked={pushDiscord} onCheckedChange={setPushDiscord} />
          <span className="text-label-md text-on-surface-variant">Push to Discord</span>
        </label>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={saveDraft}
            className="rounded-full border border-white/10 px-6 py-3 text-label-md text-on-surface transition hover:bg-white/5"
          >
            Save Draft
          </button>
          <button
            type="button"
            onClick={handlePublish}
            disabled={publish.isPending}
            className="rounded-full bg-action px-7 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
          >
            {publish.isPending ? 'Publishing…' : 'Publish & Broadcast'}
          </button>
        </div>
      </div>
    </div>
  )
}

function severityVariant(s: string): 'primary' | 'tertiary' | 'warning' | 'error' {
  switch (s.toLowerCase()) {
    case 'warn':
    case 'warning':
      return 'warning'
    case 'crit':
    case 'critical':
    case 'error':
      return 'error'
    case 'info':
      return 'tertiary'
    default:
      return 'primary'
  }
}

function severityText(s: string) {
  switch (s.toLowerCase()) {
    case 'warn':
    case 'warning':
      return 'text-tactical-yellow'
    case 'crit':
    case 'critical':
    case 'error':
      return 'text-error-alert'
    case 'info':
      return 'text-tertiary'
    default:
      return 'text-success'
  }
}

export function AuditLogsPage() {
  const [q, setQ] = useState('')
  const { data, isLoading, isError, error } = useAuditLogs({ q: q || undefined })
  const lines = data?.data ?? []
  const [selected, setSelected] = useState<AuditLog | null>(null)
  const active = selected ?? lines[0] ?? null

  return (
    <AdminGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <SplitPane
          masterWidth="60%"
          masterHeader={
            <input
              type="search"
              placeholder="Filter by admin, action, or keyword..."
              value={q}
              onChange={(e) => setQ(e.target.value)}
              className="w-full rounded-lg border border-outline-variant/40 bg-surface-container px-3 py-1.5 font-mono text-code-md outline-none focus:border-primary/60"
            />
          }
          master={
            <div className="font-mono text-code-md">
              {lines.length === 0 ? (
                <p className="px-1 py-4 text-on-surface-variant">No audit logs.</p>
              ) : (
                lines.map((line) => (
                  <button
                    key={line.id}
                    type="button"
                    onClick={() => setSelected(line)}
                    className={cn(
                      'flex w-full gap-2 rounded px-2 py-1 text-left transition-colors hover:bg-surface-variant/40',
                      active?.id === line.id && 'bg-surface-variant/60',
                      line.severity.toLowerCase().startsWith('crit') &&
                        'border-l-2 border-error-alert',
                    )}
                  >
                    <span className="shrink-0 text-outline">
                      {formatShortDate(line.created_at)}
                    </span>
                    <span className={cn('shrink-0 font-semibold', severityText(line.severity))}>
                      [{line.severity.toUpperCase()}]
                    </span>
                    <span className="truncate text-on-surface-variant">
                      {line.actor_name ? `${line.actor_name}: ` : ''}
                      {line.message}
                    </span>
                  </button>
                ))
              )}
              <span className="ml-2 inline-block h-3 w-2 animate-pulse bg-primary align-middle" />
            </div>
          }
          detail={
            active ? (
              <div className="px-8 py-7">
                <div className="mb-4 flex items-center gap-2">
                  <Badge variant={severityVariant(active.severity)}>{active.severity}</Badge>
                  <span className="ml-auto font-mono text-code-md text-outline">
                    {formatLocalDateTime(active.created_at)}
                  </span>
                </div>
                <h1 className="font-mono text-headline-sm text-on-surface">{active.action}</h1>
                <p className="mt-3 text-body-md text-on-surface-variant">{active.message}</p>
                <dl className="mt-6 space-y-2 border-t border-outline-variant/30 pt-4 font-mono text-code-md">
                  <Row label="ID" value={String(active.id)} />
                  <Row label="Actor" value={active.actor_name || active.actor_id || '—'} />
                  <Row label="Action" value={active.action} />
                  <Row label="Severity" value={active.severity} />
                </dl>
              </div>
            ) : (
              <SplitPaneEmpty
                icon={<MaterialIcon name="terminal" className="text-4xl" />}
                message="Select a log entry to inspect."
              />
            )
          }
        />
      </QueryState>
    </AdminGate>
  )
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex gap-3">
      <dt className="w-20 shrink-0 text-outline uppercase">{label}</dt>
      <dd className="text-on-surface-variant">{value}</dd>
    </div>
  )
}
