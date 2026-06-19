import { useState } from 'react'
import { Link } from 'react-router-dom'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { MaterialIcon } from '@/components/MaterialIcon'
import { SplitPane, SplitPaneEmpty } from '@/components/ui/split-pane'
import { ListDetailItem } from '@/components/ui/list-detail-item'
import { Badge } from '@/components/ui/badge'
import { useAuthStore } from '@/store/useAuthStore'
import { DEFAULT_AVATAR } from '@/lib/avatar'
import type { Announcement } from '@/types/api'
import {
  useAnnouncements,
  useDeployments,
  useEvent,
  useEvents,
  useLeaderboards,
} from '@/hooks/queries'
import { EventHubView } from '@/pages/events'
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
  const [selectedId, setSelectedId] = useState<string | null>(null)

  // Pinned first, then the rest — preserving incoming order within each group.
  const posts = [...(data?.data ?? [])].sort(
    (a, b) => Number(b.is_pinned) - Number(a.is_pinned),
  )
  const selected = posts.find((p) => p.id === selectedId) ?? posts[0] ?? null

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <SplitPane
          masterHeader={
            <>
              <h2 className="text-headline-sm tracking-wide text-on-surface uppercase">Comms Link</h2>
              <MaterialIcon name="filter_list" className="text-outline" />
            </>
          }
          master={
            posts.length === 0 ? (
              <p className="px-1 py-4 text-label-md text-on-surface-variant">No announcements yet.</p>
            ) : (
              posts.map((a) => (
                <ListDetailItem
                  key={a.id}
                  active={selected?.id === a.id}
                  onClick={() => setSelectedId(a.id)}
                  meta={`[${formatShortDate(a.published_at || a.created_at)}]`}
                  dotClassName={a.is_pinned ? 'bg-tactical-yellow' : undefined}
                  title={a.title}
                  preview={a.snippet || a.body}
                />
              ))
            )
          }
          detail={
            selected ? (
              <AnnouncementDetail post={selected} />
            ) : (
              <SplitPaneEmpty
                icon={<MaterialIcon name="campaign" className="text-4xl" />}
                message="Select a broadcast to read."
              />
            )
          }
        />
      </QueryState>
    </AuthGate>
  )
}

function AnnouncementDetail({ post }: { post: Announcement }) {
  return (
    <>
      <div className="z-10 shrink-0 border-b border-outline-variant/20 bg-surface-container-low/40 px-8 py-7 backdrop-blur-md md:px-12">
        <div className="mb-4 flex flex-wrap items-center gap-3">
          {post.is_pinned && (
            <Badge variant="warning" icon="push_pin">
              Pinned
            </Badge>
          )}
          <Badge variant="primary">{tagLabel(post.tag)}</Badge>
          {post.published_at && (
            <span className="ml-auto font-mono text-code-md text-outline">
              {formatLocalDateTime(post.published_at)}
            </span>
          )}
        </div>
        <h1 className="text-headline-lg text-on-surface">{post.title}</h1>
      </div>
      <div className="px-8 py-8 md:px-12">
        <div className="mx-auto max-w-3xl space-y-6">
          {post.thumbnail_url && (
            <div className="relative h-72 w-full overflow-hidden rounded-xl border border-outline-variant/30 shadow-2xl">
              <img src={post.thumbnail_url} alt="" className="h-full w-full object-cover" />
              <div className="absolute inset-0 bg-gradient-to-t from-surface-container-lowest/80 to-transparent" />
            </div>
          )}
          <div className="text-body-lg whitespace-pre-line text-on-surface-variant">{post.body}</div>
        </div>
      </div>
    </>
  )
}

function outcomeIsWin(outcome: string) {
  return /vict|win|success|complete/i.test(outcome)
}

