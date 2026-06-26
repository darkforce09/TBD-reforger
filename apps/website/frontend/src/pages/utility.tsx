import { useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { toast } from 'sonner'
import { AdminGate } from '@/components/AdminGate'
import { MaterialIcon } from '@/components/MaterialIcon'
import { SplitPane } from '@/components/ui/split-pane'
import { cn } from '@/lib/utils'

// ─── Server Control ────────────────────────────────────────────────────────
// macOS tactical command center: a server selector on the left, command +
// telemetry + a live RCON console on the right. Mock-driven for now — swap
// MOCK_SERVERS for `useServers()` + `useServerTelemetry()` and wire the action
// buttons / console input to `useServerRcon()` once managing multiple servers
// is supported end-to-end.

type ServerStatus = 'online' | 'starting' | 'offline'

interface ManagedServer {
  id: string
  name: string
  ip: string
  status: ServerStatus
  players: number
  maxPlayers: number
  uptime: string
  terrain: string
  mission: string
  fps: number
  modConfig: string
  log: string[]
}

const STATUS_META: Record<ServerStatus, { dot: string; label: string; pulse: boolean }> = {
  online: { dot: 'bg-success', label: 'Online', pulse: true },
  starting: { dot: 'bg-tactical-yellow', label: 'Starting', pulse: true },
  offline: { dot: 'bg-outline', label: 'Offline', pulse: false },
}

const MOCK_SERVERS: ManagedServer[] = [
  {
    id: 'main',
    name: 'TBD Main Server',
    ip: '198.51.100.24:2001',
    status: 'online',
    players: 55,
    maxPlayers: 64,
    uptime: '3d 14h 22m',
    terrain: 'Everon',
    mission: 'Operation Iron Veil',
    fps: 44,
    modConfig: 'Core Modern v2.4.1',
    log: [
      '[12:00:01] Server initialized on Everon',
      '[12:00:02] Loading modpack: Core Modern v2.4.1 (6 mods)',
      '[12:00:05] RCON listener bound to 0.0.0.0:19999',
      '[12:01:10] Player "Reaper" connected (76561198000000001)',
      '[12:03:42] Player "Hawk" connected (76561198000000002)',
      '[12:07:15] Mission "Operation Iron Veil" started',
    ],
  },
  {
    id: 'training',
    name: 'TBD Training Ground',
    ip: '198.51.100.24:2011',
    status: 'online',
    players: 8,
    maxPlayers: 32,
    uptime: '1d 02h 09m',
    terrain: 'Arland',
    mission: 'Live-Fire Range',
    fps: 60,
    modConfig: 'Training Pack v1.0',
    log: [
      '[09:30:00] Server initialized on Arland',
      '[09:30:03] Loading modpack: Training Pack v1.0 (3 mods)',
      '[09:30:06] RCON listener bound to 0.0.0.0:19998',
      '[09:45:11] Player "Recruit-04" connected',
    ],
  },
  {
    id: 'event-01',
    name: 'Event Server 01',
    ip: '198.51.100.25:2021',
    status: 'starting',
    players: 0,
    maxPlayers: 64,
    uptime: '00h 00m',
    terrain: 'Everon',
    mission: '—',
    fps: 0,
    modConfig: 'Event Pack v0.9',
    log: [
      '[20:00:00] Boot sequence started',
      '[20:00:02] Loading modpack: Event Pack v0.9 (11 mods)…',
      '[20:00:04] Waiting for mod downloads to complete…',
    ],
  },
]

export function ServerControlPage() {
  const [selectedId, setSelectedId] = useState(MOCK_SERVERS[0].id)
  const server = MOCK_SERVERS.find((s) => s.id === selectedId) ?? MOCK_SERVERS[0]

  return (
    <AdminGate>
      <div className="relative h-full w-full overflow-hidden">
        {/* Global topo-map background */}
        <div className="bg-topo-map bg-grid-overlay absolute inset-0 z-0" />
        <div className="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
          <SplitPane
            transparent
            masterWidth="17rem"
            masterHeader={
              <h1 className="w-full text-label-md font-semibold tracking-wide text-on-surface uppercase">
                Servers
                <span className="ml-2 font-mono text-code-md text-outline">{MOCK_SERVERS.length}</span>
              </h1>
            }
            master={MOCK_SERVERS.map((s) => {
              const meta = STATUS_META[s.status]
              const active = s.id === server.id
              return (
                <button
                  key={s.id}
                  type="button"
                  onClick={() => setSelectedId(s.id)}
                  className={cn(
                    'flex items-center gap-3 rounded-lg border-l-4 px-3 py-3 text-left transition-all duration-200',
                    active
                      ? 'border-primary bg-primary/15'
                      : 'border-transparent hover:bg-white/[0.03]',
                  )}
                >
                  <span className="relative flex size-2.5 shrink-0">
                    {meta.pulse && (
                      <span
                        className={cn(
                          'absolute inline-flex h-full w-full animate-ping rounded-full opacity-60',
                          meta.dot,
                        )}
                      />
                    )}
                    <span className={cn('relative inline-flex size-2.5 rounded-full', meta.dot)} />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span
                      className={cn(
                        'block truncate text-label-md font-medium',
                        active ? 'text-on-surface' : 'text-on-surface-variant',
                      )}
                    >
                      {s.name}
                    </span>
                    <span className="block font-mono text-code-md text-outline">{meta.label}</span>
                  </span>
                </button>
              )
            })}
            detail={
              <div className="flex h-full min-w-0 flex-1 flex-col">
                {/* Command header */}
                <header className="flex flex-wrap items-center justify-between gap-4 border-b border-white/5 p-6 pb-6">
            <div className="min-w-0">
              <h2 className="truncate text-headline-lg text-on-surface">{server.name}</h2>
              <div className="mt-2 inline-flex items-center gap-2 rounded-full bg-white/5 px-3 py-1">
                <MaterialIcon name="lan" className="text-[16px] text-on-surface-variant" />
                <span className="font-mono text-code-md text-on-surface">{server.ip}</span>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => toast.message(`Restart requested for ${server.name}`)}
                className="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2.5 text-label-md text-on-surface transition hover:bg-white/5"
              >
                <MaterialIcon name="restart_alt" className="text-[18px]" />
                Restart
              </button>
              <button
                type="button"
                onClick={() => toast.message(`Stop requested for ${server.name}`)}
                className="flex items-center gap-1.5 rounded-full border border-error-alert/30 px-4 py-2.5 text-label-md text-error-alert transition hover:bg-error-alert/10"
              >
                <MaterialIcon name="stop" className="text-[18px]" />
                Stop
              </button>
              <button
                type="button"
                onClick={() => toast.success(`Connecting to ${server.ip}…`)}
                className="flex items-center gap-2 rounded-full bg-action px-6 py-2.5 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
              >
                <MaterialIcon name="rocket_launch" className="text-[18px]" />
                LAUNCH &amp; CONNECT
              </button>
            </div>
          </header>

          {/* Telemetry grid */}
          <div className="grid shrink-0 grid-cols-3 divide-x divide-white/10 border-b border-white/5">
            <TelemetryCol
              primaryLabel="Active Personnel"
              primaryValue={`${server.players} / ${server.maxPlayers}`}
              secondaryLabel="Uptime"
              secondaryValue={server.uptime}
            />
            <TelemetryCol
              primaryLabel="Terrain"
              primaryValue={server.terrain}
              secondaryLabel="Active Mission"
              secondaryValue={server.mission}
            />
            <TelemetryCol
              primaryLabel="Server FPS"
              primaryValue={`${server.fps} Hz`}
              secondaryLabel="Mod Configuration"
              secondaryValue={server.modConfig}
            />
          </div>

          {/* RCON console — fills the bottom half */}
                <RconConsole key={server.id} server={server} />
              </div>
            }
          />
        </div>
      </div>
    </AdminGate>
  )
}

