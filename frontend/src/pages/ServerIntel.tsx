import type { ReactNode } from 'react'
import { toast } from 'sonner'
import { MaterialIcon } from '@/components/MaterialIcon'
import { OpsCard } from '@/components/OpsCard'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { useServers } from '@/hooks/queries'
import { useServerTelemetry } from '@/hooks/useServerTelemetry'
import { pickDefaultServer } from '@/lib/defaultServer'
import { formatBytes, formatUptime } from '@/lib/format'
import { cn } from '@/lib/utils'

function IntelField({
  label,
  children,
  className,
}: {
  label: string
  children: ReactNode
  className?: string
}) {
  return (
    <div className={cn('rounded-lg border border-border-subtle bg-surface-container p-4', className)}>
      <span className="mb-1 block text-xs font-semibold tracking-widest text-on-surface-variant uppercase">
        {label}
      </span>
      {children}
    </div>
  )
}

export function ServerIntelPage() {
  const { data: servers, isLoading, isError, error } = useServers()
  const server = servers ? pickDefaultServer(servers) : undefined
  const { status, isConnected } = useServerTelemetry(server?.id)

  const live = status ?? server?.status
  const modpack = server?.required_modpack
  const isOnline = live?.is_online ?? false
  const playerPct =
    live && live.max_players > 0
      ? Math.round((live.player_count / live.max_players) * 100)
      : 0
  const connectAddress = server ? `${server.ip} : ${server.port}` : '—'

  const copyAddress = async () => {
    if (!server) return
    try {
      await navigator.clipboard.writeText(`${server.ip}:${server.port}`)
      toast.success('Server address copied')
    } catch {
      toast.error('Could not copy to clipboard')
    }
  }

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-[var(--spacing-container-max)]">
          <header className="mb-8">
            <h1 className="mb-2 text-3xl font-bold text-on-surface">Server Intel</h1>
            <p className="text-on-surface-variant">
              Real-time server health and deployment telemetry for{' '}
              <span className="font-medium text-on-surface">{server?.name ?? '—'}</span>
              {isConnected && (
                <span className="ml-2 text-xs text-success">(live stream)</span>
              )}
            </p>
          </header>

          {!server ? (
            <p className="text-on-surface-variant">No servers configured.</p>
          ) : (
            <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
              <OpsCard className="h-full border-primary/30 bg-surface-container-high">
                <header className="mb-4 flex items-center justify-between border-b border-border-subtle pb-4">
                  <h2 className="text-lg font-semibold">Server Status</h2>
                  <div className="flex items-center gap-2">
                    <div
                      className={cn(
                        'pulse-dot h-2.5 w-2.5 rounded-full',
                        isOnline ? 'bg-success' : 'bg-error',
                      )}
                    />
                    <span
                      className={cn(
                        'text-xs font-semibold tracking-widest uppercase',
                        isOnline ? 'text-success' : 'text-error',
                      )}
                    >
                      {isOnline ? 'Online' : 'Offline'}
                    </span>
                  </div>
                </header>
                <div className="flex flex-col gap-4">
                  <IntelField label="Uptime">
                    <span className="font-mono text-sm tracking-widest text-primary">
                      {live ? formatUptime(live.uptime_seconds) : '—'}
                    </span>
                  </IntelField>
                  <IntelField label="Player Count">
                    <div className="flex items-center justify-between">
                      <span className="text-lg font-semibold">
                        {live ? `${live.player_count} / ${live.max_players}` : '—'}
                      </span>
                      <span className="text-sm text-on-surface-variant">Personnel</span>
                    </div>
                    <div className="mt-3 h-1.5 w-full overflow-hidden rounded-full bg-surface-container-highest">
                      <div
                        className="h-full rounded-full bg-primary"
                        style={{ width: `${playerPct}%` }}
                      />
                    </div>
                  </IntelField>
                  <IntelField label="Server Performance" className="mt-auto">
                    <div className="flex items-center gap-2">
                      <MaterialIcon name="speed" className="text-lg text-success" />
                      <span className={live && live.server_fps < 20 ? 'text-warning' : 'text-success'}>
                        {live ? `${live.server_fps} Server FPS` : '—'}
                      </span>
                    </div>
                  </IntelField>
                </div>
              </OpsCard>

              <OpsCard className="h-full border-primary/30 bg-surface-container-high">
                <header className="mb-4 border-b border-border-subtle pb-4">
                  <h2 className="text-lg font-semibold">Active Deployment</h2>
                </header>
                <div className="relative mb-4 flex h-32 items-center justify-center overflow-hidden rounded-lg border border-border-subtle bg-surface-container">
                  <MaterialIcon name="map" className="text-4xl text-on-surface-variant" />
                  {live?.current_match_id && (
                    <span className="absolute bottom-3 left-3 rounded border border-border-subtle bg-surface-container-highest/80 px-2 py-1 text-xs font-semibold tracking-widest backdrop-blur-sm">
                      Match {live.current_match_id.slice(0, 8)}
                    </span>
                  )}
                </div>
                <ul className="flex flex-col gap-2">
                  {[
                    {
                      icon: 'map',
                      label: 'Mission',
                      value: live?.current_match_id ? 'Active match' : 'No active match',
                    },
                    {
                      icon: 'schedule',
                      label: 'In-Game Time',
                      value: live?.ingame_time ?? '—',
                    },
                    {
                      icon: 'cloud',
                      label: 'Weather Conditions',
                      value: live?.ingame_weather ?? '—',
                    },
                  ].map((row) => (
                    <li
                      key={row.label}
                      className="flex items-start gap-3 rounded-lg border border-border-subtle bg-surface-container p-3"
                    >
                      <MaterialIcon name={row.icon} className="mt-0.5 text-primary" />
                      <div>
                        <span className="block text-xs font-semibold tracking-widest text-on-surface-variant uppercase">
                          {row.label}
                        </span>
                        <span className="text-on-surface">{row.value}</span>
                      </div>
                    </li>
                  ))}
                </ul>
              </OpsCard>

              <OpsCard className="h-full border-primary/30 bg-surface-container-high">
                <header className="mb-4 border-b border-border-subtle pb-4">
                  <h2 className="text-lg font-semibold">Connection Gateway</h2>
                </header>
                <div className="flex flex-1 flex-col gap-4">
                  <div className="flex flex-1 flex-col justify-center">
                    <div className="relative mb-4 rounded-lg border border-border-subtle bg-background p-4">
                      <span className="mb-2 block text-xs font-semibold tracking-widest text-on-surface-variant uppercase">
                        IP / Port
                      </span>
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-mono text-sm tracking-widest text-primary">
                          {connectAddress}
                        </span>
                        <button
                          type="button"
                          onClick={copyAddress}
                          className="rounded p-1 text-on-surface-variant transition-colors hover:text-primary"
                          title="Copy to clipboard"
                        >
                          <MaterialIcon name="content_copy" className="text-sm" />
                        </button>
                      </div>
                    </div>
                    <IntelField label="Required Modpack">
                      <div className="flex items-center gap-2">
                        <MaterialIcon name="extension" className="text-lg text-primary" />
                        <span>
                          {modpack
                            ? `${modpack.name} v${modpack.version}`
                            : 'No modpack required'}
                        </span>
                        {modpack?.is_current && (
                          <span className="ml-auto rounded border border-green-700/50 bg-success-muted px-2 py-0.5 text-xs font-semibold tracking-widest text-on-surface">
                            Verified
                          </span>
                        )}
                      </div>
                      {modpack && (
                        <p className="mt-1 text-xs text-on-surface-variant">
                          {formatBytes(modpack.total_size_bytes)} — {modpack.mods.length} mods
                        </p>
                      )}
                    </IntelField>
                  </div>
                  <button
                    type="button"
                    className="mt-auto flex w-full items-center justify-center gap-2 rounded-lg bg-primary py-3 text-sm font-medium text-on-primary shadow-[0_0_15px_rgba(59,130,246,0.3)] transition-colors hover:bg-primary/90"
                    onClick={() => toast.message('Launch requires the Reforger client')}
                  >
                    <MaterialIcon name="play_arrow" className="text-lg" />
                    Launch Reforger &amp; Connect
                  </button>
                </div>
              </OpsCard>
            </div>
          )}
        </div>
      </QueryState>
    </AuthGate>
  )
}
