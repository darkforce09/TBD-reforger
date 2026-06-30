import { Link } from 'react-router-dom'
import { MaterialIcon } from '@/components/MaterialIcon'
import { OpsCard } from '@/components/OpsCard'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { useDashboard } from '@/hooks/queries'
import {
  countdownLabel,
  formatBytes,
  formatUptime,
  formatZuluTime,
  stripHtml,
  terrainLabel,
} from '@/lib/format'
import { cn } from '@/lib/utils'

/** Cinematic hero backdrop from the tactical-command-center blueprint. */
const HERO_IMAGE =
  'https://lh3.googleusercontent.com/aida-public/AB6AXuB_SlrhFHaG9jlm7NfoEUTrANNfG_-m0cqYcJVwKZ1pAUA_LTEnwP1zyNasVKfTgKdnX14ssTtYpEc3I1qn0UaEjwwEQyuAGxherp9Eu5rIpF4afr0sjFAUSjc9Z5NpB2xub7NkJCKNYCkkFsIa25L2e5QrbN4lEOZHeGZeLxpbVtQC8WATlT2skffHxtraZAi95LpXOqnuyLkxHIoJOHtxsFj2rJ4xCywZTnNZy_bJSzmLgPaun0eZsYw-Prx2nJ2GeJMP72x2l-4'

