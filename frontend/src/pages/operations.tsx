import { useState } from 'react'
import { Link } from 'react-router-dom'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import {
  useAnnouncements,
  useDeployments,
  useEvents,
  useLeaderboards,
} from '@/hooks/queries'
import {
  formatLocalDateTime,
  formatShortDate,
  tagLabel,
  terrainLabel,
} from '@/lib/format'
import { cn } from '@/lib/utils'

const LEADERBOARD_TABS = [
  { label: 'K/D Ratio', category: 'kd' },
  { label: 'Command Win Rate', category: 'command_win' },
  { label: 'Wall of Shame', category: 'team_kills' },
] as const

export function AnnouncementsPage() {
  const { data, isLoading, isError, error } = useAnnouncements(50, 0)
  const posts = data?.data ?? []
  const pinned = posts.filter((p) => p.is_pinned)
  const rest = posts.filter((p) => !p.is_pinned)

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-4xl">
          <PageHeader title="Command Announcements" subtitle="Operational updates from command staff." />
          {pinned.map((a) => (
            <OpsCard key={a.id} className="mb-6 border-primary/30 bg-surface-container-high">
              <div className="mb-4 flex flex-wrap gap-2">
                <span className="rounded bg-primary/20 px-2 py-0.5 text-xs font-semibold text-primary">
                  PINNED
                </span>
                <span className="rounded bg-surface-container-highest px-2 py-0.5 text-xs font-semibold text-on-surface-variant">
                  {tagLabel(a.tag)}
                </span>
              </div>
              <h2 className="text-xl font-semibold">{a.title}</h2>
              {a.published_at && (
                <p className="mt-1 text-sm text-on-surface-variant">
                  Published {formatLocalDateTime(a.published_at)}
                </p>
              )}
              <p className="mt-4 text-on-surface-variant">{a.snippet || a.body}</p>
            </OpsCard>
          ))}
          <div className="flex flex-col gap-4">
            {rest.length === 0 && pinned.length === 0 ? (
              <p className="text-on-surface-variant">No announcements yet.</p>
            ) : (
              rest.map((a) => (
                <OpsCard key={a.id} className="bg-surface-container-high">
                  <span className="mb-2 inline-block rounded bg-surface-container-highest px-2 py-0.5 text-xs font-semibold text-on-surface-variant">
                    {tagLabel(a.tag)}
                  </span>
                  <h3 className="text-lg font-semibold">{a.title}</h3>
                  {a.published_at && (
                    <p className="mt-1 text-sm text-on-surface-variant">
                      {formatLocalDateTime(a.published_at)}
                    </p>
                  )}
                  <p className="mt-2 text-sm text-on-surface-variant">{a.snippet || a.body.slice(0, 200)}</p>
                </OpsCard>
              ))
            )}
          </div>
        </div>
      </QueryState>
    </AuthGate>
  )
}

