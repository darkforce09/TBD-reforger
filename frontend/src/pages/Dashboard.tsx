import { Link } from 'react-router-dom'
import { MaterialIcon } from '@/components/MaterialIcon'
import { OpsCard } from '@/components/OpsCard'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { Badge } from '@/components/ui/badge'
import { useDashboard } from '@/hooks/queries'
import {
  countdownLabel,
  formatBytes,
  formatLocalDateTime,
  formatShortDate,
  terrainLabel,
} from '@/lib/format'

export function DashboardPage() {
  const { data, isLoading, isError, error } = useDashboard()

  const next = data?.next_event
  const assignment = data?.my_assignment
  const server = data?.server_status
  const modpack = data?.current_modpack
  const announcements = data?.recent_announcements ?? []

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-4">
          {/* Hero — next operation countdown */}
          <OpsCard glass glow className="items-center gap-6 p-8 text-center">
            <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-primary/10 to-transparent" />
            <div className="z-10 flex flex-col items-center gap-2">
              {next && <Badge variant="primary">{terrainLabel(next.terrain)}</Badge>}
              <div className="my-6 font-mono text-headline-lg tracking-widest text-primary md:text-5xl">
                {next ? `T-MINUS ${countdownLabel(next.start_time)}` : 'NO UPCOMING OPS'}
              </div>
              <p className="text-on-surface-variant">
                {next
                  ? `${next.name} — ${formatLocalDateTime(next.start_time)}`
                  : 'Check the event schedule for new operations.'}
              </p>
            </div>
            {next && (
              <Link
                to={`/events/${next.event_id}`}
                className="z-10 rounded-xl bg-primary px-6 py-3 text-label-md font-medium text-on-primary"
              >
                Open Operation Hub
              </Link>
            )}
          </OpsCard>

          {/* Bento status cards */}
          <section className="grid grid-cols-1 gap-4 md:grid-cols-3">
            <OpsCard glass>
              <div className="flex items-center gap-3">
                <MaterialIcon name="dns" className={server?.is_online ? 'text-success' : 'text-error-alert'} />
                <h3 className="text-headline-sm text-on-surface">Server Uplink</h3>
              </div>
              <Badge variant={server?.is_online ? 'success' : 'neutral'}>
                {server?.is_online ? 'Online' : 'Offline'}
              </Badge>
              <p className="text-on-surface-variant">
                {server ? `${server.player_count}/${server.max_players} Players` : 'No server telemetry'}
              </p>
            </OpsCard>

            <OpsCard glass>
              <div className="flex items-center gap-3">
                <MaterialIcon name="extension" className="text-primary" />
                <h3 className="text-headline-sm text-on-surface">
                  {modpack ? `${modpack.name} v${modpack.version}` : 'Modpack'}
                </h3>
              </div>
              <Badge variant={modpack ? 'primary' : 'neutral'}>{modpack ? 'Synced' : '—'}</Badge>
              <p className="text-on-surface-variant">
                {modpack ? formatBytes(modpack.total_size_bytes) : 'No modpack configured'}
              </p>
            </OpsCard>

            <OpsCard glass>
              <div className="flex items-center gap-3">
                <MaterialIcon name="military_tech" className="text-tertiary" />
                <h3 className="text-headline-sm text-on-surface">Deployment</h3>
              </div>
              {assignment ? (
                <p className="text-label-md text-on-surface-variant">
                  {assignment.faction} — {assignment.squad} — {assignment.role}
                </p>
              ) : (
                <p className="text-label-md text-on-surface-variant">No active assignment</p>
              )}
            </OpsCard>
          </section>

          <section>
            <h2 className="mb-4 text-label-md text-on-surface-variant uppercase tracking-wide">
              Recent Intelligence
            </h2>
            <div className="flex flex-col gap-3">
              {announcements.length === 0 ? (
                <p className="text-label-md text-on-surface-variant">No announcements yet.</p>
              ) : (
                announcements.map((a) => (
                  <Link key={a.id} to="/announcements">
                    <OpsCard glass className="flex-row gap-4 p-4 transition-colors hover:border-primary/40">
                      <div className="flex h-12 w-14 shrink-0 items-center justify-center rounded-lg bg-surface-container-high font-mono text-label-sm font-bold text-primary">
                        {a.published_at ? formatShortDate(a.published_at) : '—'}
                      </div>
                      <div className="min-w-0">
                        <h4 className="text-label-md font-semibold text-on-surface">{a.title}</h4>
                        <p className="mt-1 line-clamp-2 text-label-md text-on-surface-variant">
                          {a.snippet || a.body.slice(0, 120)}
                        </p>
                      </div>
                    </OpsCard>
                  </Link>
                ))
              )}
            </div>
            <Link to="/announcements" className="mt-3 inline-block text-label-md text-primary hover:underline">
              View all announcements
            </Link>
          </section>
        </div>
      </QueryState>
    </AuthGate>
  )
}
