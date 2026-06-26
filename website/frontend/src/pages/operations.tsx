import { useState } from 'react'
import { Link } from 'react-router-dom'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { MaterialIcon } from '@/components/MaterialIcon'
import { SplitPane, SplitPaneEmpty } from '@/components/ui/split-pane'
import { ListDetailItem } from '@/components/ui/list-detail-item'
import { Badge } from '@/components/ui/badge'
import { useAuthStore } from '@/store/useAuthStore'
import { DEFAULT_AVATAR } from '@/lib/avatar'
import type { Announcement, LeaderboardRow } from '@/types/api'
import {
  useAnnouncements,
  useDeployments,
  useEvent,
  useEvents,
  useLeaderboards,
} from '@/hooks/queries'
import { EventHubView } from '@/pages/events'
import {
  countdownLabel,
  formatLocalDateTime,
  formatShortDate,
  tagLabel,
} from '@/lib/format'
import { cn } from '@/lib/utils'

const LEADERBOARD_TABS = [
  { label: 'K/D Ratio', category: 'kd' },
  { label: 'Command Win Rate', category: 'command_win' },
  { label: 'Missions Played', category: 'missions' },
  { label: 'Longest Kill', category: 'longest_kill' },
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
        <div className="relative h-full w-full overflow-hidden">
          {/* Global Topo Map Background */}
          <div className="bg-topo-map bg-grid-overlay absolute inset-0 z-0" />

          {/* Frosted Glass Encasing */}
          <div className="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
            <SplitPane
              transparent
              masterHeader={
                <>
                  <h2 className="text-headline-sm tracking-wide text-on-surface uppercase">
                    Comms Link
                  </h2>
                  <MaterialIcon name="filter_list" className="text-outline" />
                </>
              }
              master={
                posts.length === 0 ? (
                  <p className="px-1 py-4 text-label-md text-on-surface-variant">
                    No announcements yet.
                  </p>
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
          </div>
        </div>
      </QueryState>
    </AuthGate>
  )
}

function AnnouncementDetail({ post }: { post: Announcement }) {
  return (
    <div className="relative flex h-full flex-col overflow-hidden bg-surface-container-highest/10">
      <div className="z-10 shrink-0 border-b border-outline-variant/20 bg-surface-container-low/40 px-8 py-10 backdrop-blur-md md:px-12">
        <div className="mb-4 flex flex-wrap items-center gap-3">
          {post.is_pinned && (
            <span className="font-code-md flex items-center gap-1 rounded border border-tactical-yellow/30 bg-tactical-yellow/10 px-2 py-1 text-tactical-yellow">
              <MaterialIcon name="push_pin" className="text-[14px]" />
              [PINNED]
            </span>
          )}
          <span className="font-code-md rounded border border-primary/30 bg-primary/10 px-2 py-1 text-primary uppercase">
            [{tagLabel(post.tag)}]
          </span>
          {post.published_at && (
            <span className="font-code-md ml-auto text-outline uppercase">
              AUTHORITY: COMMAND // TS: {formatLocalDateTime(post.published_at)}
            </span>
          )}
        </div>
        <h1 className="text-headline-lg font-bold tracking-tight text-on-surface">{post.title}</h1>
      </div>

      <div className="relative flex-1 overflow-y-auto px-8 py-8 md:px-12">
        <div className="mx-auto max-w-4xl space-y-8 pb-20">
          {post.thumbnail_url && (
            <div className="group relative h-80 w-full overflow-hidden rounded-xl border border-outline-variant/30 shadow-2xl">
              {/* Tactical Corners */}
              <div className="absolute top-0 left-0 z-10 m-2 h-4 w-4 border-t-2 border-l-2 border-primary opacity-50" />
              <div className="absolute top-0 right-0 z-10 m-2 h-4 w-4 border-t-2 border-r-2 border-primary opacity-50" />
              <div className="absolute bottom-0 left-0 z-10 m-2 h-4 w-4 border-b-2 border-l-2 border-primary opacity-50" />
              <div className="absolute bottom-0 right-0 z-10 m-2 h-4 w-4 border-b-2 border-r-2 border-primary opacity-50" />

              <img
                src={post.thumbnail_url}
                alt=""
                className="h-full w-full object-cover opacity-80 mix-blend-luminosity transition-all duration-700 group-hover:opacity-100 group-hover:mix-blend-normal"
              />
              <div className="absolute inset-0 bg-gradient-to-t from-surface-container-lowest/80 via-transparent to-transparent" />
            </div>
          )}
          <div
            className="text-body-lg leading-relaxed text-on-surface-variant space-y-6 [&>p]:mb-4"
            dangerouslySetInnerHTML={{ __html: post.body }}
          />
        </div>
      </div>
    </div>
  )
}

function outcomeIsWin(outcome: string) {
  return /vict|win|success|complete/i.test(outcome)
}

// --- Placeholder service-record data ---------------------------------------
// The API does not yet supply K/D, win rate, favourite loadout, or per-op K/D.
// These mocks establish the dossier layout; swap for real fields once served.
const MOCK_KD = '2.45'
const MOCK_WIN_RATE = '68%'

// Inline SVG data-URIs so thumbnails always render offline (no CDN dependency).
const FAV_WEAPON = {
  name: 'M4A1 Block II',
  img:
    "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='120' height='56'><rect width='120' height='56' fill='%23242a3a'/><rect x='12' y='25' width='86' height='6' rx='2' fill='%23adc6ff'/><rect x='80' y='22' width='11' height='18' rx='2' fill='%233a4252'/><rect x='30' y='31' width='10' height='12' rx='2' fill='%233a4252'/></svg>",
}
const FAV_ASSET = {
  name: 'M1A2 Abrams',
  img:
    "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='120' height='56'><rect width='120' height='56' fill='%23242a3a'/><rect x='22' y='28' width='76' height='14' rx='3' fill='%233a4252'/><rect x='44' y='20' width='30' height='10' rx='2' fill='%233a4252'/><rect x='70' y='30' width='34' height='4' rx='2' fill='%23adc6ff'/><circle cx='36' cy='44' r='5' fill='%23adc6ff'/><circle cx='84' cy='44' r='5' fill='%23adc6ff'/></svg>",
}
const BANNER_IMG =
  "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='200'><rect width='400' height='200' fill='%23151b2b'/><g stroke='%23adc6ff' stroke-width='0.5' opacity='0.5'><path d='M0 40 H400 M0 80 H400 M0 120 H400 M0 160 H400 M50 0 V200 M120 0 V200 M190 0 V200 M260 0 V200 M330 0 V200'/></g><circle cx='190' cy='100' r='26' fill='none' stroke='%23facc15' stroke-width='1.5'/><path d='M190 66 V134 M156 100 H224' stroke='%23facc15' stroke-width='1'/></svg>"

// Deterministic per-op K/D so the timeline shows varied (but stable) numbers.
function mockKd(seed: string): string {
  let h = 0
  for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) % 100000
  return (0.8 + (h % 250) / 100).toFixed(2) // 0.80 – 3.29
}