export function DeploymentsPage() {
  const { data, isLoading, isError, error } = useDeployments()
  const upcoming = data?.upcoming ?? []
  const history = data?.service_history ?? []

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-5xl">
          <PageHeader title="My Deployments" subtitle="Service record and upcoming operations." />
          <OpsCard className="mb-8 bg-surface-container-high">
            <div className="flex flex-wrap gap-8 text-sm">
              <div>
                <span className="text-on-surface-variant">Total Operations</span>
                <p className="text-2xl font-bold text-primary">{data?.total_operations ?? 0}</p>
              </div>
              <div>
                <span className="text-on-surface-variant">Attendance Rate</span>
                <p className="text-2xl font-bold text-success">{data?.attendance_rate ?? 0}%</p>
              </div>
            </div>
          </OpsCard>
          <h2 className="mb-4 text-lg font-semibold">Awaiting Deployment</h2>
          {upcoming.length === 0 ? (
            <p className="mb-8 text-sm text-on-surface-variant">No upcoming deployments registered.</p>
          ) : (
            upcoming.map((e) => (
              <OpsCard key={e.event_id} className="mb-4 border-primary/20 bg-surface-container-high">
                <h3 className="text-lg font-semibold">{e.name}</h3>
                <p className="mt-1 text-on-surface-variant">
                  {formatLocalDateTime(e.start_time)} — {terrainLabel(e.terrain)}
                </p>
                {(e.faction || e.squad || e.role) && (
                  <p className="mt-3 text-sm">
                    <span className="text-on-surface-variant">ASSIGNED SLOT: </span>
                    {[e.faction, e.squad, e.role].filter(Boolean).join(' — ')}
                  </p>
                )}
              </OpsCard>
            ))
          )}
          <h2 className="mb-4 text-lg font-semibold">Service Record</h2>
          {history.length === 0 ? (
            <p className="text-sm text-on-surface-variant">No service history yet.</p>
          ) : (
            <div className="overflow-hidden rounded-xl border border-border-subtle">
              <table className="w-full text-left text-sm">
                <thead className="bg-surface-container-high text-xs font-semibold tracking-widest text-on-surface-variant uppercase">
                  <tr>
                    <th className="px-4 py-3">Operation</th>
                    <th className="px-4 py-3">Date</th>
                    <th className="px-4 py-3">Role</th>
                    <th className="px-4 py-3">Result</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle bg-surface-container">
                  {history.map((row, i) => (
                    <tr key={`${row.operation}-${i}`}>
                      <td className="px-4 py-3">{row.operation}</td>
                      <td className="px-4 py-3 text-on-surface-variant">{formatShortDate(row.date)}</td>
                      <td className="px-4 py-3 text-on-surface-variant">{row.role}</td>
                      <td className="px-4 py-3 text-success">{row.outcome}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </QueryState>
    </AuthGate>
  )
}

export function LeaderboardsPage() {
  const [tab, setTab] = useState(0)
  const [search, setSearch] = useState('')
  const category = LEADERBOARD_TABS[tab]?.category ?? 'kd'
  const { data, isLoading, isError, error } = useLeaderboards(category, search || undefined)
  const rows = data?.data ?? []
  const podium = rows.slice(0, 3)
  const tableRows = rows.slice(3)

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-5xl">
          <PageHeader
            title="Global Leaderboards"
            subtitle="Real-time tactical performance metrics across all active theaters."
          />
          <div className="mb-4">
            <input
              type="search"
              placeholder="Search operators..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="w-full max-w-sm rounded-lg border border-border-subtle bg-surface-container px-3 py-2 text-sm"
            />
          </div>
          <div className="mb-6 flex flex-wrap gap-2">
            {LEADERBOARD_TABS.map((t, i) => (
              <button
                key={t.category}
                type="button"
                onClick={() => setTab(i)}
                className={cn(
                  'rounded-lg px-4 py-2 text-sm font-medium',
                  i === tab ? 'bg-primary text-on-primary' : 'bg-surface-container-high text-on-surface-variant',
                )}
              >
                {t.label}
              </button>
            ))}
          </div>
          {podium.length > 0 && (
            <div className="mb-8 grid grid-cols-3 items-end gap-4">
              {[podium[1], podium[0], podium[2]].filter(Boolean).map((p) => (
                <OpsCard
                  key={p.discord_id}
                  className={cn(
                    'bg-surface-container-high text-center',
                    p.rank === 1 && 'border-primary/40 py-8',
                  )}
                >
                  <span className="text-3xl font-bold text-primary">#{p.rank}</span>
                  <p className="mt-2 font-semibold">{p.username}</p>
                  <p className="text-sm text-on-surface-variant">
                    {category === 'kd' && p.kd_ratio != null && `${p.kd_ratio.toFixed(2)} K/D`}
                    {category === 'command_win' &&
                      p.command_win_rate != null &&
                      `${p.command_win_rate.toFixed(0)}% win rate`}
                    {category === 'team_kills' &&
                      p.team_kills != null &&
                      `${p.team_kills} team kills`}
                  </p>
                </OpsCard>
              ))}
            </div>
          )}
          {rows.length === 0 ? (
            <p className="text-on-surface-variant">No leaderboard data yet.</p>
          ) : (
            <div className="overflow-hidden rounded-xl border border-border-subtle">
              <table className="w-full text-sm">
                <thead className="bg-surface-container-high text-xs font-semibold uppercase text-on-surface-variant">
                  <tr>
                    <th className="px-4 py-3">Rank</th>
                    <th className="px-4 py-3">Operator</th>
                    {category === 'kd' && (
                      <>
                        <th className="px-4 py-3">Kills</th>
                        <th className="px-4 py-3">K/D</th>
                      </>
                    )}
                    {category === 'command_win' && <th className="px-4 py-3">Win Rate</th>}
                    {category === 'team_kills' && <th className="px-4 py-3">Team Kills</th>}
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle bg-surface-container">
                  {(tableRows.length ? tableRows : rows).map((r) => (
                    <tr key={r.discord_id}>
                      <td className="px-4 py-3 text-on-surface-variant">{r.rank}</td>
                      <td className="px-4 py-3 font-medium">{r.username}</td>
                      {category === 'kd' && (
                        <>
                          <td className="px-4 py-3">{r.kills ?? '—'}</td>
                          <td className="px-4 py-3 text-primary">
                            {r.kd_ratio != null ? r.kd_ratio.toFixed(2) : '—'}
                          </td>
                        </>
                      )}
                      {category === 'command_win' && (
                        <td className="px-4 py-3 text-primary">
                          {r.command_win_rate != null ? `${r.command_win_rate.toFixed(0)}%` : '—'}
                        </td>
                      )}
                      {category === 'team_kills' && (
                        <td className="px-4 py-3 text-error">{r.team_kills ?? '—'}</td>
                      )}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </QueryState>
    </AuthGate>
  )
}

export function EventSchedulePage() {
  const { data, isLoading, isError, error } = useEvents('upcoming')
  const events = data?.data ?? []

  const statusLabel = (status: string, locked: boolean) => {
    if (status === 'live') return 'LIVE'
    if (locked) return 'LOCKED'
    return status.toUpperCase()
  }

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-5xl">
          <PageHeader
            title="Upcoming Operations"
            subtitle="Review and register for scheduled tactical deployments."
          />
          {events.length === 0 ? (
            <p className="text-on-surface-variant">No upcoming operations scheduled.</p>
          ) : (
            <div className="overflow-hidden rounded-xl border border-border-subtle">
              <table className="w-full text-sm">
                <thead className="bg-surface-container-high text-xs font-semibold uppercase text-on-surface-variant">
                  <tr>
                    <th className="px-4 py-3 text-left">Operation</th>
                    <th className="px-4 py-3 text-left">Schedule</th>
                    <th className="px-4 py-3 text-right">Missions</th>
                    <th className="px-4 py-3 text-left">Status</th>
                    <th className="px-4 py-3 text-right">Slots Filled</th>
                    <th className="px-4 py-3 text-right" />
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle bg-surface-container">
                  {events.map((e) => {
                    const label = statusLabel(e.status, e.registration_locked)
                    return (
                      <tr key={e.id}>
                        <td className="px-4 py-3 font-medium">
                          {e.name_override || 'Untitled Operation'}
                        </td>
                        <td className="px-4 py-3 text-on-surface-variant">
                          {formatLocalDateTime(e.start_time)}
                        </td>
                        <td className="px-4 py-3 text-right text-on-surface-variant">
                          {e.mission_count}
                        </td>
                        <td className="px-4 py-3">
                          <span
                            className={cn(
                              'rounded px-2 py-0.5 text-xs font-semibold',
                              label === 'LIVE' && 'bg-primary/20 text-primary',
                              label === 'LOCKED' && 'bg-surface-container-highest text-on-surface-variant',
                              label !== 'LIVE' && label !== 'LOCKED' && 'bg-success-muted text-success',
                            )}
                          >
                            {label}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-right text-on-surface-variant">
                          {e.filled}/{e.total_slots}
                        </td>
                        <td className="px-4 py-3 text-right">
                          <Link
                            to={`/events/${e.id}`}
                            className="text-sm text-primary hover:underline"
                          >
                            Open Hub
                          </Link>
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )}
          {data && (
            <p className="mt-4 text-sm text-on-surface-variant">
              Showing {events.length} of {data.total} upcoming operations
            </p>
          )}
        </div>
      </QueryState>
    </AuthGate>
  )
}