function TelemetryCol({
  primaryLabel,
  primaryValue,
  secondaryLabel,
  secondaryValue,
}: {
  primaryLabel: string
  primaryValue: string
  secondaryLabel: string
  secondaryValue: string
}) {
  return (
    <div className="px-6 py-6">
      <p className="font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
        {primaryLabel}
      </p>
      <p className="mt-1 truncate font-mono text-3xl font-bold tracking-tight text-on-surface">
        {primaryValue}
      </p>
      <p className="mt-4 font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
        {secondaryLabel}
      </p>
      <p className="mt-1 truncate text-label-md text-on-surface">{secondaryValue}</p>
    </div>
  )
}

// Common RCON tasks surfaced as one-click buttons above the console.
const QUICK_ACTIONS: { label: string; icon: string; cmd: string }[] = [
  { label: 'Change Map', icon: 'map', cmd: 'rcon map_change everon' },
  { label: 'Swap Modpack', icon: 'extension', cmd: 'rcon modpack_load core_modern_v2.4.1' },
  {
    label: 'Global Broadcast',
    icon: 'campaign',
    cmd: 'rcon say -1 "All units: stand by for orders"',
  },
  { label: 'Force Restart', icon: 'restart_alt', cmd: 'rcon #restart --force' },
]

/** Live RCON terminal — mock output; echoes sent commands locally. */
function RconConsole({ server }: { server: ManagedServer }) {
  const [lines, setLines] = useState<string[]>(server.log)
  const [command, setCommand] = useState('')
  const endRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [lines])

  const runCommand = (cmd: string) => {
    const trimmed = cmd.trim()
    if (!trimmed) return
    const now = new Date().toLocaleTimeString('en-GB')
    setLines((prev) => [...prev, `$ ${trimmed}`, `[${now}] RCON: '${trimmed}' acknowledged`])
  }

  const send = () => {
    runCommand(command)
    setCommand('')
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col bg-surface/40">
      {/* Quick Actions toolbar */}
      <div className="flex flex-wrap items-center gap-3 border-b border-white/5 bg-surface-container/30 p-4">
        <span className="text-label-sm tracking-wider text-on-surface-variant uppercase">
          Quick Actions:
        </span>
        {QUICK_ACTIONS.map((a) => (
          <button
            key={a.label}
            type="button"
            onClick={() => runCommand(a.cmd)}
            className="flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface backdrop-blur-md transition hover:bg-white/10"
          >
            <MaterialIcon name={a.icon} className="text-[16px] text-on-surface-variant" />
            {a.label}
          </button>
        ))}
      </div>

      {/* Console */}
      <div className="flex min-h-0 flex-1 flex-col p-6">
        <div className="mb-3 flex items-center gap-2">
          <MaterialIcon name="terminal" className="text-[18px] text-on-surface-variant" />
          <h3 className="text-label-md font-semibold tracking-wide text-on-surface uppercase">
            RCON Console
          </h3>
        </div>
        <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto rounded-xl border border-white/5 bg-black/30 p-4 font-mono text-sm leading-relaxed text-on-surface-variant">
          {lines.map((line, i) => (
            <p
              key={i}
              className={cn(
                'whitespace-pre-wrap',
                line.startsWith('$') && 'text-primary',
                line.includes('RCON:') && 'text-success',
              )}
            >
              {line}
            </p>
          ))}
          <div ref={endRef} />
        </div>
        <div className="mt-3 flex items-center gap-2 rounded-full border border-white/10 bg-white/5 py-1.5 pr-1.5 pl-5 focus-within:border-primary/40">
          <span className="font-mono text-sm text-on-surface-variant/60">$</span>
          <input
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault()
                send()
              }
            }}
            placeholder="Send RCON command…"
            className="flex-1 bg-transparent font-mono text-sm text-on-surface placeholder:text-on-surface-variant/50 outline-none"
          />
          <button
            type="button"
            onClick={send}
            aria-label="Send command"
            className="flex size-9 items-center justify-center rounded-full bg-primary text-on-primary transition hover:bg-primary/80"
          >
            <MaterialIcon name="arrow_upward" className="text-[20px]" />
          </button>
        </div>
      </div>
    </section>
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
