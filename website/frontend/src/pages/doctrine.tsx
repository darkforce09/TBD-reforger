import { useMemo, useState, type ReactNode } from 'react'
import { toast } from 'sonner'
import { MaterialIcon } from '@/components/MaterialIcon'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { useSolveFireMission } from '@/hooks/mutations'
import { formatBytes } from '@/lib/format'
import { cn } from '@/lib/utils'
import { Badge } from '@/components/ui/badge'
import { SplitPane, SplitPaneEmpty } from '@/components/ui/split-pane'
import { ListDetailItem } from '@/components/ui/list-detail-item'
import type { ModpackDTO, ModpackMod } from '@/types/api'

// ─── Shared doctrine master/detail shell ──────────────────────────────────
// Topo-map background → frosted glass encasing → transparent SplitPane, matching
// the Announcements blueprint. Reused by Modpacks, Wiki and the Vehicle Database.
function GlassSplit({
  masterHeader,
  master,
  detail,
  masterWidth,
}: {
  masterHeader: ReactNode
  master: ReactNode
  detail: ReactNode
  masterWidth?: string
}) {
  return (
    <div className="relative h-full w-full overflow-hidden">
      <div className="bg-topo-map bg-grid-overlay absolute inset-0 z-0" />
      <div className="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
        <SplitPane transparent masterWidth={masterWidth} masterHeader={masterHeader} master={master} detail={detail} />
      </div>
    </div>
  )
}

/** Search box used across the doctrine master panes. */
function SidebarSearch({
  value,
  onChange,
  placeholder,
}: {
  value: string
  onChange: (v: string) => void
  placeholder: string
}) {
  return (
    <div className="relative w-full">
      <MaterialIcon
        name="search"
        className="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-base text-on-surface-variant"
      />
      <input
        type="search"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full rounded-lg border border-white/10 bg-black/30 py-2 pr-3 pl-9 text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none"
      />
    </div>
  )
}

// ─── Server Modpacks ──────────────────────────────────────────────────────
// "macOS Tactical" split-pane: a searchable modpack list on the left, an
// App-Store-style dossier on the right with an admin Read/Edit toggle.
// Mock-driven for now — swap MOCK_MODPACKS for `useModpacks()` data and wire
// `onSave` to a mutation once the backend grows multi-pack editing.

const mockMod = (modpackId: string, sort: number, name: string, required = false): ModpackMod => ({
  id: `${modpackId}-${sort}`,
  modpack_id: modpackId,
  name,
  is_key_dependency: required,
  sort_order: sort,
})

const MOCK_MODPACKS: ModpackDTO[] = [
  {
    id: 'core-modern',
    name: 'Core Modern Expansion',
    version: '2.4.1',
    total_size_bytes: 18_897_856_102,
    workshop_url: 'https://reforger.armaplatform.com/workshop',
    is_current: true,
    created_at: '2026-05-02T00:00:00Z',
    mods: [
      mockMod('core-modern', 0, 'RHS: Status Quo', true),
      mockMod('core-modern', 1, 'TFAR — Task Force Radio', true),
      mockMod('core-modern', 2, 'ACE Reforged — Medical', true),
      mockMod('core-modern', 3, 'Enhanced Movement Plus'),
      mockMod('core-modern', 4, 'WCS — Weapon Customization Suite'),
      mockMod('core-modern', 5, 'Everon Topographic Maps'),
    ],
  },
  {
    id: 'desert-storm',
    name: 'Operation Desert Storm',
    version: '1.1.0',
    total_size_bytes: 12_348_030_976,
    workshop_url: 'https://reforger.armaplatform.com/workshop',
    is_current: false,
    created_at: '2026-03-18T00:00:00Z',
    mods: [
      mockMod('desert-storm', 0, 'RHS: Gulf War Arsenal', true),
      mockMod('desert-storm', 1, 'TFAR — Task Force Radio', true),
      mockMod('desert-storm', 2, 'Sand & Heat Environment Pack'),
      mockMod('desert-storm', 3, 'M1A1 Abrams Pack'),
      mockMod('desert-storm', 4, 'Coalition Uniforms 1991'),
    ],
  },
  {
    id: 'cold-war-80s',
    name: 'Cold War 1980s',
    version: '0.9.3',
    total_size_bytes: 9_663_676_416,
    is_current: false,
    created_at: '2026-01-09T00:00:00Z',
    mods: [
      mockMod('cold-war-80s', 0, 'RHS: GREF', true),
      mockMod('cold-war-80s', 1, 'TFAR — Task Force Radio', true),
      mockMod('cold-war-80s', 2, 'Spectrum Devices — Cold War Optics'),
      mockMod('cold-war-80s', 3, 'Arland Winter Retexture'),
    ],
  },
]