function TelemetryStat({
  label,
  value,
  className,
}: {
  label: string
  value: string
  className?: string
}) {
  return (
    <div>
      <span className="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
        {label}
      </span>
      <p className={cn('text-[5rem] font-bold leading-none tracking-tighter', className)}>{value}</p>
    </div>
  )
}

function FavLoadout({ label, item }: { label: string; item: { name: string; img: string } }) {
  return (
    <div>
      <span className="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
        {label}
      </span>
      <div className="flex items-center gap-3 rounded-lg border border-white/10 bg-surface-container/50 p-2">
        <img
          src={item.img}
          alt=""
          className="h-10 w-20 shrink-0 rounded border border-white/10 object-cover"
        />
        <span className="font-mono text-sm text-on-surface">{item.name}</span>
      </div>
    </div>
  )
}

export function DeploymentsPage() {
  const { data, isLoading, isError, error } = useDeployments()
  const user = useAuthStore((s) => s.user)
  const upcoming = data?.upcoming ?? []
  const history = data?.service_history ?? []
  const activeOrder = upcoming[0] ?? null

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="bg-topo-map bg-grid-overlay h-full w-full overflow-hidden">
          <div className="flex h-full w-full flex-col overflow-hidden bg-surface-glass backdrop-blur-xl lg:flex-row">
            {/* Left pane: telemetry dossier */}
            <aside className="custom-scrollbar flex shrink-0 flex-col gap-8 overflow-y-auto border-b border-white/10 bg-surface-container-lowest/40 p-8 lg:w-[30%] lg:border-b-0 lg:border-r">
              <header>
                <div className="mb-6 flex h-16 w-16 items-center justify-center text-primary">
                  <MaterialIcon name="military_tech" className="text-[4rem] leading-none" />
                </div>
                <h2 className="text-4xl font-black uppercase leading-none tracking-tighter text-on-surface">
                  {user?.username}
                </h2>
                <span className="mt-1 block font-mono text-sm uppercase tracking-widest text-primary">
                  {user?.role}
                </span>
              </header>

              <div className="space-y-6">
                <TelemetryStat label="K/D Ratio" value={MOCK_KD} className="text-primary" />
                <TelemetryStat label="Win Rate" value={MOCK_WIN_RATE} className="text-success" />
              </div>

              <div className="space-y-5 border-t border-white/10 pt-6">
                <div>
                  <span className="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                    Total Deployments
                  </span>
                  <p className="font-mono text-3xl font-bold text-on-surface">
                    {data?.total_operations ?? 0}
                  </p>
                </div>
                <FavLoadout label="Fav Weapon" item={FAV_WEAPON} />
                <FavLoadout label="Fav Asset" item={FAV_ASSET} />
              </div>
            </aside>

            {/* Right pane: active orders + combat history */}
            <main className="custom-scrollbar flex min-h-0 flex-1 flex-col overflow-y-auto bg-surface-container-highest/10">
              {/* Active Orders banner */}
              <section className="relative shrink-0 overflow-hidden border-b border-white/10">
                <img
                  src={BANNER_IMG}
                  alt=""
                  className="absolute inset-0 h-full w-full object-cover opacity-30 mix-blend-luminosity"
                />
                <div className="absolute inset-0 bg-gradient-to-r from-surface-container-lowest/80 to-transparent" />
                <div className="relative z-10 flex min-h-[240px] flex-col justify-center gap-3 p-8">
                  <span className="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                    Active Orders
                  </span>
                  {activeOrder ? (
                    <>
                      <h3 className="text-4xl font-black text-on-surface">{activeOrder.name}</h3>
                      <p className="font-mono text-sm text-on-surface-variant">
                        ASSIGNED:{' '}
                        <span className="text-on-surface">
                          {[activeOrder.faction, activeOrder.squad, activeOrder.role]
                            .filter(Boolean)
                            .join(' — ') || 'Unassigned'}
                        </span>
                      </p>
                      <p className="font-mono text-3xl text-tactical-yellow drop-shadow-md animate-pulse">
                        {countdownLabel(activeOrder.start_time)}
                      </p>
                      <Link
                        to={`/events/${activeOrder.event_id}`}
                        className="mt-2 inline-flex w-fit items-center gap-2 rounded-lg border border-white/10 bg-surface-glass px-4 py-2 text-sm font-medium text-on-surface backdrop-blur-md transition-colors hover:border-primary/40 hover:text-primary"
                      >
                        <MaterialIcon name="edit" className="text-base" />
                        MODIFY ASSIGNMENT
                      </Link>
                    </>
                  ) : (
                    <div className="flex flex-col items-center justify-center gap-3 py-4 text-center">
                      <MaterialIcon
                        name="track_changes"
                        className="text-7xl text-on-surface-variant/40 animate-pulse drop-shadow-[0_0_12px_rgba(173,198,255,0.25)]"
                      />
                      <h3 className="text-3xl font-black uppercase tracking-tight text-on-surface-variant/60">
                        No Active Orders
                      </h3>
                      <p className="font-mono text-sm text-on-surface-variant">
                        Stand by for deployment tasking.
                      </p>
                    </div>
                  )}
                </div>
              </section>

              {/* Combat History */}
              <section className="p-8">
                <h2 className="mb-4 font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                  Combat History
                </h2>
                {history.length === 0 ? (
                  <div className="bg-grid-overlay flex min-h-[200px] items-center justify-center rounded-xl border border-white/10 shadow-[inset_0_0_30px_rgba(173,198,255,0.06)]">
                    <p className="font-mono text-code-md uppercase tracking-widest text-on-surface-variant">
                      No Service History Compiled
                    </p>
                  </div>
                ) : (
                  <div className="flex flex-col">
                    {history.map((row, i) => {
                      const win = outcomeIsWin(row.outcome)
                      const last = i === history.length - 1
                      return (
                        <div
                          key={`${row.operation}-${i}`}
                          className="flex items-center gap-4 rounded-lg px-3 py-3 transition-colors hover:bg-white/[0.02]"
                        >
                          {/* Date */}
                          <span className="w-24 shrink-0 font-mono text-code-md text-on-surface-variant">
                            {formatShortDate(row.date)}
                          </span>
                          {/* Timeline node + identity */}
                          <div className="relative flex flex-1 items-center gap-4">
                            <div className="relative flex w-3 shrink-0 items-stretch justify-center self-stretch">
                              <span className="absolute top-1/2 h-3 w-3 -translate-y-1/2 rounded-full border-2 border-primary bg-surface shadow-[0_0_8px_rgba(173,198,255,0.5)]" />
                              {!last && (
                                <span className="absolute top-1/2 h-[calc(100%+1.5rem)] w-px bg-outline-variant/30" />
                              )}
                            </div>
                            <div className="min-w-0">
                              <h3 className="truncate text-label-md font-semibold text-on-surface">
                                {row.operation}
                              </h3>
                              <p className="font-mono text-label-sm text-outline">{row.role}</p>
                            </div>
                          </div>
                          {/* Stats */}
                          <span className="shrink-0 font-mono text-code-md text-on-surface-variant">
                            {mockKd(row.operation)} K/D
                          </span>
                          <span
                            className={cn(
                              'shrink-0 rounded-full px-2.5 py-0.5 font-mono text-[11px] font-semibold uppercase tracking-wide',
                              win
                                ? 'bg-success-muted/20 text-success'
                                : 'bg-error-alert/10 text-error-alert/80',
                            )}
                          >
                            {win ? 'Victory' : 'Defeat'}
                          </span>
                        </div>
                      )
                    })}
                  </div>
                )}
              </section>
            </main>
          </div>
        </div>
      </QueryState>
    </AuthGate>
  )
}