// eslint-disable-next-line complexity -- landing page render: loading/error/empty + bento hero/status/intel sections
export function DashboardPage() {
  const { data, isLoading, isError, error } = useDashboard()

  const next = data?.next_event
  const assignment = data?.my_assignment
  const server = data?.server_status
  const modpack = data?.current_modpack
  const announcements = data?.recent_announcements ?? []

  const playerPct =
    server && server.max_players > 0
      ? Math.round((server.player_count / server.max_players) * 100)
      : 0

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="custom-scrollbar flex h-full w-full flex-col gap-8 overflow-y-auto p-6 md:p-8">
          {/* Hero Banner */}
          <div className="glass border-glow relative flex min-h-[300px] flex-col justify-end overflow-hidden rounded-xl p-8">
            <div className="absolute inset-0 z-0">
              <img
                alt="Operation theater"
                src={HERO_IMAGE}
                className="h-full w-full object-cover opacity-40 mix-blend-overlay"
              />
              <div className="absolute inset-0 bg-gradient-to-t from-surface-container-lowest via-surface-container-lowest/80 to-transparent" />
            </div>
            <div className="relative z-10 flex w-full flex-wrap items-end justify-between gap-4">
              <div className="flex flex-col">
                <h2 className="text-glow mb-2 font-mono text-5xl font-bold tracking-tighter text-primary md:text-7xl">
                  {next ? `T-MINUS ${countdownLabel(next.start_time)}` : 'NO UPCOMING OPS'}
                </h2>
                <p className="flex items-center gap-2 text-sm tracking-widest text-on-surface uppercase opacity-80">
                  <span className="h-2 w-2 animate-pulse rounded-full bg-primary" />
                  {next
                    ? `OPERATION: ${next.name} — ${terrainLabel(next.terrain)}`
                    : 'Check the event schedule for new operations.'}
                </p>
              </div>
              {next && (
                <Link
                  to={`/events/${next.event_id}`}
                  className="group flex items-center gap-2 rounded-lg border border-primary/50 bg-surface/50 px-6 py-3 text-sm font-bold tracking-widest text-primary uppercase backdrop-blur-md transition-all hover:bg-primary/20 active:scale-95"
                >
                  Open Operation Hub
                  <MaterialIcon
                    name="arrow_forward"
                    className="transition-transform group-hover:translate-x-1"
                  />
                </Link>
              )}
            </div>
          </div>

          {/* Bento Grid */}
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
            {/* Server Uplink */}
            <OpsCard glass className="gap-4">
              <div className="flex items-center justify-between border-b border-border-subtle pb-3">
                <h3 className="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                  <MaterialIcon name="dns" className="text-[18px]" />
                  Server Uplink
                </h3>
                <div className="flex items-center gap-2 rounded-full border border-success/30 bg-success-muted px-2 py-1">
                  <div
                    className={cn(
                      'h-2 w-2 rounded-full',
                      server?.is_online ? 'bg-success tactical-pulse' : 'bg-outline',
                    )}
                  />
                  <span className="font-mono text-[10px] font-bold tracking-widest text-success">
                    {server?.is_online ? 'ONLINE' : 'OFFLINE'}
                  </span>
                </div>
              </div>
              <div className="mt-2 flex flex-col">
                <div className="mb-2 flex items-end justify-between">
                  <span className="font-mono text-3xl font-light text-on-surface">
                    {server?.player_count ?? 0}
                    <span className="text-lg text-on-surface-variant">
                      /{server?.max_players ?? 0}
                    </span>
                  </span>
                  <span className="mb-1 font-mono text-xs text-on-surface-variant">PLAYERS</span>
                </div>
                <div className="h-1.5 w-full overflow-hidden rounded-full bg-surface-container-highest">
                  <div
                    className="h-1.5 rounded-full bg-primary shadow-[0_0_10px_#adc6ff]"
                    style={{ width: `${playerPct}%` }}
                  />
                </div>
              </div>
              <div className="mt-auto flex items-center justify-between pt-4 font-mono text-xs text-on-surface-variant/60">
                <span>FPS: {server ? server.server_fps : '—'}</span>
                <span>UPTIME: {server ? formatUptime(server.uptime_seconds) : '—'}</span>
              </div>
            </OpsCard>

            {/* Deployment */}
            <OpsCard glass className="group gap-4">
              <div className="pointer-events-none absolute -right-10 -bottom-10 opacity-5 transition-opacity group-hover:opacity-10">
                <MaterialIcon name="military_tech" filled className="text-[200px]" />
              </div>
              <div className="relative z-10 flex items-center justify-between border-b border-border-subtle pb-3">
                <h3 className="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                  <MaterialIcon name="person" className="text-[18px]" />
                  Deployment
                </h3>
              </div>
              {assignment ? (
                <div className="relative z-10 mt-4 flex items-center gap-6">
                  <div className="flex gap-2">
                    <div className="flex h-12 w-12 items-center justify-center rounded-lg border border-border-subtle bg-surface-container-highest">
                      <MaterialIcon name="swords" filled className="text-[28px] text-primary" />
                    </div>
                    <div className="flex h-12 w-12 items-center justify-center rounded-lg border border-border-subtle bg-surface-container-highest">
                      <MaterialIcon name="security" filled className="text-[28px] text-primary" />
                    </div>
                  </div>
                  <div className="flex flex-col">
                    <span className="font-bold tracking-wide text-on-surface">
                      {assignment.faction}
                    </span>
                    <span className="text-sm text-on-surface-variant">{assignment.squad}</span>
                    <span className="mt-1 font-mono text-xs text-primary uppercase">
                      Role: {assignment.role}
                    </span>
                  </div>
                </div>
              ) : (
                <p className="relative z-10 mt-4 text-sm text-on-surface-variant">
                  No active assignment
                </p>
              )}
            </OpsCard>

            {/* Modpack */}
            <OpsCard glass className="gap-4">
              <div className="flex items-center justify-between border-b border-border-subtle pb-3">
                <h3 className="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                  <MaterialIcon name="extension" className="text-[18px]" />
                  Modpack
                </h3>
              </div>
              <div className="mt-2 flex h-full flex-col justify-between">
                <div>
                  <h4 className="mb-1 text-xl font-bold text-on-surface">
                    {modpack ? `${modpack.name} v${modpack.version}` : 'No modpack'}
                  </h4>
                  <span className="font-mono text-xs text-on-surface-variant">
                    {modpack ? `SIZE: ${formatBytes(modpack.total_size_bytes)}` : '—'}
                  </span>
                </div>
                <div className="mt-4 flex items-center justify-between rounded-lg border border-border-subtle bg-surface-container-lowest p-3">
                  <span
                    className={cn(
                      'flex items-center gap-2 font-mono text-xs font-bold tracking-widest',
                      modpack ? 'text-success' : 'text-on-surface-variant',
                    )}
                  >
                    <MaterialIcon name="check_circle" filled className="text-[14px]" />
                    STATUS: {modpack ? 'SYNCED' : 'NONE'}
                  </span>
                  <MaterialIcon name="sync" className="text-[18px] text-on-surface-variant" />
                </div>
              </div>
            </OpsCard>
          </div>

          {/* Recent Intelligence Feed */}
          <OpsCard glass className="flex-1 gap-4">
            <h3 className="flex items-center gap-2 border-b border-border-subtle pb-3 text-label-sm text-on-surface-variant uppercase">
              <MaterialIcon name="list_alt" className="text-[18px]" />
              Recent Intelligence
            </h3>
            <div className="custom-scrollbar flex flex-col gap-2 overflow-y-auto pr-2">
              {announcements.length === 0 ? (
                <p className="text-label-md text-on-surface-variant">No announcements yet.</p>
              ) : (
                announcements.map((a, i) => {
                  const unread = i === 0
                  return (
                    <Link
                      key={a.id}
                      to="/announcements"
                      className={cn(
                        'flex items-start gap-4 rounded-lg p-3 transition-colors hover:bg-surface-variant/30',
                        unread && 'border-l-2 border-l-primary bg-surface-container/50',
                      )}
                    >
                      <span
                        className={cn(
                          'mt-0.5 font-mono text-xs whitespace-nowrap',
                          unread ? 'text-primary opacity-80' : 'text-on-surface-variant opacity-60',
                        )}
                      >
                        [{formatZuluTime(a.published_at ?? a.created_at)}]
                      </span>
                      <div className="flex min-w-0 flex-col">
                        <span
                          className={cn(
                            'text-sm',
                            unread ? 'font-bold text-on-surface' : 'font-medium text-on-surface/80',
                          )}
                        >
                          {a.title}
                        </span>
                        <span className="mt-1 line-clamp-2 text-xs text-on-surface-variant/70">
                          {stripHtml(a.snippet || a.body)}
                        </span>
                      </div>
                      {unread && (
                        <MaterialIcon
                          name="fiber_new"
                          filled
                          className="ml-auto text-[16px] text-primary opacity-50"
                        />
                      )}
                    </Link>
                  )
                })
              )}
            </div>
          </OpsCard>
        </div>
      </QueryState>
    </AuthGate>
  )
}