export function ModpacksPage() {
  const [packs, setPacks] = useState<ModpackDTO[]>(MOCK_MODPACKS)
  const [selectedId, setSelectedId] = useState(packs[0]?.id ?? '')
  const [query, setQuery] = useState('')
  const [mode, setMode] = useState<'read' | 'edit'>('read')
  // Mocked admin flag — replace with the real role check (`useAuthStore`) later.
  const isAdmin = true

  const selected = packs.find((p) => p.id === selectedId) ?? packs[0]

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return packs
    return packs.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.mods.some((m) => m.name.toLowerCase().includes(q)),
    )
  }, [packs, query])

  function handleSave(updated: ModpackDTO) {
    setPacks((prev) => prev.map((p) => (p.id === updated.id ? updated : p)))
    setMode('read')
    toast.success(`Saved "${updated.name}"`)
  }

  return (
    <AuthGate>
      <GlassSplit
        masterWidth="18rem"
        masterHeader={
          <div className="w-full space-y-3">
            <h1 className="text-headline-sm tracking-wide text-on-surface uppercase">Modpacks</h1>
            <SidebarSearch value={query} onChange={setQuery} placeholder="Search packs & mods…" />
          </div>
        }
        master={
          filtered.length === 0 ? (
            <p className="px-1 py-4 text-label-md text-on-surface-variant">
              No modpacks match “{query}”.
            </p>
          ) : (
            filtered.map((pack) => (
              <ListDetailItem
                key={pack.id}
                active={pack.id === selected?.id}
                onClick={() => {
                  setSelectedId(pack.id)
                  setMode('read')
                }}
                title={pack.name}
                trailing={pack.is_current ? <Badge variant="success">Active</Badge> : undefined}
                preview={
                  <span className="font-mono text-on-surface-variant">
                    v{pack.version} · {pack.mods.length} mods · {formatBytes(pack.total_size_bytes)}
                  </span>
                }
              />
            ))
          )
        }
        detail={
          selected ? (
            mode === 'edit' ? (
              <ModpackEditor
                key={selected.id}
                pack={selected}
                onCancel={() => setMode('read')}
                onSave={handleSave}
              />
            ) : (
              <ModpackDossier pack={selected} isAdmin={isAdmin} onEdit={() => setMode('edit')} />
            )
          ) : (
            <SplitPaneEmpty
              icon={<MaterialIcon name="inventory_2" className="text-4xl" />}
              message="Select a modpack."
            />
          )
        }
      />
    </AuthGate>
  )
}

/** Read-only "App Store" view of a modpack. */
function ModpackDossier({
  pack,
  isAdmin,
  onEdit,
}: {
  pack: ModpackDTO
  isAdmin: boolean
  onEdit: () => void
}) {
  return (
    <div className="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
      {/* Header */}
      <header className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-4xl font-bold tracking-tight text-on-surface">{pack.name}</h2>
          <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
            <span>v{pack.version}</span>
            <span>
              <span className="text-on-surface">{formatBytes(pack.total_size_bytes)}</span> total
            </span>
            <span>
              <span className="text-on-surface">{pack.mods.length}</span> mods included
            </span>
          </div>
        </div>
        {isAdmin && <ReadEditToggle mode="read" onChange={(m) => m === 'edit' && onEdit()} />}
      </header>

      {/* Mod list */}
      <ul className="mt-8">
        {pack.mods.map((mod) => (
          <li
            key={mod.id}
            className="flex items-center gap-4 rounded-xl border-b border-white/5 px-4 py-5 transition hover:bg-white/[0.02]"
          >
            <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-white/5 text-on-surface-variant">
              <MaterialIcon name="extension" />
            </div>
            <span className="flex-1 font-medium text-on-surface">{mod.name}</span>
            {mod.is_key_dependency && (
              <span className="rounded-md border border-tactical-yellow/20 bg-tactical-yellow/10 px-2.5 py-1 font-mono text-xs tracking-wider text-tactical-yellow">
                [ REQUIRED ]
              </span>
            )}
          </li>
        ))}
      </ul>

      {/* Action button */}
      <div className="mt-10 pt-2">
        <button
          type="button"
          onClick={() => toast.message('Launch requires the Reforger client')}
          className="w-full rounded-full bg-action py-5 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
        >
          [ Launch Game &amp; Auto-Download ]
        </button>
        {pack.workshop_url && (
          <a
            href={pack.workshop_url}
            target="_blank"
            rel="noreferrer"
            className="mt-4 block text-center text-sm text-on-surface-variant transition hover:text-on-surface"
          >
            View collection in Reforger Workshop ↗
          </a>
        )}
      </div>
    </div>
  )
}

/** Admin edit view: rename the pack and add/remove/flag mods. */
function ModpackEditor({
  pack,
  onCancel,
  onSave,
}: {
  pack: ModpackDTO
  onCancel: () => void
  onSave: (updated: ModpackDTO) => void
}) {
  const [name, setName] = useState(pack.name)
  const [mods, setMods] = useState<ModpackMod[]>(pack.mods)
  const [newMod, setNewMod] = useState('')

  function addMod() {
    const trimmed = newMod.trim()
    if (!trimmed) return
    setMods((prev) => [
      ...prev,
      {
        id: `${pack.id}-new-${Date.now()}`,
        modpack_id: pack.id,
        name: trimmed,
        is_key_dependency: false,
        sort_order: prev.length,
      },
    ])
    setNewMod('')
  }

  function removeMod(id: string) {
    setMods((prev) => prev.filter((m) => m.id !== id))
  }

  function toggleRequired(id: string) {
    setMods((prev) =>
      prev.map((m) => (m.id === id ? { ...m, is_key_dependency: !m.is_key_dependency } : m)),
    )
  }

  return (
    <div className="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
      <header className="flex items-start justify-between gap-4">
        <div className="flex-1">
          <label className="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
            Modpack name
          </label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full rounded-xl border border-white/10 bg-black/30 px-4 py-3 text-2xl font-bold tracking-tight text-on-surface focus:border-primary/50 focus:outline-none"
          />
        </div>
        <ReadEditToggle mode="edit" onChange={(m) => m === 'read' && onCancel()} />
      </header>

      {/* Editable mod list */}
      <ul className="mt-8">
        {mods.map((mod) => (
          <li
            key={mod.id}
            className="flex items-center gap-3 rounded-xl border-b border-white/5 px-4 py-4"
          >
            <MaterialIcon name="drag_indicator" className="text-on-surface-variant/50" />
            <span className="flex-1 font-medium text-on-surface">{mod.name}</span>
            <button
              type="button"
              onClick={() => toggleRequired(mod.id)}
              className={cn(
                'rounded-md border px-2.5 py-1 font-mono text-xs tracking-wider transition',
                mod.is_key_dependency
                  ? 'border-tactical-yellow/20 bg-tactical-yellow/10 text-tactical-yellow'
                  : 'border-white/10 text-on-surface-variant hover:bg-white/5',
              )}
            >
              [ REQUIRED ]
            </button>
            <button
              type="button"
              onClick={() => removeMod(mod.id)}
              aria-label={`Remove ${mod.name}`}
              className="flex size-8 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
            >
              <MaterialIcon name="close" />
            </button>
          </li>
        ))}
        {mods.length === 0 && (
          <li className="px-4 py-6 text-center text-sm text-on-surface-variant">
            No mods yet — add one below.
          </li>
        )}
      </ul>

      {/* Add-mod row */}
      <div className="mt-4 flex gap-2">
        <input
          value={newMod}
          onChange={(e) => setNewMod(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              addMod()
            }
          }}
          placeholder="Add a mod (e.g. ACE Reforged)…"
          className="flex-1 rounded-xl border border-white/10 bg-black/30 px-4 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none"
        />
        <button
          type="button"
          onClick={addMod}
          className="flex items-center gap-1.5 rounded-xl border border-white/10 px-4 text-sm font-medium text-on-surface transition hover:bg-white/5"
        >
          <MaterialIcon name="add" className="text-base" />
          Add
        </button>
      </div>

      {/* Save / cancel */}
      <div className="mt-10 flex gap-3 pt-2">
        <button
          type="button"
          onClick={() => onSave({ ...pack, name: name.trim() || pack.name, mods })}
          className="flex-1 rounded-full bg-action py-4 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
        >
          Save Changes
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="rounded-full border border-white/10 px-8 text-base font-medium text-on-surface-variant transition hover:bg-white/5 hover:text-on-surface"
        >
          Cancel
        </button>
      </div>
    </div>
  )
}

