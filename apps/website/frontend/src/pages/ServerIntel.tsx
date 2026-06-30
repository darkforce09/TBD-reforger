import { Link } from 'react-router-dom'
import { toast } from 'sonner'
import { MaterialIcon } from '@/components/MaterialIcon'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { useServers } from '@/hooks/queries'
import { useServerTelemetry } from '@/hooks/useServerTelemetry'
import { pickDefaultServer } from '@/lib/defaultServer'
import { formatUptime } from '@/lib/format'
import { cn } from '@/lib/utils'

/** Static cinematic terrain backdrop until the API exposes per-terrain imagery. */
const THEATER_IMAGE =
  'https://lh3.googleusercontent.com/aida-public/AB6AXuBJhklFaKKJXQ3-uOGwrugGr_URw1Dq_3Jslvkc3lEtT4ObLWKv52ipE-EQWEm3QF4HeoY5vA8NcYt_e87d76A14Z48tuHODNidNphecUVm_Zy7NLBRexvt9uUcFOBLTk3RbiSAetUEMYX2BmQMPU-BU-HvmweLf1P4-jc1CjC0jDdMMR-fzb5BVtNID-Ak1iW3MuGzWiO4LfZ4WIPy8Ijk3kcsqRFXVroQ_rZSJ8yw4se-gszeDoVOc8Vp9HL5qLcEAtnI4pFEC4I'

/** Global tactical map backdrop — gives the glass panels something to frost. */
const COMMAND_MAP_IMAGE =
  'https://lh3.googleusercontent.com/aida-public/AB6AXuBqY9NRsaLKSRk7V0g9XrVkysuxuTRsc8FcMfq76JZujkDPkAAihMyRIw6mOuvFI4tTOwRDvDEhOe-p2Coym8zpmONJeueKLL379Yzecw64o3wzqJMRZdGCA7iBbwrno1hge-AU7AZNCE4XVo9q6IXTH5A2NRf3IToSchzAuj5JUT-Y81VVXfb-Ic4CrnLbV_So9xy2vBIxVHrwDztZ-YuY78DL-Jb5qsgNACRmxHXgRYRrsCxsCJnHBrgj-DD3LUVa31rIo4Arzrc'

