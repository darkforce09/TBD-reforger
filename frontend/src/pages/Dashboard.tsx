import { Link } from 'react-router-dom'
import { MaterialIcon } from '@/components/MaterialIcon'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { useDashboard } from '@/hooks/queries'
import { useRegisterEvent } from '@/hooks/mutations'
import {
  countdownLabel,
  formatBytes,
  formatLocalDateTime,
  formatShortDate,
  terrainLabel,
} from '@/lib/format'
import { toast } from 'sonner'

export function DashboardPage() {
  const { data, isLoading, isError, error } = useDashboard()
  const register = useRegisterEvent()

  const next = data?.next_event
  const assignment = data?.my_assignment
  const server = data?.server_status
  const modpack = data?.current_modpack
  const announcements = data?.recent_announcements ?? []

  const handleRegister = () => {
    if (!next?.event_id) return
    register.mutate(next.event_id, {
      onSuccess: () => toast.success('Registered for deployment'),
      onError: () => toast.error('Registration failed'),
    })
  }

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto flex w-full max-w-5xl flex-col gap-4">
          <section className="relative flex flex-col items-center justify-center gap-6 overflow-hidden rounded-xl border border-border-subtle bg-surface-container p-8 text-center">
            <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-primary/10 to-transparent" />
            <div className="z-10 flex flex-col items-center gap-2">
              {next && (
                <span className="rounded-full border border-border-subtle bg-surface-container-high px-3 py-1 text-xs font-semibold tracking-widest uppercase">
                  {terrainLabel(next.terrain)}
                </span>
              )}
              <div className="my-6 text-2xl font-semibold tracking-widest text-primary md:text-4xl">
                {next ? `T-MINUS ${countdownLabel(next.start_time)}` : 'NO UPCOMING OPS'}
              </div>
              <p className="text-on-surface-variant">
                {next
                  ? `${next.name} — ${formatLocalDateTime(next.start_time)}`
                  : 'Check the event schedule for new operations.'}
              </p>
            </div>
            {next && (
              <button
                type="button"
                onClick={handleRegister}
                disabled={register.isPending}
                className="z-10 rounded-xl bg-primary px-6 py-3 text-sm font-medium text-on-primary disabled:opacity-50"
              >
                Register for Deployment
              </button>
            )}
          </section>

          <section className="grid grid-cols-1 gap-4 md:grid-cols-3">
            <div className="relative flex flex-col gap-3 overflow-hidden rounded-xl border border-border-subtle bg-surface-container p-6">
              <div className="flex items-center gap-3">
                <MaterialIcon name="dns" className={server?.is_online ? 'text-success' : 'text-error'} />
                <h3 className="font-semibold">{server?.is_online ? 'Online' : 'Offline'}</h3>
              </div>
              <span className={`text-xs font-semibold uppercase ${server?.is_online ? 'text-success' : 'text-on-surface-variant'}`}>
                Status: {server?.is_online ? 'Active' : 'Unavailable'}
              </span>
              <p className="text-on-surface-variant">
                {server
                  ? `${server.player_count}/${server.max_players} Players`
                  : 'No server telemetry'}
              </p>
            </div>

            <div className="flex flex-col gap-3 rounded-xl border border-border-subtle bg-surface-container p-6">
              <div className="flex items-center gap-3">
                <MaterialIcon name="extension" className="text-primary" />
                <h3 className="font-semibold">
                  {modpack ? `${modpack.name} v${modpack.version}` : 'Modpack'}
                </h3>
              </div>
              <span className="text-xs font-semibold text-primary uppercase">
                {modpack ? 'Synced' : '—'}
              </span>
              <p className="text-on-surface-variant">
                {modpack ? formatBytes(modpack.total_size_bytes) : 'No modpack configured'}
              </p>
            </div>

            <div className="flex flex-col gap-3 rounded-xl border border-border-subtle bg-surface-container p-6 opacity-90">
              <div className="flex items-center gap-3">
                <MaterialIcon name="military_tech" className="text-on-surface-variant" />
                <h3 className="font-semibold">Deployed</h3>
              </div>
              {assignment ? (
                <p className="text-sm text-on-surface-variant">
                  {assignment.faction} — {assignment.squad} — {assignment.role}
                </p>
              ) : (
                <p className="text-sm text-on-surface-variant">No active assignment</p>
              )}
            </div>
          </section>

          <section>
            <h2 className="mb-4 text-lg font-semibold">Recent Announcements</h2>
            <div className="flex flex-col gap-3">
              {announcements.length === 0 ? (
                <p className="text-sm text-on-surface-variant">No announcements yet.</p>
              ) : (
                announcements.map((a) => (
                  <div
                    key={a.id}
                    className="flex gap-4 rounded-xl border border-border-subtle bg-surface-container p-4"
                  >
                    <div className="flex h-12 w-14 shrink-0 items-center justify-center rounded-lg bg-surface-container-high text-xs font-bold text-primary">
                      {a.published_at ? formatShortDate(a.published_at) : '—'}
                    </div>
                    <div>
                      <h4 className="font-semibold">{a.title}</h4>
                      <p className="mt-1 text-sm text-on-surface-variant">
                        {a.snippet || a.body.slice(0, 120)}
                      </p>
                    </div>
                  </div>
                ))
              )}
            </div>
            <Link to="/announcements" className="mt-3 inline-block text-sm text-primary hover:underline">
              View all announcements
            </Link>
          </section>
        </div>
      </QueryState>
    </AuthGate>
  )
}