/** Segmented [ READ ] / [ EDIT ] pill toggle. */
function ReadEditToggle({
  mode,
  onChange,
}: {
  mode: 'read' | 'edit'
  onChange: (mode: 'read' | 'edit') => void
}) {
  return (
    <div className="flex shrink-0 items-center rounded-full border border-white/10 bg-black/30 p-1 font-mono text-xs">
      {(['read', 'edit'] as const).map((m) => (
        <button
          key={m}
          type="button"
          onClick={() => onChange(m)}
          className={cn(
            'rounded-full px-4 py-1.5 tracking-wider uppercase transition',
            mode === m
              ? 'bg-primary/20 text-primary shadow-[0_0_12px_rgba(173,198,255,0.25)]'
              : 'text-on-surface-variant hover:text-on-surface',
          )}
        >
          [ {m} ]
        </button>
      ))}
    </div>
  )
}

// ─── SOPs & Manuals (Wiki) ────────────────────────────────────────────────
// Mock doctrine content so the premium reading layout can be evaluated before
// the wiki CMS is wired up. Replace with `useWikiPages()` data when available.

/** Inline monospace token for frequencies, hotkeys, callsigns. */
function Mono({ children }: { children: ReactNode }) {
  return (
    <code className="rounded bg-black/40 px-1.5 py-0.5 font-mono text-[0.85em] text-primary">
      {children}
    </code>
  )
}

type CalloutVariant = 'critical' | 'warning' | 'info'

const CALLOUT_STYLES: Record<CalloutVariant, { box: string; label: string; title: string }> = {
  critical: { box: 'bg-error/10 border-error', label: 'text-error-alert', title: 'CRITICAL RULE' },
  warning: { box: 'bg-tactical-yellow/10 border-tactical-yellow', label: 'text-tactical-yellow', title: 'WARNING' },
  info: { box: 'bg-primary/10 border-primary', label: 'text-primary', title: 'NOTE' },
}

function Callout({
  variant,
  title,
  children,
}: {
  variant: CalloutVariant
  title?: string
  children: ReactNode
}) {
  const s = CALLOUT_STYLES[variant]
  return (
    <div className={cn('my-6 rounded-2xl border border-l-4 p-4 shadow-lg backdrop-blur-md', s.box)}>
      <p className={cn('mb-1 font-mono text-xs font-bold tracking-widest uppercase', s.label)}>
        {title ?? s.title}
      </p>
      <div className="text-body-md leading-relaxed text-on-surface-variant">{children}</div>
    </div>
  )
}

/** Section heading inside a manual document. */
function DocH2({ children }: { children: ReactNode }) {
  return (
    <h2 className="mt-10 mb-3 border-b border-white/10 pb-2 text-xl font-bold tracking-tight text-white">
      {children}
    </h2>
  )
}

interface Manual {
  id: string
  category: string
  title: string
  updated: string
  /** Raw Markdown — the single source of truth, editable in-app and AI-ready. */
  body: string
}

// GitHub-style alert tags → our Callout variants.
const CALLOUT_TAGS: Record<string, { variant: CalloutVariant; title?: string }> = {
  CRITICAL: { variant: 'critical' },
  CAUTION: { variant: 'critical' },
  WARNING: { variant: 'warning' },
  TIP: { variant: 'info', title: 'PRO-TIP' },
  NOTE: { variant: 'info' },
  INFO: { variant: 'info' },
}