export function DeploymentsPage() {
  const { data, isLoading, isError, error } = useDeployments()
  const user = useAuthStore((s) => s.user)
  const upcoming = data?.upcoming ?? []
  const history = data?.service_history ?? []

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="bg-topo-map bg-grid-overlay flex h-full min-h-0 w-full overflow-hidden">
          {/* Left: service stats dossier */}
          <aside className="custom-scrollbar flex h-full w-72 shrink-0 flex-col gap-6 overflow-y-auto border-r border-outline-variant/30 bg-surface-container-lowest/50 p-6">
            <div className="flex flex-col items-center text-center">
              <img
                src={user?.avatar_url || DEFAULT_AVATAR}
                alt=""
                className="h-20 w-20 rounded-full border border-outline-variant/50 object-cover"
              />
              <h2 className="mt-3 text-headline-sm text-on-surface">{user?.username}</h2>
              <span className="text-label-sm text-on-surface-variant uppercase">{user?.role}</span>
            </div>
            <div className="space-y-4">
              <div>
                <span className="text-label-sm text-on-surface-variant uppercase">Total Operations</span>
                <p className="font-mono text-headline-lg text-primary">{data?.total_operations ?? 0}</p>
              </div>
              <div>
                <span className="text-label-sm text-on-surface-variant uppercase">Attendance Rate</span>
                <p className="font-mono text-headline-lg text-success">{data?.attendance_rate ?? 0}%</p>
              </div>
              <div>
                <span className="text-label-sm text-on-surface-variant uppercase">Awaiting Deployment</span>
                <p className="font-mono text-headline-lg text-tertiary">{upcoming.length}</p>
              </div>
            </div>
          </aside>

          {/* Right: upcoming + combat timeline */}
          <main className="custom-scrollbar flex h-full min-h-0 flex-1 flex-col gap-8 overflow-y-auto bg-surface-container-highest/10 p-8">
            <section>
              <h2 className="mb-4 text-label-md text-on-surface-variant uppercase tracking-wide">
                Awaiting Deployment
              </h2>
              {upcoming.length === 0 ? (
                <p className="text-label-md text-on-surface-variant">No upcoming deployments registered.</p>
              ) : (
                <div className="grid gap-4 sm:grid-cols-2">
                  {upcoming.map((e) => (
                    <Link key={e.event_id} to={`/events/${e.event_id}`}>
                      <OpsCard glass className="h-full transition-colors hover:border-primary/40">
                        <h3 className="text-headline-sm text-on-surface">{e.name}</h3>
                        <p className="text-label-md text-on-surface-variant">
                          {formatLocalDateTime(e.start_time)} — {terrainLabel(e.terrain)}
                        </p>
                        {(e.faction || e.squad || e.role) && (
                          <p className="mt-2 text-label-md">
                            <span className="text-on-surface-variant">ASSIGNED: </span>
                            {[e.faction, e.squad, e.role].filter(Boolean).join(' — ')}
                          </p>
                        )}
                      </OpsCard>
                    </Link>
                  ))}
                </div>
              )}
            </section>

            <section>
              <h2 className="mb-4 text-label-md text-on-surface-variant uppercase tracking-wide">
                Combat History
              </h2>
              {history.length === 0 ? (
                <p className="text-label-md text-on-surface-variant">No service history yet.</p>
              ) : (
                <ol className="relative ml-2 border-l border-outline-variant/30">
                  {history.map((row, i) => {
                    const win = outcomeIsWin(row.outcome)
                    return (
                      <li key={`${row.operation}-${i}`} className="mb-5 ml-6">
                        <span
                          className={cn(
                            'absolute -left-[7px] mt-1.5 h-3 w-3 rounded-full border-2 border-background',
                            win ? 'bg-success' : 'bg-error-alert',
                          )}
                        />
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <h3 className="text-label-md font-semibold text-on-surface">{row.operation}</h3>
                          <Badge variant={win ? 'success' : 'error'}>{row.outcome}</Badge>
                        </div>
                        <p className="mt-1 text-label-sm text-outline">
                          {formatShortDate(row.date)} · {row.role}
                        </p>
                      </li>
                    )
                  })}
                </ol>
              )}
            </section>
          </main>
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
                  glass
                  key={p.discord_id}
                  className={cn(
                    'text-center',
                    p.rank === 1 && 'border-primary/40 py-8',
                  )}
                >
                  <span className="font-mono text-headline-lg text-primary">#{p.rank}</span>
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

function eventStatusLabel(status: string, locked: boolean) {
  if (status === 'live') return 'LIVE'
  if (locked) return 'LOCKED'
  return status.toUpperCase()
}

export function EventSchedulePage() {
  const { data, isLoading, isError, error } = useEvents('upcoming')
  const events = data?.data ?? []
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const selected = events.find((e) => e.id === selectedId) ?? events[0] ?? null

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <SplitPane
          masterWidth="24rem"
          masterHeader={
            <h2 className="text-headline-sm tracking-wide text-on-surface uppercase">Upcoming Ops</h2>
          }
          master={
            events.length === 0 ? (
              <p className="px-1 py-4 text-label-md text-on-surface-variant">
                No upcoming operations scheduled.
              </p>
            ) : (
              events.map((e) => {
                const label = eventStatusLabel(e.status, e.registration_locked)
                const active = selected?.id === e.id
                return (
                  <button
                    key={e.id}
                    type="button"
                    onClick={() => setSelectedId(e.id)}
                    className={cn(
                      'group relative w-full overflow-hidden rounded-lg border p-3 text-left transition-all',
                      active
                        ? 'border-primary/30 bg-surface-variant/80 shadow-[inset_0_0_15px_rgba(173,198,255,0.1)]'
                        : 'border-transparent hover:border-outline-variant/30 hover:bg-surface-variant/40',
                    )}
                  >
                    {active && <span className="absolute top-0 bottom-0 left-0 w-1 bg-primary" />}
                    <div className="mb-1 flex items-center justify-between gap-2">
                      <span className="font-mono text-code-md text-outline">
                        {formatLocalDateTime(e.start_time)}
                      </span>
                      <Badge
                        variant={label === 'LIVE' ? 'primary' : label === 'LOCKED' ? 'neutral' : 'success'}
                      >
                        {label}
                      </Badge>
                    </div>
                    <h3 className="text-label-md font-semibold text-on-surface">
                      {e.name_override || 'Untitled Operation'}
                    </h3>
                    <p className="mt-0.5 text-label-sm text-outline">
                      {e.mission_count} mission{e.mission_count === 1 ? '' : 's'}
                    </p>
                    <div className="mt-2 flex items-center gap-2">
                      <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-container-highest">
                        <div
                          className="h-full rounded-full bg-primary"
                          style={{ width: `${Math.min(100, e.percent)}%` }}
                        />
                      </div>
                      <span className="shrink-0 font-mono text-label-sm text-on-surface-variant">
                        {e.filled}/{e.total_slots}
                      </span>
                    </div>
                  </button>
                )
              })
            )
          }
          detail={
            selected ? (
              <div className="px-8 py-8">
                <EventScheduleDetail id={selected.id} />
              </div>
            ) : (
              <SplitPaneEmpty
                icon={<MaterialIcon name="calendar_month" className="text-4xl" />}
                message="Select an operation to view its hub."
              />
            )
          }
        />
      </QueryState>
    </AuthGate>
  )
}

function EventScheduleDetail({ id }: { id: string }) {
  const { data: event, isLoading, isError, error } = useEvent(id)
  return (
    <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
      {event && <EventHubView event={event} />}
    </QueryState>
  )
}
