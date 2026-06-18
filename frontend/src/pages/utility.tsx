import { useState } from 'react'
import { Link } from 'react-router-dom'
import { toast } from 'sonner'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AdminGate } from '@/components/AdminGate'
import { QueryState } from '@/components/QueryState'
import { useServers } from '@/hooks/queries'
import { useServerRcon } from '@/hooks/mutations'
import { pickDefaultServer } from '@/lib/defaultServer'
import { useServerTelemetry } from '@/hooks/useServerTelemetry'

export function ServerControlPage() {
  const { data: servers, isLoading, isError, error } = useServers()
  const server = servers ? pickDefaultServer(servers) : undefined
  const { status } = useServerTelemetry(server?.id)
  const rcon = useServerRcon()
  const [command, setCommand] = useState('')
  const [consoleLog, setConsoleLog] = useState<string[]>([])

  const live = status ?? server?.status
  const isOnline = live?.is_online ?? false

  const runRcon = (action: 'restart' | 'change_map' | 'custom', map?: string, cmd?: string) => {
    if (!server) return
    rcon.mutate(
      { serverId: server.id, action, map, command: cmd },
      {
        onSuccess: (data) => {
          const line = typeof data === 'object' && data && 'output' in data
            ? String((data as { output?: string }).output)
            : JSON.stringify(data)
          setConsoleLog((prev) => [...prev, `[${action}] ${line}`])
          toast.success('RCON command sent')
        },
        onError: () => toast.error('RCON command failed'),
      },
    )
  }

  return (
    <AdminGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="mx-auto w-full max-w-4xl">
          <PageHeader
            title="Server Control"
            subtitle={`RCON panel for ${server?.name ?? 'server'} — restart, map change, live console.`}
          />
          {!server ? (
            <p className="text-on-surface-variant">No servers configured.</p>
          ) : (
            <div className="grid gap-6 lg:grid-cols-2">
              <OpsCard className="bg-surface-container-high">
                <h2 className="mb-4 text-lg font-semibold">{server.name}</h2>
                <p className="mb-4 text-sm text-on-surface-variant">
                  Status:{' '}
                  <span className={isOnline ? 'text-success' : 'text-error'}>
                    {isOnline ? 'Online' : 'Offline'}
                  </span>
                  {live && (
                    <span className="ml-2">
                      — {live.player_count}/{live.max_players} players
                    </span>
                  )}
                </p>
                <div className="flex flex-col gap-2">
                  <button
                    type="button"
                    onClick={() => runRcon('restart')}
                    disabled={rcon.isPending}
                    className="rounded-lg bg-primary py-2 text-sm text-on-primary disabled:opacity-50"
                  >
                    Restart Server
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      const map = window.prompt('Map name:')
                      if (map) runRcon('change_map', map)
                    }}
                    disabled={rcon.isPending}
                    className="rounded-lg border border-border-subtle py-2 text-sm disabled:opacity-50"
                  >
                    Change Map
                  </button>
                </div>
              </OpsCard>
              <OpsCard className="bg-surface-container-high">
                <h2 className="mb-4 text-lg font-semibold">RCON Console</h2>
                <div className="mb-2 flex gap-2">
                  <input
                    type="text"
                    value={command}
                    onChange={(e) => setCommand(e.target.value)}
                    placeholder="Custom command..."
                    className="flex-1 rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
                  />
                  <button
                    type="button"
                    onClick={() => {
                      if (command.trim()) {
                        runRcon('custom', undefined, command.trim())
                        setCommand('')
                      }
                    }}
                    disabled={rcon.isPending}
                    className="rounded-lg bg-primary px-3 py-2 text-sm text-on-primary disabled:opacity-50"
                  >
                    Send
                  </button>
                </div>
                <pre className="h-48 overflow-auto rounded-lg bg-[#0a0e18] p-3 font-mono text-xs text-on-surface-variant">
                  {consoleLog.length === 0
                    ? '[RCON] Ready — send a command to begin.'
                    : consoleLog.join('\n')}
                </pre>
              </OpsCard>
            </div>
          )}
          <Link to="/" className="mt-6 inline-block text-primary hover:underline">
            Return to Dashboard
          </Link>
        </div>
      </QueryState>
    </AdminGate>
  )
}

export function NotFoundPage() {
  return (
    <div className="flex flex-col items-center justify-center py-24 text-center">
      <span className="text-6xl font-bold text-primary">404</span>
      <h1 className="mt-4 text-2xl font-bold">Sector Not Found</h1>
      <p className="mt-2 text-on-surface-variant">The requested route does not exist in this AO.</p>
      <Link to="/" className="mt-6 text-primary hover:underline">
        Return to Dashboard
      </Link>
    </div>
  )
}