/** Inline Markdown: **bold**, *italic*, `code`. */
function renderInline(text: string): ReactNode {
  const nodes: ReactNode[] = []
  const regex = /(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g
  let last = 0
  let key = 0
  let match: RegExpExecArray | null
  while ((match = regex.exec(text)) !== null) {
    if (match.index > last) nodes.push(text.slice(last, match.index))
    const tok = match[0]
    if (tok.startsWith('**')) {
      nodes.push(
        <strong key={key++} className="font-semibold text-on-surface">
          {tok.slice(2, -2)}
        </strong>,
      )
    } else if (tok.startsWith('`')) {
      nodes.push(<Mono key={key++}>{tok.slice(1, -1)}</Mono>)
    } else {
      nodes.push(<em key={key++}>{tok.slice(1, -1)}</em>)
    }
    last = regex.lastIndex
  }
  if (last < text.length) nodes.push(text.slice(last))
  return nodes
}

/**
 * Minimal Markdown → React renderer for the wiki reading pane. Supports the
 * subset we author: `##` headings, `-` bullet lists, GitHub `> [!TYPE]` alert
 * blocks (mapped to Callouts), and paragraphs with inline bold/italic/code.
 * Swap for `react-markdown` + `remark-gfm` if richer Markdown is ever needed —
 * the stored `body` string stays the same source of truth.
 */
function Markdown({ source }: { source: string }) {
  const lines = source.replace(/\r\n/g, '\n').split('\n')
  const blocks: ReactNode[] = []
  let i = 0
  let key = 0

  while (i < lines.length) {
    const line = lines[i]
    if (!line.trim()) {
      i++
      continue
    }

    if (line.startsWith('## ')) {
      blocks.push(<DocH2 key={key++}>{renderInline(line.slice(3))}</DocH2>)
      i++
      continue
    }
    if (line.startsWith('# ')) {
      blocks.push(
        <h1 key={key++} className="mb-4 text-2xl font-bold tracking-tight text-white">
          {renderInline(line.slice(2))}
        </h1>,
      )
      i++
      continue
    }

    // Callout block: one or more consecutive `>` lines.
    if (line.startsWith('>')) {
      const quoted: string[] = []
      while (i < lines.length && lines[i].startsWith('>')) {
        quoted.push(lines[i].replace(/^>\s?/, ''))
        i++
      }
      let variant: CalloutVariant = 'info'
      let title: string | undefined
      let bodyLines = quoted
      const tagMatch = quoted[0].match(/^\[!([A-Za-z-]+)\]\s*(.*)$/)
      if (tagMatch) {
        const mapped = CALLOUT_TAGS[tagMatch[1].toUpperCase()]
        if (mapped) {
          variant = mapped.variant
          title = tagMatch[2].trim() || mapped.title
        }
        bodyLines = quoted.slice(1)
      }
      blocks.push(
        <Callout key={key++} variant={variant} title={title}>
          {renderInline(bodyLines.join(' '))}
        </Callout>,
      )
      continue
    }

    // Bullet list.
    if (line.startsWith('- ') || line.startsWith('* ')) {
      const items: string[] = []
      while (i < lines.length && (lines[i].startsWith('- ') || lines[i].startsWith('* '))) {
        items.push(lines[i].slice(2))
        i++
      }
      blocks.push(
        <ul key={key++} className="mt-3 ml-1 space-y-2 text-body-md text-on-surface-variant">
          {items.map((it, idx) => (
            <li key={idx}>• {renderInline(it)}</li>
          ))}
        </ul>,
      )
      continue
    }

    // Paragraph: gather until a blank line or a new block starts.
    const para: string[] = []
    while (
      i < lines.length &&
      lines[i].trim() &&
      !lines[i].startsWith('#') &&
      !lines[i].startsWith('>') &&
      !lines[i].startsWith('- ') &&
      !lines[i].startsWith('* ')
    ) {
      para.push(lines[i])
      i++
    }
    blocks.push(
      <p key={key++} className="mt-3 text-body-md leading-relaxed text-on-surface-variant">
        {renderInline(para.join(' '))}
      </p>,
    )
  }

  return <>{blocks}</>
}

// Sidebar category order (independent of document declaration order).
const CATEGORY_ORDER = [
  'Leadership Fundamentals',
  'Timeline & Mission Planning',
  'Dynamic Communications Strategy',
  'Combat Formations & Maneuvers',
]

const MANUALS: Manual[] = [
  {
    id: 'plan-timeline',
    category: 'Timeline & Mission Planning',
    title: 'Timeline & Mission Planning',
    updated: '2026-06-18',
    body: `In a 1-life PvP environment, the plan you make in the staging area decides the fight before a single shot is fired. You do not get a second attempt. This guide uses the Swedish squad leader methodology — a simple, repeatable loop for turning a vague objective into a clear, time-bound plan your squad can actually execute under pressure.

The whole process answers three questions, in order: **What is the problem? How much time do I have? How do we adapt when it breaks?**

## Phase 1 — Define the Problem

Before you talk about movement, get brutally clear on what you are actually being asked to do and what stands in the way. If you cannot state the objective in one sentence, you are not ready to brief it.

- **The objective.** What does winning look like — hold a grid, destroy an asset, break the enemy's main effort? Name it.
- **Enemy composition.** How many, how equipped, armor or air? Where are they likely strong, and where are they thin?
- **Terrain.** What covers our approach, what channels us into a kill zone, and what high ground matters?
- **Our assets.** Squad size, weapon teams, vehicles, and — critically — how much daylight and time you have.

> [!CRITICAL]
> Plan against what the enemy *can* do, not what you *hope* they do. Build your plan around their most dangerous option, then exploit the gaps.

## Phase 2 — Define the Timeline

This is the part most squad leads skip, and it is the part that wins matches. Work **backwards** from the moment you expect contact and assign hard times to every step. A plan without a clock is just a wish.

- Set your decisive moment — call it \`H-Hour\` (the assault, the ambush trigger, the objective seizure).
- Back-plan from it: \`H-15\` in support-by-fire position, \`H-30\` at the last covered rally, \`H-45\` step off from staging.
- Reserve time for the things that always run long: crossing danger areas, re-org after contact, and getting everyone on the same map.
- Give the squad a **time hack** so every watch matches. "We move in 5" means nothing if nobody agrees on now.

> [!TIP]
> Budget roughly a third of your available time for planning and rehearsal, and two-thirds for movement and execution. If you spend the whole window talking, you will be rushing — and loud — when it counts.

## Phase 3 — Execution & Adaptability

No plan survives first contact. The point of Phases 1 and 2 is not a rigid script — it is to give your squad enough shared understanding that they keep fighting *your intent* when the plan falls apart and you can't talk to them.

- Brief **intent**, not just instructions: "I want us holding the north ridge by \`H+10\`, even if Alpha gets pinned." Tell them the *why*.
- Push decisions down. A fireteam that understands the goal will make a good call faster than they can raise you on a (possibly looted) radio.
- Name your triggers and branches in advance: "If we take fire from the treeline, Bravo suppresses, Alpha flanks left — no further orders needed."
- Run a 60-second rehearsal or backbrief. Have a team lead repeat the plan back; you will catch the gaps before the enemy does.

> [!WARNING]
> When the plan breaks, the worst choice is to freeze and wait for perfect information. Make a decision, communicate it in one line, and keep the squad moving. Momentum beats hesitation in a 1-life fight.`,
  },
  {
    id: 'lead-role',
    category: 'Leadership Fundamentals',
    title: 'The Squad Leader Mindset',
    updated: '2026-06-15',
    body: `Your job is not to be the best shooter — it is to make the rest of the squad more effective than they would be alone. You fight with your radio and your map first, your rifle second. A squad lead who is heads-down in a firefight is a squad lead who has stopped leading.

## What You Own

- **The plan** — and making sure everyone understands the intent behind it.
- **Tempo** — knowing when to push hard and when to slow down and reset.
- **Information** — building the picture and pushing the relevant parts down.

> [!TIP]
> Position yourself where you can *see and influence*, not where the action is hottest. Usually that is just behind your lead element, with eyes on the objective.

> [!CRITICAL]
> Calm is contagious, and so is panic. The squad takes its emotional temperature from you — if you keep your voice level under fire, they will too.`,
  },
  {
    id: 'lead-decisions',
    category: 'Leadership Fundamentals',
    title: 'Decision-Making Under Pressure',
    updated: '2026-06-10',
    body: `In a 1-life fight you will rarely have complete information, and waiting for it is itself a decision — usually a bad one. Train yourself to act on a good-enough read of the situation.

## A Fast Decision Loop

- **Read** — what just changed, and what is the biggest threat right now?
- **Decide** — pick the option that keeps initiative and protects the squad.
- **Act** — give one clear order and commit; correct on the move.

> [!WARNING]
> A decent decision made now beats a perfect decision made too late. Indecision gets people killed faster than a wrong call you correct quickly.`,
  },
  {
    id: 'comms-dynamic',
    category: 'Dynamic Communications Strategy',
    title: 'Operating With Looted Radios',
    updated: '2026-06-16',
    body: `We do not use fixed frequencies. The enemy can loot a radio off a body and listen to everything you say — so our comms plan assumes the net is compromised from the start. Frequencies are randomized each match and treated as throwaway.

## Assume You Are Being Heard

- Distribute the match frequency in the staging area, never over an open channel.
- If a member goes down in enemy territory, treat that frequency as **burned** and jump to your pre-agreed fallback.
- Reference locations by features or a private grid-shift, not raw map grids the enemy can also read.
- Keep transmissions short. Long, chatty traffic gives away your strength, intent, and rough position.

> [!CRITICAL]
> The moment a radio is lost behind enemy lines, every callsign and reference on that net is assumed compromised. Switch frequency and stop using any code words tied to it.

> [!TIP]
> Agree on a one-word **flash** signal before the op that means "the net is blown, jump to fallback now." One word, everyone moves, no debate on the radio.`,
  },
  {
    id: 'combat-formations',
    category: 'Combat Formations & Maneuvers',
    title: 'Fire & Movement',
    updated: '2026-06-05',
    body: `Everything in a gunfight comes down to one principle: one element fixes the enemy with fire while the other moves. If nobody is shooting, nobody should be moving in the open.

## Bounding

- Split into a **base of fire** and a **maneuver element** before you make contact, not during.
- Short bounds between hard cover — stay up only as long as your buddy can realistically cover you.
- The flank, not the frontal push, wins the position. Use fire to pin them in place while you get to their side.

> [!WARNING]
> Stay dispersed. In a 1-life fight, two operators caught in the same blast or burst is two permanent losses for the rest of the match.

> [!TIP]
> Read the terrain backwards from the objective: pick your support-by-fire position and your assault lane *before* you move, and the formation almost chooses itself.`,
  },
]

export function WikiPage() {
  // Mocked for now — wire to the real role (useAuthStore) when editing goes live.
  const isAdmin = true

  const [activeId, setActiveId] = useState<string>('plan-timeline')
  const [search, setSearch] = useState('')
  const [mode, setMode] = useState<'read' | 'edit'>('read')
  // In-session Markdown edits keyed by manual id (source of truth stays the string).
  const [edits, setEdits] = useState<Record<string, string>>({})

  const filtered = search
    ? MANUALS.filter((m) => `${m.title} ${m.category}`.toLowerCase().includes(search.toLowerCase()))
    : MANUALS

  // Group filtered manuals by category for the navigation index, in a fixed order.
  const byCategory = filtered.reduce<Record<string, Manual[]>>((acc, m) => {
    ;(acc[m.category] ??= []).push(m)
    return acc
  }, {})
  const orderedCategories = CATEGORY_ORDER.filter((c) => byCategory[c]?.length)

  const active = MANUALS.find((m) => m.id === activeId) ?? MANUALS[0]
  const source = edits[active.id] ?? active.body

  const selectDoc = (id: string) => {
    setActiveId(id)
    setMode('read')
  }

  return (
    <AuthGate>
      <GlassSplit
        masterWidth="17rem"
        masterHeader={
          <div className="w-full space-y-3">
            <p className="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
              SOPs &amp; Manuals
            </p>
            <SidebarSearch value={search} onChange={setSearch} placeholder="Search manuals..." />
          </div>
        }
        master={
          orderedCategories.length === 0 ? (
            <p className="px-1 py-4 text-label-md text-on-surface-variant">No manuals found.</p>
          ) : (
            orderedCategories.map((category) => (
              <div key={category} className="mb-3">
                <p className="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                  {category}
                </p>
                <div className="mt-1 flex flex-col gap-1">
                  {byCategory[category].map((m) => (
                    <ListDetailItem
                      key={m.id}
                      active={m.id === activeId}
                      onClick={() => selectDoc(m.id)}
                      title={m.title}
                    />
                  ))}
                </div>
              </div>
            ))
          )
        }
        detail={
          <section className="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
          <header className="flex shrink-0 items-start justify-between gap-4 border-b border-white/10 px-8 pt-8 pb-5 md:px-12">
            <div className="min-w-0">
              <div className="mb-3 flex items-center gap-2">
                <Badge variant="neutral" icon="schedule">
                  Last updated {active.updated}
                </Badge>
                <span className="font-mono text-xs tracking-widest text-outline uppercase">
                  {active.category}
                </span>
              </div>
              <h1 className="text-4xl font-bold tracking-tight text-white">{active.title}</h1>
            </div>
            {/* Read / Edit toggle — admin only; removed from the DOM otherwise. */}
            {isAdmin && (
              <div className="inline-flex shrink-0 gap-1 rounded-full border border-white/5 bg-black/20 p-1 font-mono text-xs">
                <button
                  type="button"
                  onClick={() => setMode('read')}
                  className={cn(
                    'rounded-full px-3 py-1 font-medium transition-all',
                    mode === 'read'
                      ? 'bg-surface-glass text-on-surface shadow-md'
                      : 'text-on-surface-variant hover:text-on-surface',
                  )}
                >
                  [ READ ]
                </button>
                <button
                  type="button"
                  onClick={() => setMode('edit')}
                  className={cn(
                    'rounded-full px-3 py-1 font-medium transition-all',
                    mode === 'edit'
                      ? 'bg-surface-glass text-on-surface shadow-md'
                      : 'text-on-surface-variant hover:text-on-surface',
                  )}
                >
                  [ EDIT ]
                </button>
              </div>
            )}
          </header>

          {isAdmin && mode === 'edit' ? (
            // Distraction-free Markdown editor — fills the entire pane.
            <textarea
              value={source}
              onChange={(e) => setEdits((prev) => ({ ...prev, [active.id]: e.target.value }))}
              spellCheck={false}
              className="h-full w-full flex-1 resize-none border-none bg-transparent p-8 font-mono text-sm leading-relaxed text-on-surface-variant outline-none focus:ring-0 md:p-12"
            />
          ) : (
            <article className="custom-scrollbar flex-1 overflow-y-auto p-8 md:p-12">
              <div className="max-w-3xl">
                <Markdown source={source} />
              </div>
            </article>
          )}
          </section>
        }
      />
    </AuthGate>
  )
}

// ─── Vehicle Database ─────────────────────────────────────────────────────
// AI-ready tactical intel: every field is a discrete, actionable signal an
// agent (or another page) can ingest or interlink by `id`. No encyclopedic
// fluff — only what a player needs to fight or avoid the asset.

type ThreatLevel = 'LOW' | 'MED' | 'HIGH'

interface VehicleIntel {
  id: string
  name: string
  faction: string // grouping bucket, e.g. "USSR Forces"
  class: string // e.g. APC, IFV, MBT
  threatLevel: ThreatLevel
  shortDescription: string
  criticalDirective: string
  telemetry: { mobility: string; defense: string; capacity: string }
  armament: string[]
  primaryThreats: string[]
  image?: string
}

const VEHICLES: VehicleIntel[] = [
  {
    id: 'btr-70',
    name: 'BTR-70',
    faction: 'USSR Forces',
    class: 'APC',
    threatLevel: 'MED',
    shortDescription:
      '8×8 wheeled amphibious APC. A fast road mover for shuttling infantry — thin armour means it is a battle taxi, not a fighting vehicle.',
    criticalDirective:
      'Do not let it bully you with the KPVT. The hull stops rifle rounds only — a single RPG or sustained .50 cal will brew it up with the whole squad inside.',
    telemetry: { mobility: '80 km/h · Amphibious', defense: 'Light · ~10mm steel', capacity: '2 crew + 8 pax' },
    armament: ['14.5mm KPVT HMG', '7.62mm PKT coax'],
    primaryThreats: ['Infantry AT', 'Heavy MG', 'Autocannon'],
    image:
      'https://lh3.googleusercontent.com/aida-public/AB6AXuAa6uQ_tsnbNhDf8cIp17ebpWDCdpJntK9g1ME75jrq_heGg9E-S3PYbbWNY2nunPGsJZDn-Zd7FEt3Jff2dDz_ZIqRZzxXlXp3OKqkQIoTmXkozbiwqK3iC_VLuc3hKtPKcznvLKREbs_XU_mNuUq7r7Wx9aX6GYMjJrlVza8sEz5zAAcKFdjbj5giyYLbY8jd3ZoBYl-IEL8aAWt9a9P6R7bs7wJyjK1DEuGhhu-z1dypXTXCul5dANMGZJAcAwNp4Hk4C_5-60c',
  },
  {
    id: 'bmp-2',
    name: 'BMP-2',
    faction: 'USSR Forces',
    class: 'IFV',
    threatLevel: 'HIGH',
    shortDescription:
      'Tracked IFV pairing a hard-hitting 30mm autocannon with an ATGM. Lethal to infantry and light vehicles, but its armour is still thin.',
    criticalDirective:
      'The 30mm is the real threat to your squad, not the hull. Break line of sight immediately — do not try to outrun it across open ground.',
    telemetry: { mobility: '65 km/h · Amphibious', defense: 'Light · spaced steel', capacity: '3 crew + 7 pax' },
    armament: ['30mm 2A42 autocannon', '9M113 Konkurs ATGM', '7.62mm PKT coax'],
    primaryThreats: ['Tank main gun', 'Tandem ATGM', 'Top-attack'],
  },
  {
    id: 'm1a1-abrams',
    name: 'M1A1 Abrams',
    faction: 'US Forces',
    class: 'MBT',
    threatLevel: 'HIGH',
    shortDescription:
      'Main battle tank. The frontal armour is near-impervious to most man-portable AT; the exploitable threat is its flanks, rear, and top.',
    criticalDirective:
      'Never engage frontally with light AT — you will only give away your position. Maneuver for a side or rear shot, or hit the top with tandem/top-attack munitions.',
    telemetry: { mobility: '67 km/h · 1500 hp', defense: 'Composite + DU armour', capacity: '4 crew' },
    armament: ['120mm M256 smoothbore', '12.7mm M2 cupola', '7.62mm M240 coax'],
    primaryThreats: ['Tandem ATGM', 'Top-attack', 'AT mines'],
  },
  {
    id: 'm2-bradley',
    name: 'M2 Bradley',
    faction: 'US Forces',
    class: 'IFV',
    threatLevel: 'HIGH',
    shortDescription:
      'Tracked IFV pairing a 25mm autocannon with TOW missiles. It will shred infantry up close and kill armour at range.',
    criticalDirective:
      'The TOW outranges your AT launchers. Close the distance through hard cover, or stay out of its line of sight entirely — do not trade in the open.',
    telemetry: { mobility: '66 km/h · 600 hp', defense: 'Aluminium + appliqué', capacity: '3 crew + 6 pax' },
    armament: ['25mm M242 Bushmaster', 'TOW ATGM launcher', '7.62mm M240 coax'],
    primaryThreats: ['Tank main gun', 'Tandem ATGM', 'Autocannon'],
  },
]

const VEHICLE_FACTION_ORDER = ['USSR Forces', 'US Forces']

function threatBadgeVariant(level: ThreatLevel): 'error' | 'warning' | 'success' {
  return level === 'HIGH' ? 'error' : level === 'MED' ? 'warning' : 'success'
}

export function VehicleDatabasePage() {
  const [q, setQ] = useState('')
  const [selectedId, setSelectedId] = useState('btr-70')

  const filtered = q
    ? VEHICLES.filter((v) => `${v.name} ${v.class} ${v.faction}`.toLowerCase().includes(q.toLowerCase()))
    : VEHICLES

  const byFaction = filtered.reduce<Record<string, VehicleIntel[]>>((acc, v) => {
    ;(acc[v.faction] ??= []).push(v)
    return acc
  }, {})
  const orderedFactions = VEHICLE_FACTION_ORDER.filter((f) => byFaction[f]?.length)

  const selected = filtered.find((v) => v.id === selectedId) ?? filtered[0] ?? null

  return (
    <AuthGate>
      <GlassSplit
        masterWidth="18rem"
        masterHeader={
          <div className="w-full space-y-3">
            <p className="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
              Vehicle Database
            </p>
            <SidebarSearch value={q} onChange={setQ} placeholder="Search assets..." />
          </div>
        }
        master={
          orderedFactions.length === 0 ? (
            <p className="px-1 py-4 text-label-md text-on-surface-variant">No assets found.</p>
          ) : (
            orderedFactions.map((faction) => (
              <div key={faction} className="mb-3">
                <p className="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                  {faction}
                </p>
                <div className="mt-1 flex flex-col gap-1">
                  {byFaction[faction].map((v) => (
                    <ListDetailItem
                      key={v.id}
                      active={v.id === selectedId}
                      onClick={() => setSelectedId(v.id)}
                      title={v.name}
                      preview={
                        <span className="font-mono uppercase text-outline">{v.class}</span>
                      }
                    />
                  ))}
                </div>
              </div>
            ))
          )
        }
        detail={
          selected ? (
            <VehicleDossier vehicle={selected} />
          ) : (
            <SplitPaneEmpty
              icon={<MaterialIcon name="directions_car" className="text-4xl" />}
              message="Select an asset to view its dossier."
            />
          )
        }
      />
    </AuthGate>
  )
}

function VehicleStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4">
      <p className="font-mono text-[11px] tracking-widest text-on-surface-variant uppercase">{label}</p>
      <p className="mt-1 font-mono text-base text-white">{value}</p>
    </div>
  )
}