// --- Mock leaderboard data (used until the backend serves ranked rows) ------
const MOCK_LEADERBOARD: LeaderboardRow[] = [
  { rank: 1, discord_id: 'mock-1', username: 'Reaper', avatar_url: 'https://cdn.discordapp.com/embed/avatars/0.png', kills: 1842, deaths: 311, kd_ratio: 5.92, team_kills: 1, command_win_rate: 81, missions_played: 154, longest_kill_m: 912 },
  { rank: 2, discord_id: 'mock-2', username: 'Wraith', avatar_url: 'https://cdn.discordapp.com/embed/avatars/1.png', kills: 1610, deaths: 354, kd_ratio: 4.55, team_kills: 3, command_win_rate: 74, missions_played: 138, longest_kill_m: 1043 },
  { rank: 3, discord_id: 'mock-3', username: 'Havoc', avatar_url: 'https://cdn.discordapp.com/embed/avatars/2.png', kills: 1455, deaths: 402, kd_ratio: 3.62, team_kills: 0, command_win_rate: 69, missions_played: 171, longest_kill_m: 720 },
  { rank: 4, discord_id: 'mock-4', username: 'Cobra', avatar_url: 'https://cdn.discordapp.com/embed/avatars/3.png', kills: 1245, deaths: 388, kd_ratio: 3.21, team_kills: 5, command_win_rate: 66, missions_played: 129, longest_kill_m: 655 },
  { rank: 5, discord_id: 'mock-5', username: 'Specter', avatar_url: 'https://cdn.discordapp.com/embed/avatars/4.png', kills: 1130, deaths: 410, kd_ratio: 2.76, team_kills: 2, command_win_rate: 61, missions_played: 147, longest_kill_m: 588 },
  { rank: 6, discord_id: 'mock-6', username: 'Viper', avatar_url: 'https://cdn.discordapp.com/embed/avatars/0.png', kills: 998, deaths: 421, kd_ratio: 2.37, team_kills: 8, command_win_rate: 57, missions_played: 112, longest_kill_m: 503 },
  { rank: 7, discord_id: 'mock-7', username: 'Ghost', avatar_url: 'https://cdn.discordapp.com/embed/avatars/1.png', kills: 874, deaths: 399, kd_ratio: 2.19, team_kills: 4, command_win_rate: 54, missions_played: 133, longest_kill_m: 471 },
  { rank: 8, discord_id: 'mock-8', username: 'Bandit', avatar_url: 'https://cdn.discordapp.com/embed/avatars/2.png', kills: 765, deaths: 388, kd_ratio: 1.97, team_kills: 11, command_win_rate: 49, missions_played: 121, longest_kill_m: 402 },
]

