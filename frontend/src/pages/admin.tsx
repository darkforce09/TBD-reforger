import { useState } from 'react'
import { toast } from 'sonner'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AdminGate } from '@/components/AdminGate'
import { QueryState } from '@/components/QueryState'
import {
  useApprovals,
  useAuditLogs,
  useEvents,
  useMissions,
  usePersonnel,
} from '@/hooks/queries'
import {
  useAddEventMission,
  useApproveMission,
  useBanUser,
  useCreateEvent,
  useDeleteEvent,
  usePublishAnnouncement,
  useRejectMission,
  useUpdateUserRole,
} from '@/hooks/mutations'
import { formatLocalDateTime, terrainLabel } from '@/lib/format'
import { cn } from '@/lib/utils'

export function EventManagerPage() {
  const { data: eventsData } = useEvents('all')
  const { data: missionsData } = useMissions('global')
  const createEvent = useCreateEvent()
  const deleteEvent = useDeleteEvent()

  const [startTime, setStartTime] = useState('')
  const [maxSlots, setMaxSlots] = useState(64)
  const [nameOverride, setNameOverride] = useState('')
  const [briefing, setBriefing] = useState('')
  const [locked, setLocked] = useState(false)
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null)

  // Attach-mission form (operates on the selected event).
  const [missionId, setMissionId] = useState('')
  const [missionStart, setMissionStart] = useState('')
  const addMission = useAddEventMission(selectedEventId ?? '')

  const missions = missionsData?.data ?? []
  const events = eventsData?.data ?? []

  const handlePublish = () => {
    if (!startTime) {
      toast.error('Start time is required')
      return
    }
    createEvent.mutate(
      {
        start_time: new Date(startTime).toISOString(),
        max_slots: maxSlots,
        name_override: nameOverride || undefined,
        briefing: briefing || undefined,
        registration_locked: locked,
      },
      {
        onSuccess: () => toast.success('Operation created — select it to attach missions'),
        onError: () => toast.error('Failed to create operation'),
      },
    )
  }

  const handleAttach = () => {
    if (!selectedEventId) {
      toast.error('Select an operation first')
      return
    }
    if (!missionId || !missionStart) {
      toast.error('Mission and start time are required')
      return
    }
    addMission.mutate(
      { mission_id: missionId, start_time: new Date(missionStart).toISOString() },
      {
        onSuccess: () => {
          toast.success('Mission attached — ORBAT generated from mission.json')
          setMissionId('')
          setMissionStart('')
        },
        onError: () => toast.error('Failed to attach mission'),
      },
    )
  }

  const handleDelete = () => {
    if (!selectedEventId) return
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
      <div className="mx-auto w-full max-w-6xl">
        <PageHeader
          title="Event Manager"
          subtitle="Create campaign operations, then attach missions — ORBATs generate automatically."
        />
        <div className="grid gap-6 lg:grid-cols-2">
          <OpsCard className="bg-surface-container-high">
            <h2 className="mb-4 text-lg font-semibold">Operations</h2>
            {events.length === 0 ? (
              <p className="text-sm text-on-surface-variant">No operations scheduled.</p>
            ) : (
              <ul className="max-h-80 space-y-2 overflow-y-auto text-sm">
                {events.map((e) => (
                  <li key={e.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedEventId(e.id)}
                      className={cn(
                        'w-full rounded-lg border px-3 py-2 text-left',
                        selectedEventId === e.id
                          ? 'border-primary bg-primary/10'
                          : 'border-border-subtle hover:bg-surface-container',
                      )}
                    >
                      <span className="font-medium">{e.name_override || 'Untitled Operation'}</span>
                      <span className="mt-1 block text-on-surface-variant">
                        {formatLocalDateTime(e.start_time)} • {e.mission_count} mission
                        {e.mission_count === 1 ? '' : 's'}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <div className="mt-4">
              <button
                type="button"
                onClick={handleDelete}
                disabled={!selectedEventId || deleteEvent.isPending}
                className="rounded-lg border border-error/50 px-4 py-2 text-sm text-error disabled:opacity-50"
              >
                Delete Selected
              </button>
            </div>
          </OpsCard>

          <div className="flex flex-col gap-6">
            <OpsCard className="bg-surface-container-high">
              <h2 className="mb-4 text-lg font-semibold">Create Operation</h2>
              <div className="space-y-4">
                <div>
                  <label className="mb-1 block text-sm text-on-surface-variant">Start Time</label>
                  <input
                    type="datetime-local"
                    value={startTime}
                    onChange={(e) => setStartTime(e.target.value)}
                    className="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm text-on-surface-variant">Name</label>
                  <input
                    type="text"
                    value={nameOverride}
                    onChange={(e) => setNameOverride(e.target.value)}
                    placeholder="e.g. Twin Theaters"
                    className="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
                  />
                </div>
                <div>
                  <label className="mb-1 block text-sm text-on-surface-variant">Briefing</label>
                  <textarea
                    value={briefing}
                    onChange={(e) => setBriefing(e.target.value)}
                    placeholder="Overarching operation lore / briefing"
                    rows={3}
                    className="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
                  />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="mb-1 block text-sm text-on-surface-variant">Max Slots</label>
                    <input
                      type="number"
                      value={maxSlots}
                      onChange={(e) => setMaxSlots(Number(e.target.value))}
                      className="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-sm text-on-surface-variant">Registration</label>
                    <select
                      value={locked ? 'locked' : 'open'}
                      onChange={(e) => setLocked(e.target.value === 'locked')}
                      className="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
                    >
                      <option value="open">Open</option>
                      <option value="locked">Locked</option>
                    </select>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={handlePublish}
                  disabled={createEvent.isPending}
                  className="w-full rounded-lg bg-primary py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                >
                  Create Operation
                </button>
              </div>
            </OpsCard>

            <OpsCard className="bg-surface-container-high">
              <h2 className="mb-1 text-lg font-semibold">Attach Mission</h2>
              <p className="mb-4 text-sm text-on-surface-variant">
                {selectedEventId
                  ? 'ORBAT squads/slots are parsed from the mission.json payload.'
                  : 'Select an operation on the left to attach missions.'}
              </p>
              <div className="space-y-4">
                <div>
                  <label className="mb-1 block text-sm text-on-surface-variant">Mission</label>
                  <select
                    value={missionId}
                    onChange={(e) => setMissionId(e.target.value)}
                    disabled={!selectedEventId}
                    className="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm disabled:opacity-50"
                  >
                    <option value="">Select from Mission Library...</option>
                    {missions.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.title} ({terrainLabel(m.terrain)})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="mb-1 block text-sm text-on-surface-variant">Mission Start Time</label>
                  <input
                    type="datetime-local"
                    value={missionStart}
                    onChange={(e) => setMissionStart(e.target.value)}
                    disabled={!selectedEventId}
                    className="w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm disabled:opacity-50"
                  />
                </div>
                <button
                  type="button"
                  onClick={handleAttach}
                  disabled={!selectedEventId || addMission.isPending}
                  className="w-full rounded-lg bg-primary py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                >
                  Attach Mission
                </button>
              </div>
            </OpsCard>
          </div>
        </div>
      </div>
    </AdminGate>
  )
}

export function MissionApprovalsPage() {
  const { data, isLoading, isError, error } = useApprovals()
  const approve = useApproveMission()
  const reject = useRejectMission()
  const rows = data?.data ?? []

  return (
    <AdminGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-5xl">
          <PageHeader
            title="Mission Approvals"
            subtitle="Review and authorize community-submitted missions for the live database."
          />
          {rows.length === 0 ? (
            <p className="text-on-surface-variant">No pending approvals.</p>
          ) : (
            <div className="overflow-hidden rounded-xl border border-border-subtle">
              <table className="w-full text-sm">
                <thead className="bg-surface-container-high text-xs font-semibold uppercase text-on-surface-variant">
                  <tr>
                    <th className="px-4 py-3 text-left">Mission</th>
                    <th className="px-4 py-3 text-left">Author</th>
                    <th className="px-4 py-3 text-left">Map</th>
                    <th className="px-4 py-3 text-left">Submitted</th>
                    <th className="px-4 py-3 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle bg-surface-container">
                  {rows.map((r) => (
                    <tr key={r.mission_id}>
                      <td className="px-4 py-3 font-medium">{r.title}</td>
                      <td className="px-4 py-3">{r.author_name}</td>
                      <td className="px-4 py-3">{terrainLabel(r.terrain)}</td>
                      <td className="px-4 py-3 text-on-surface-variant">
                        {formatLocalDateTime(r.submitted_at)}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <button
                          type="button"
                          className="mr-2 text-success hover:underline disabled:opacity-50"
                          disabled={approve.isPending}
                          onClick={() =>
                            approve.mutate(r.mission_id, {
                              onSuccess: () => toast.success('Mission approved'),
                              onError: () => toast.error('Approval failed'),
                            })
                          }
                        >
                          Approve
                        </button>
                        <button
                          type="button"
                          className="text-error hover:underline disabled:opacity-50"
                          disabled={reject.isPending}
                          onClick={() =>
                            reject.mutate(
                              { id: r.mission_id },
                              {
                                onSuccess: () => toast.success('Mission rejected'),
                                onError: () => toast.error('Rejection failed'),
                              },
                            )
                          }
                        >
                          Reject
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {data && (
            <p className="mt-4 text-sm text-on-surface-variant">
              Showing {rows.length} of {data.total} pending
            </p>
          )}
        </div>
      </QueryState>
    </AdminGate>
  )
}

export function PersonnelRosterPage() {
  const [q, setQ] = useState('')
  const { data, isLoading, isError, error } = usePersonnel(q || undefined)
  const updateRole = useUpdateUserRole()
  const banUser = useBanUser()
  const users = data?.data ?? []

  return (
    <AdminGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-5xl">
          <PageHeader title="Personnel Roster" subtitle="Manage and audit registered platform users." />
          <input
            type="search"
            placeholder="Search Discord ID or Arma Name..."
            value={q}
            onChange={(e) => setQ(e.target.value)}
            className="mb-6 w-full max-w-md rounded-lg border border-border-subtle bg-surface-container px-3 py-2 text-sm"
          />
          {users.length === 0 ? (
            <p className="text-on-surface-variant">No users found.</p>
          ) : (
            <div className="overflow-hidden rounded-xl border border-border-subtle">
              <table className="w-full text-sm">
                <thead className="bg-surface-container-high text-xs font-semibold uppercase text-on-surface-variant">
                  <tr>
                    <th className="px-4 py-3 text-left">Discord</th>
                    <th className="px-4 py-3 text-left">Arma Identity</th>
                    <th className="px-4 py-3 text-left">Role</th>
                    <th className="px-4 py-3 text-left">Warnings</th>
                    <th className="px-4 py-3 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle bg-surface-container">
                  {users.map((u) => (
                    <tr key={u.discord_id}>
                      <td className="px-4 py-3">{u.discord_handle || u.username}</td>
                      <td className="px-4 py-3">
                        {u.arma_character || u.arma_id || 'Unlinked'}
                      </td>
                      <td className="px-4 py-3">
                        <select
                          value={u.role}
                          onChange={(e) =>
                            updateRole.mutate(
                              { discordId: u.discord_id, role: e.target.value },
                              {
                                onSuccess: () => toast.success('Role updated'),
                                onError: () => toast.error('Failed to update role'),
                              },
                            )
                          }
                          className="rounded border border-border-subtle bg-surface px-2 py-1 text-sm"
                        >
                          <option value="enlisted">Enlisted</option>
                          <option value="mission_maker">Mission Maker</option>
                          <option value="admin">Admin</option>
                        </select>
                      </td>
                      <td className="px-4 py-3">{u.warnings}</td>
                      <td className="px-4 py-3 text-right">
                        {!u.is_banned && (
                          <button
                            type="button"
                            className="text-error hover:underline"
                            onClick={() => {
                              const reason = window.prompt('Ban reason (optional):')
                              banUser.mutate(
                                { discordId: u.discord_id, reason: reason ?? undefined },
                                {
                                  onSuccess: () => toast.success('User banned'),
                                  onError: () => toast.error('Ban failed'),
                                },
                              )
                            }}
                          >
                            Ban
                          </button>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </QueryState>
    </AdminGate>
  )
}

export function ContentManagerPage() {
  const publish = usePublishAnnouncement()
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [tag, setTag] = useState('update')
  const [pinned, setPinned] = useState(false)
  const [pushDiscord, setPushDiscord] = useState(true)

  const handlePublish = () => {
    if (!title.trim() || !body.trim()) {
      toast.error('Title and body are required')
      return
    }
    publish.mutate(
      {
        title: title.trim(),
        body: body.trim(),
        tag,
        is_pinned: pinned,
        push_to_discord: pushDiscord,
        status: 'published',
      },
      {
        onSuccess: () => {
          toast.success('Announcement published')
          setTitle('')
          setBody('')
        },
        onError: () => toast.error('Publish failed'),
      },
    )
  }

  return (
    <AdminGate>
      <div className="mx-auto w-full max-w-3xl">
        <PageHeader
          title="Content Manager"
          subtitle="Create and distribute operational updates across the network."
        />
        <OpsCard className="bg-surface-container-high">
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Post Title"
            className="mb-4 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm font-medium"
          />
          <select
            value={tag}
            onChange={(e) => setTag(e.target.value)}
            className="mb-4 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
          >
            <option value="update">Update</option>
            <option value="event">Community Event</option>
            <option value="modpack_update">Modpack Update</option>
            <option value="important">Important</option>
          </select>
          <textarea
            rows={8}
            value={body}
            onChange={(e) => setBody(e.target.value)}
            placeholder="Draft your briefing here..."
            className="mb-4 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
          />
          <label className="mb-2 flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={pinned}
              onChange={(e) => setPinned(e.target.checked)}
              className="rounded"
            />
            Pin announcement
          </label>
          <label className="mb-4 flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={pushDiscord}
              onChange={(e) => setPushDiscord(e.target.checked)}
              className="rounded"
            />
            Push to Discord Webhook
          </label>
          <button
            type="button"
            onClick={handlePublish}
            disabled={publish.isPending}
            className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
          >
            {publish.isPending ? 'Publishing…' : 'Publish Content'}
          </button>
        </OpsCard>
      </div>
    </AdminGate>
  )
}

export function AuditLogsPage() {
  const [q, setQ] = useState('')
  const { data, isLoading, isError, error } = useAuditLogs({ q: q || undefined })
  const lines = data?.data ?? []

  const severityClass = (s: string) => {
    switch (s.toLowerCase()) {
      case 'warn':
      case 'warning':
        return 'text-warning'
      case 'crit':
      case 'critical':
      case 'error':
        return 'text-error'
      default:
        return 'text-success'
    }
  }

  return (
    <AdminGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-5xl">
          <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
            <div>
              <h1 className="mb-2 text-3xl font-bold">System Audit Logs</h1>
              <p className="text-on-surface-variant">Terminal view of system events.</p>
            </div>
          </div>
          <input
            type="search"
            placeholder="Filter by admin, event, or keyword..."
            value={q}
            onChange={(e) => setQ(e.target.value)}
            className="mb-4 w-full rounded-lg border border-border-subtle bg-surface-container px-3 py-2 font-mono text-sm"
          />
          <div className="rounded-xl border border-border-subtle bg-[#0a0e18] p-4 font-mono text-sm">
            {lines.length === 0 ? (
              <p className="text-on-surface-variant">No audit logs.</p>
            ) : (
              lines.map((line) => (
                <p key={line.id} className="mb-2">
                  <span className={cn('font-semibold', severityClass(line.severity))}>
                    [{line.severity.toUpperCase()}]
                  </span>{' '}
                  <span className="text-on-surface-variant">
                    {line.actor_name ? `${line.actor_name}: ` : ''}
                    {line.message}
                  </span>
                  <span className="ml-2 text-xs text-on-surface-variant/60">
                    {formatLocalDateTime(line.created_at)}
                  </span>
                </p>
              ))
            )}
          </div>
        </div>
      </QueryState>
    </AdminGate>
  )
}