function VehicleSectionTitle({ children }: { children: ReactNode }) {
  return (
    <h2 className="mb-3 font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
      {children}
    </h2>
  )
}

function VehicleDossier({ vehicle }: { vehicle: VehicleIntel }) {
  return (
    <div>
      {/* Cinematic hero banner */}
      <div className="relative h-72 w-full overflow-hidden">
        {vehicle.image ? (
          <img src={vehicle.image} alt="" className="h-full w-full object-cover" />
        ) : (
          <div className="flex h-full w-full items-center justify-center bg-surface-container-low">
            <MaterialIcon name="directions_car" className="text-7xl text-outline" />
          </div>
        )}
        <div className="absolute inset-0 bg-gradient-to-t from-surface-dim to-transparent" />
        <div className="absolute right-8 bottom-6 left-8">
          <div className="mb-3 flex flex-wrap items-center gap-2">
            <Badge variant="neutral">CLASS: {vehicle.class}</Badge>
            <Badge variant={threatBadgeVariant(vehicle.threatLevel)}>THREAT: {vehicle.threatLevel}</Badge>
            <Badge variant="primary">{vehicle.faction}</Badge>
          </div>
          <h1 className="text-4xl font-black tracking-tighter text-white uppercase">{vehicle.name}</h1>
          <p className="mt-2 max-w-2xl text-body-md text-on-surface-variant">{vehicle.shortDescription}</p>
        </div>
      </div>

      <div className="space-y-8 p-8 md:p-12">
        {/* Critical directive */}
        <div className="rounded-2xl border-l-4 border-tactical-yellow bg-tactical-yellow/10 p-4 shadow-lg backdrop-blur-md">
          <p className="mb-1 font-mono text-xs font-bold tracking-widest text-tactical-yellow uppercase">
            Critical Directive
          </p>
          <p className="text-body-md leading-relaxed text-on-surface-variant">{vehicle.criticalDirective}</p>
        </div>

        {/* Telemetry */}
        <div>
          <VehicleSectionTitle>Telemetry</VehicleSectionTitle>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <VehicleStat label="Mobility" value={vehicle.telemetry.mobility} />
            <VehicleStat label="Defense" value={vehicle.telemetry.defense} />
            <VehicleStat label="Capacity" value={vehicle.telemetry.capacity} />
          </div>
        </div>

        {/* Armament & Primary Threats */}
        <div className="grid grid-cols-1 gap-8 md:grid-cols-2">
          <div>
            <VehicleSectionTitle>Armament</VehicleSectionTitle>
            <ul className="space-y-2">
              {vehicle.armament.map((weapon) => (
                <li
                  key={weapon}
                  className="flex items-center gap-3 rounded-lg border border-white/10 bg-white/5 px-3 py-2.5 font-mono text-sm text-on-surface-variant"
                >
                  <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-primary" />
                  {weapon}
                </li>
              ))}
            </ul>
          </div>
          <div>
            <VehicleSectionTitle>Primary Threats</VehicleSectionTitle>
            <div className="flex flex-wrap gap-2">
              {vehicle.primaryThreats.map((threat) => (
                <span
                  key={threat}
                  className="rounded-full border border-error-alert/30 bg-error-alert/10 px-3 py-1 font-mono text-xs tracking-wide text-error-alert uppercase"
                >
                  {threat}
                </span>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export function MortarCalculatorPage() {
  const solve = useSolveFireMission()
  const [fpX, setFpX] = useState(1000)
  const [fpY, setFpY] = useState(2000)
  const [tgtX, setTgtX] = useState(2200)
  const [tgtY, setTgtY] = useState(1800)
  const solution = solve.data

  const handleSolve = () => {
    solve.mutate(
      { fp_x: fpX, fp_y: fpY, tgt_x: tgtX, tgt_y: tgtY, weapon_system: 'm252_81mm' },
      {
        onError: () => toast.error('Could not compute firing solution'),
      },
    )
  }

  return (
    <AuthGate>
      <div className="relative flex h-full w-full flex-col overflow-hidden">
        {/* Global topo-map background */}
        <div className="bg-topo-map bg-grid-overlay absolute inset-0 z-0" />
        <div className="relative z-10 flex h-full w-full flex-col gap-4 bg-surface-glass p-6 backdrop-blur-xl md:p-8">
        <PageHeader title="Mortar Calculator" subtitle="Enter grid coordinates for M252 81mm solution." />
        <OpsCard glass className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <label className="text-sm">
            FP X
            <input
              type="number"
              value={fpX}
              onChange={(e) => setFpX(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
          <label className="text-sm">
            FP Y
            <input
              type="number"
              value={fpY}
              onChange={(e) => setFpY(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
          <label className="text-sm">
            TGT X
            <input
              type="number"
              value={tgtX}
              onChange={(e) => setTgtX(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
          <label className="text-sm">
            TGT Y
            <input
              type="number"
              value={tgtY}
              onChange={(e) => setTgtY(Number(e.target.value))}
              className="mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm"
            />
          </label>
        </OpsCard>
        <button
          type="button"
          onClick={handleSolve}
          disabled={solve.isPending}
          className="self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
        >
          {solve.isPending ? 'Computing…' : 'Calculate Solution'}
        </button>
        <div className="relative min-h-0 flex-1 overflow-hidden rounded-xl border border-border-subtle bg-surface-container-lowest">
          <div
            className="absolute inset-0 opacity-30"
            style={{
              backgroundImage:
                'linear-gradient(rgba(59,130,246,0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(59,130,246,0.08) 1px, transparent 1px)',
              backgroundSize: '40px 40px',
            }}
          />
          <div
            className="absolute top-1/4 left-1/3 h-4 w-4 rounded-full border-2 border-success bg-success/30"
            title="Fire Position"
          />
          <div
            className="absolute top-1/2 left-2/3 h-4 w-4 rounded-full border-2 border-error bg-error/30"
            title="Target"
          />
          <OpsCard glass className="absolute right-4 bottom-4 w-72 border-t-2 border-tertiary">
            <h2 className="text-sm font-semibold text-primary">
              Firing Solution — {solution?.weapon_system ?? 'M252 81mm'}
            </h2>
            {solution ? (
              <dl className="mt-3 space-y-2 font-mono text-sm">
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">Distance</dt>
                  <dd>{Math.round(solution.distance_m).toLocaleString()} m</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">Azimuth</dt>
                  <dd>{solution.azimuth_deg.toFixed(1)}°</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">Elevation</dt>
                  <dd className="text-primary">{solution.elevation_mils} mils</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-on-surface-variant">TOF</dt>
                  <dd>{solution.time_of_flight_s.toFixed(1)} s</dd>
                </div>
              </dl>
            ) : (
              <p className="mt-3 text-xs text-on-surface-variant">
                Enter coordinates and calculate to see solution.
              </p>
            )}
          </OpsCard>
        </div>
        </div>
      </div>
    </AuthGate>
  )
}