// Per-category stat extraction: the value to rank/sort by, plus the formatted
// primary (headline) and secondary stat and an accent colour for the primary.
function categoryValue(r: LeaderboardRow, category: string): number {
  switch (category) {
    case 'command_win': return r.command_win_rate ?? 0
    case 'missions': return r.missions_played ?? 0
    case 'longest_kill': return r.longest_kill_m ?? 0
    case 'team_kills': return r.team_kills ?? 0
    default: return r.kd_ratio ?? 0
  }
}

function statFor(r: LeaderboardRow, category: string): {
  primary: string
  secondary: string
  accent: string
} {
  switch (category) {
    case 'command_win':
      return { primary: `${(r.command_win_rate ?? 0).toFixed(0)}%`, secondary: `${r.missions_played ?? 0} Ops`, accent: 'text-success' }
    case 'missions':
      return { primary: `${r.missions_played ?? 0}`, secondary: `${r.kills ?? 0} Kills`, accent: 'text-primary' }
    case 'longest_kill':
      return { primary: `${r.longest_kill_m ?? 0}m`, secondary: `${r.kills ?? 0} Kills`, accent: 'text-tactical-yellow' }
    case 'team_kills':
      return { primary: `${r.team_kills ?? 0}`, secondary: `${r.missions_played ?? 0} Ops`, accent: 'text-error-alert' }
    default:
      return { primary: (r.kd_ratio ?? 0).toFixed(2), secondary: `${r.kills ?? 0} Kills`, accent: 'text-success' }
  }
}