// eslint-disable-next-line complexity -- intel page render: loading/error/empty + per-card status/setting branches
export function ServerIntelPage() {
  const { data: servers, isLoading, isError, error } = useServers()
  const server = servers ? pickDefaultServer(servers) : undefined
  const { status } = useServerTelemetry(server?.id)

  const live = status ?? server?.status
  const modpack = server?.required_modpack
  const isOnline = live?.is_online ?? false
  const connectAddress = server ? `${server.ip} : ${server.port}` : '—'

  // `terrain` is not yet part of the server API contract; read it defensively so
  // the overlay populates automatically once the backend surfaces it.
  const terrainName = (server as { terrain?: string } | undefined)?.terrain ?? 'Theater Unknown'
  const missionName = live?.current_match_id
    ? `Match ${live.current_match_id.slice(0, 8)}`
    : 'No Active Mission'

  const fpsOptimal = (live?.server_fps ?? 0) >= 30

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
        <div className="relative h-full overflow-y-auto">
          {/* Global satellite-map backdrop — the surface the glass panel frosts. */}
          <div
            className="absolute inset-0 z-0 bg-cover bg-center"
            style={{ backgroundImage: `url('${COMMAND_MAP_IMAGE}')` }}
          >
            <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" />
            <div className="absolute inset-0 bg-gradient-to-t from-background via-transparent to-transparent" />
          </div>

          <div className="relative z-10 flex w-full flex-col">
            {!server ? (
              <p className="text-on-surface-variant">No servers configured.</p>
            ) : (
              <div className="flex w-full flex-col overflow-hidden bg-surface-glass backdrop-blur-xl">
                {/* Panel Header */}
                <div className="flex flex-col justify-between gap-6 border-b border-white/5 bg-surface/40 px-8 py-6 md:flex-row md:items-center">
                  <div>
                    <div className="mb-2 flex items-center gap-3">
                      <div
                        className={cn(
                          'pulse-dot h-2.5 w-2.5 rounded-full',
                          isOnline ? 'bg-success' : 'bg-tactical-yellow',
                        )}
                        title={isOnline ? 'Server Online' : 'Server Offline'}
                      />
                      <h2 className="text-headline-md uppercase tracking-wider text-on-surface">
                        {server.name}
                      </h2>
                    </div>
                    <div className="inline-flex items-center gap-2 rounded-md border border-white/5 bg-surface-container px-3 py-1.5 text-code-md text-on-surface-variant">
                      <MaterialIcon name="dns" className="text-[16px]" />
                      <span>{connectAddress}</span>
                      <button
                        type="button"
                        onClick={copyAddress}
                        aria-label="Copy IP"
                        className="ml-2 transition-colors hover:text-primary"
                      >
                        <MaterialIcon name="content_copy" className="text-[16px]" />
                      </button>
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => toast.message('Launch requires the Reforger client')}
                    className="flex shrink-0 items-center gap-2 rounded-full border border-secondary-container/50 bg-secondary-container px-6 py-3 text-label-md text-on-secondary-container transition-all duration-300 hover:shadow-[0_0_20px_rgba(5,102,217,0.4)]"
                  >
                    <MaterialIcon name="play_arrow" filled />
                    LAUNCH &amp; CONNECT
                  </button>
                </div>

                {/* Telemetry Grid */}
                <div className="grid grid-cols-1 gap-8 border-b border-white/5 p-8 md:grid-cols-[1fr_2fr_1fr] md:divide-x md:divide-white/10">
                  {/* Column 1: Performance */}
                  <div className="flex flex-col justify-center space-y-4 md:pr-8">
                    <div>
                      <span className="mb-1 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                        Active Personnel
                      </span>
                      <div className="flex items-baseline gap-2">
                        <span className="font-mono text-[30px] font-bold leading-tight text-tertiary-container">
                          {live?.player_count ?? 0}
                        </span>
                        <span className="font-mono text-[20px] font-semibold text-on-surface-variant">
                          / {live?.max_players ?? 0}
                        </span>
                      </div>
                    </div>
                    <div className="space-y-2 pt-2">
                      <div className="flex items-center justify-between text-code-md text-on-surface-variant">
                        <span>Uptime:</span>
                        <span>{live ? formatUptime(live.uptime_seconds) : '—'}</span>
                      </div>
                      <div className="flex items-center justify-between text-code-md text-on-surface-variant">
                        <span>Server FPS:</span>
                        <span className={fpsOptimal ? 'text-tactical-yellow' : 'text-error'}>
                          {live ? `${live.server_fps} (${fpsOptimal ? 'Optimal' : 'Low'})` : '—'}
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Column 2: Theater of Operations */}
                  <div className="flex flex-col justify-center md:px-8">
                    <span className="mb-3 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                      Theater of Operations
                    </span>
                    <Link to="/events" className="block focus:outline-none">
                      <div className="group relative aspect-[21/9] w-full cursor-pointer overflow-hidden rounded-lg border border-white/10 transition-all duration-300 hover:ring-2 hover:ring-primary hover:ring-offset-2 hover:ring-offset-background">
                        <img
                          alt={`${terrainName} terrain`}
                          src={THEATER_IMAGE}
                          className="h-full w-full object-cover transition-transform duration-700 group-hover:scale-105"
                        />
                        <div className="absolute inset-0 bg-gradient-to-t from-surface-container-highest/90 via-surface-container-highest/20 to-transparent" />
                        <div className="absolute bottom-3 left-3 right-3 flex items-center justify-between">
                          <div>
                            <span className="block text-label-md text-on-surface">
                              {terrainName}
                            </span>
                            <span className="mt-0.5 block text-label-sm text-primary">
                              {missionName}
                            </span>
                          </div>
                          <MaterialIcon name="map" className="text-on-surface-variant" />
                        </div>
                      </div>
                    </Link>
                  </div>

                  {/* Column 3: Environment & Mods */}
                  <div className="flex flex-col justify-center space-y-6 md:pl-8">
                    <div className="flex items-center gap-4">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/5 bg-surface-container">
                        <MaterialIcon name="schedule" className="text-primary" />
                      </div>
                      <div>
                        <span className="block text-label-sm uppercase text-on-surface-variant">
                          Simulated Time
                        </span>
                        <span className="text-body-md text-on-surface">
                          {live?.ingame_time ?? '—'}
                        </span>
                      </div>
                    </div>
                    <div className="flex items-center gap-4">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/5 bg-surface-container">
                        <MaterialIcon name="rainy" className="text-tertiary-container" />
                      </div>
                      <div>
                        <span className="block text-label-sm uppercase text-on-surface-variant">
                          Conditions
                        </span>
                        <span className="text-body-md text-on-surface">
                          {live?.ingame_weather ?? '—'}
                        </span>
                      </div>
                    </div>
                    <div className="flex items-center gap-4">
                      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/5 bg-surface-container">
                        <MaterialIcon name="verified" className="text-tactical-yellow" />
                      </div>
                      <div>
                        <span className="block text-label-sm uppercase text-on-surface-variant">
                          Mod Configuration
                        </span>
                        <span className="text-body-md text-on-surface">
                          {modpack ? (
                            <>
                              {modpack.name} v{modpack.version}{' '}
                              {modpack.is_current && (
                                <span className="text-[12px] text-on-surface-variant">
                                  (Synced)
                                </span>
                              )}
                            </>
                          ) : (
                            'No modpack required'
                          )}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>

                {/* Recent Intelligence */}
                <div className="bg-surface/20 p-8">
                  <span className="mb-4 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                    Recent Intelligence
                  </span>
                  <div className="space-y-3">
                    <div className="flex items-center gap-4 border-b border-white/5 py-2 text-code-md">
                      <span className="shrink-0 text-primary">[14:02:00Z]</span>
                      <span className="text-on-surface">
                        New hostile movement detected in Sector 4
                      </span>
                    </div>
                    <div className="flex items-center gap-4 border-b border-white/5 py-2 text-code-md">
                      <span className="shrink-0 text-primary">[13:45:12Z]</span>
                      <span className="text-on-surface">Server Uplink maintenance completed</span>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </QueryState>
    </AuthGate>
  )
}