// Podium tier styling for ranks 1/2/3 (gold / silver / bronze).
const PODIUM_TIERS: Record<
  number,
  { avatar: string; ring: string; badge: string; score: string; order: string }
> = {
  1: { avatar: 'h-32 w-32', ring: 'border-tactical-yellow shadow-[0_0_50px_rgba(250,204,21,0.5)]', badge: 'bg-tactical-yellow text-black', score: 'text-4xl text-tactical-yellow', order: 'order-2' },
  2: { avatar: 'h-24 w-24', ring: 'border-slate-300 shadow-[0_0_35px_rgba(203,213,225,0.45)]', badge: 'bg-slate-300 text-black', score: 'text-2xl text-slate-200', order: 'order-1' },
  3: { avatar: 'h-20 w-20', ring: 'border-orange-400 shadow-[0_0_30px_rgba(251,146,60,0.45)]', badge: 'bg-orange-400 text-black', score: 'text-xl text-orange-300', order: 'order-3' },
}

function PodiumPlace({ row, category }: { row: LeaderboardRow; category: string }) {
  const tier = PODIUM_TIERS[row.rank] ?? PODIUM_TIERS[3]
  const stat = statFor(row, category)
  return (
    <div className={cn('flex flex-col items-center', tier.order)}>
      <div className="relative">
        <img
          src={row.avatar_url || DEFAULT_AVATAR}
          alt=""
          className={cn('rounded-xl border-2 object-cover', tier.avatar, tier.ring)}
        />
        <span
          className={cn(
            'absolute -bottom-3 left-1/2 -translate-x-1/2 rounded-full px-3 py-0.5 text-xs font-bold',
            tier.badge,
          )}
        >
          #{row.rank}
        </span>
      </div>
      <p className="mt-6 text-label-md font-semibold text-on-surface">{row.username}</p>
      <p className={cn('mt-1 font-bold drop-shadow-md', tier.score)}>{stat.primary}</p>
      <span className="text-label-sm text-on-surface-variant">{stat.secondary}</span>
      {row.rank === 1 && (
        <button
          type="button"
          className="mt-3 font-mono text-[11px] tracking-widest text-tactical-yellow/80 transition-colors hover:text-tactical-yellow"
        >
          [ VIEW DOSSIER ]
        </button>
      )}
    </div>
  )
}

export function LeaderboardsPage() {
  const [tab, setTab] = useState(0)
  const [search, setSearch] = useState('')
  const category = LEADERBOARD_TABS[tab]?.category ?? 'kd'
  const { data, isLoading, isError, error } = useLeaderboards(category, search || undefined)

  // Real data is backend-ranked; fall back to mock (re-sorted per category) so
  // the layout is populated until telemetry produces real boards.
  const realRows = data?.data ?? []
  const q = search.trim().toLowerCase()
  const rows = realRows.length
    ? realRows
    : MOCK_LEADERBOARD.filter((r) => !q || r.username.toLowerCase().includes(q))
        .slice()
        .sort((a, b) => categoryValue(b, category) - categoryValue(a, category))
        .map((r, i) => ({ ...r, rank: i + 1 }))

  const podium = rows.slice(0, 3)
  const rest = rows.slice(3)

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        <div className="bg-topo-map bg-grid-overlay h-full w-full overflow-hidden">
          <div className="custom-scrollbar flex h-full w-full flex-col overflow-y-auto bg-surface-glass p-6 backdrop-blur-xl md:p-10">
            <PageHeader
              title="Global Leaderboards"
              subtitle="Real-time tactical performance metrics across all active theaters."
            />

            {/* Segmented control + search */}
            <div className="flex flex-wrap items-center justify-between gap-4">
              <div className="flex w-max rounded-full border border-white/5 bg-black/20 p-1">
                {LEADERBOARD_TABS.map((t, i) => (
                  <button
                    key={t.category}
                    type="button"
                    onClick={() => setTab(i)}
                    className={cn(
                      'rounded-full px-6 py-1.5 text-label-md transition-colors',
                      i === tab
                        ? 'bg-white/10 text-white shadow-sm'
                        : 'text-on-surface-variant hover:bg-white/5 hover:text-on-surface',
                    )}
                  >
                    {t.label}
                  </button>
                ))}
              </div>
              <input
                type="search"
                placeholder="Search operators..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="w-full max-w-xs rounded-full border border-white/10 bg-black/20 px-4 py-2 text-sm text-on-surface placeholder:text-on-surface-variant"
              />
            </div>

            {rows.length === 0 ? (
              <p className="mt-8 text-on-surface-variant">No operators match your search.</p>
            ) : (
              <>
                {/* 3-tier podium */}
                {podium.length > 0 && (
                  <div className="flex flex-row items-end justify-center gap-8 pt-16 pb-12 md:gap-16">
                    {podium.map((p) => (
                      <PodiumPlace key={p.discord_id} row={p} category={category} />
                    ))}
                  </div>
                )}

                {/* Finder-style roster (rank 4+) */}
                {rest.length > 0 && (
                  <div className="mt-2 flex flex-col gap-0.5 border-t border-white/5 pt-4">
                    {rest.map((r) => {
                      const stat = statFor(r, category)
                      return (
                        <div
                          key={r.discord_id}
                          className="group flex cursor-pointer items-center gap-4 rounded-lg px-2 py-3 transition-colors hover:bg-white/5"
                        >
                          <span className="w-8 shrink-0 font-mono text-sm text-on-surface-variant">
                            {String(r.rank).padStart(2, '0')}
                          </span>
                          <img
                            src={r.avatar_url || DEFAULT_AVATAR}
                            alt=""
                            className="h-8 w-8 shrink-0 rounded-full object-cover"
                          />
                          <span className="flex-1 truncate text-label-md font-medium text-on-surface">
                            {r.username}
                          </span>
                          <span className="hidden text-sm text-on-surface-variant sm:inline">
                            {stat.secondary}
                          </span>
                          <span className={cn('w-16 text-right font-mono font-semibold', stat.accent)}>
                            {stat.primary}
                          </span>
                          <MaterialIcon
                            name="chevron_right"
                            className="text-on-surface-variant group-hover:text-white"
                          />
                        </div>
                      )
                    })}
                  </div>
                )}
              </>
            )}
          </div>
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
