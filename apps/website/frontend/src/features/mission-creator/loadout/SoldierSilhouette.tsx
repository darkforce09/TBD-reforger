// The Arsenal paper-doll (T-068.10.7, de-cluttered T-068.10.8): hand-drawn schematic SVG
// soldier where every loadout region is itself a clickable hotspot (the ACE model —
// including the optic and magazine ON the rifle). Pure presentational: picks + activeKey
// in, onSelect(key) out. The doll carries NO text — names live in the slot rail, the list
// header, the context column and the caption under the doll; hotspots keep <title>
// tooltips + aria labels. States: empty (dashed), equipped (primary-tinted fill), active
// (bright fill + thick stroke). Region completeness is asserted in arsenalDollModel.test.ts.

import type { KeyboardEvent, MouseEvent, ReactNode } from 'react'
import { cn } from '@/lib/utils'
import type { RegistryItem } from '@/types/models/registry'
import { DOLL_REGIONS } from './arsenalDollModel'
import type { LoadoutKey } from './arsenalRules'

const REGION_LABEL: Record<string, string> = Object.fromEntries(
  DOLL_REGIONS.map((r) => [r.key, r.label]),
)
REGION_LABEL.optic = 'Optic'
REGION_LABEL.magazine = 'Magazine'

function Hotspot({
  k,
  active,
  itemName,
  onSelect,
  children,
}: {
  k: LoadoutKey
  active: boolean
  /** Display name of the equipped item, or null when the region is empty. */
  itemName: string | null
  onSelect: (key: LoadoutKey) => void
  children: ReactNode
}) {
  const label = REGION_LABEL[k]
  const equipped = itemName !== null
  const onClick = (e: MouseEvent) => {
    e.stopPropagation() // sub-hotspots (optic/magazine) sit inside the rifle group
    onSelect(k)
  }
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== 'Enter' && e.key !== ' ') return
    e.preventDefault()
    e.stopPropagation()
    onSelect(k)
  }
  return (
    <g
      role="button"
      tabIndex={0}
      aria-label={equipped ? `${label}: ${itemName}` : `${label}: empty`}
      aria-pressed={active}
      onClick={onClick}
      onKeyDown={onKeyDown}
      className={cn(
        'cursor-pointer outline-none transition-colors',
        // Shape styling inherits from the group (children may override, e.g. rim fills).
        equipped
          ? active
            ? 'fill-primary/25 stroke-primary'
            : 'fill-primary/15 stroke-primary/60 hover:fill-primary/20'
          : active
            ? 'fill-primary/10 stroke-primary [stroke-dasharray:4_3]'
            : 'fill-surface-container-lowest/30 stroke-outline-variant/70 hover:stroke-outline [stroke-dasharray:4_3]',
        active ? 'stroke-[2.5]' : 'stroke-1',
      )}
    >
      <title>{equipped ? `${label}: ${itemName}` : `${label} — empty`}</title>
      {children}
    </g>
  )
}

export function SoldierSilhouette({
  picks,
  activeKey,
  onSelect,
  catalogByName,
}: {
  picks: Record<LoadoutKey, string>
  activeKey: LoadoutKey
  onSelect: (key: LoadoutKey) => void
  catalogByName: ReadonlyMap<string, RegistryItem>
}) {
  const name = (k: LoadoutKey): string | null => {
    const rn = picks[k]
    if (!rn) return null
    return catalogByName.get(rn)?.display_name ?? rn
  }
  const spot = (k: LoadoutKey) => ({
    k,
    active: activeKey === k,
    itemName: name(k),
    onSelect,
  })

  return (
    <svg
      viewBox="0 0 360 640"
      role="group"
      aria-label="Soldier loadout"
      className="h-full w-full"
    >
      {/* ── behind the body ─────────────────────────────────────────────── */}
      {/* Backpack: pokes out on the viewer-left side, behind the arm. */}
      <Hotspot {...spot('backpack')}>
        <rect x={84} y={165} width={44} height={120} rx={12} />
        <line x1={84} y1={205} x2={128} y2={205} />
      </Hotspot>
      {/* Launcher: tube slung over the viewer-right shoulder. */}
      <Hotspot {...spot('launcher')}>
        <g transform="rotate(28 255 132)">
          <rect x={246} y={72} width={18} height={120} rx={8} />
          <rect x={243} y={72} width={24} height={10} rx={4} />
        </g>
      </Hotspot>

      {/* ── body decor (not clickable) ──────────────────────────────────── */}
      <g className="fill-surface-container-lowest/60 stroke-outline-variant/40">
        <circle cx={180} cy={92} r={26} />
        <rect x={170} y={114} width={20} height={18} />
      </g>

      {/* ── wear ────────────────────────────────────────────────────────── */}
      {/* Jacket: torso + both arms. */}
      <Hotspot {...spot('jacket')}>
        <rect x={140} y={132} width={80} height={150} rx={10} />
        <rect x={108} y={140} width={26} height={140} rx={12} />
        <rect x={226} y={140} width={26} height={140} rx={12} />
      </Hotspot>
      {/* Pants: hip block + two legs. */}
      <Hotspot {...spot('pants')}>
        <rect x={146} y={282} width={68} height={28} rx={6} />
        <rect x={146} y={302} width={30} height={178} rx={8} />
        <rect x={184} y={302} width={30} height={178} rx={8} />
      </Hotspot>
      {/* Boots: toes point outward. */}
      <Hotspot {...spot('boots')}>
        <path d="M 178 484 L 178 514 L 134 514 Q 130 514 130 508 L 130 500 L 146 484 Z" />
        <path d="M 182 484 L 182 514 L 226 514 Q 230 514 230 508 L 230 500 L 214 484 Z" />
      </Hotspot>
      {/* Gloves: both hands, one hotspot. */}
      <Hotspot {...spot('handwear')}>
        <circle cx={121} cy={296} r={13} />
        <circle cx={239} cy={296} r={13} />
      </Hotspot>
      {/* Vest (chest rig): pouch panel on the chest. */}
      <Hotspot {...spot('vest')}>
        <rect x={150} y={150} width={60} height={64} rx={6} />
        <rect x={155} y={188} width={14} height={20} rx={2} />
        <rect x={173} y={188} width={14} height={20} rx={2} />
        <rect x={191} y={188} width={14} height={20} rx={2} />
      </Hotspot>
      {/* Armored vest: plate-carrier rim around the chest rig + collar — a second,
          simultaneous torso layer (both vest slots render at once). */}
      <Hotspot {...spot('armoredVest')}>
        <rect x={142} y={142} width={76} height={110} rx={10} className="fill-none" />
        <rect x={160} y={126} width={40} height={10} rx={4} />
      </Hotspot>
      {/* Helmet: dome over the head. */}
      <Hotspot {...spot('headCover')}>
        <path d="M 146 90 A 34 34 0 0 1 214 90 L 214 98 L 146 98 Z" />
      </Hotspot>

      {/* ── belt kit ────────────────────────────────────────────────────── */}
      {/* Throwable: grenade pouch on the left of the belt. */}
      <Hotspot {...spot('throwable')}>
        <rect x={112} y={326} width={26} height={30} rx={5} />
        <rect x={112} y={326} width={26} height={10} rx={4} />
      </Hotspot>
      {/* Handgun: holster on the right hip. */}
      <Hotspot {...spot('handgun')}>
        <path d="M 222 312 L 248 312 L 248 336 L 236 352 L 226 352 Q 222 352 222 346 Z" />
        <line x1={222} y1={320} x2={248} y2={320} />
      </Hotspot>

      {/* ── the rifle (front-most), held diagonally across the thighs, own sub-hotspots ── */}
      <g transform="translate(19 76) rotate(-32 176 329)">
        <Hotspot {...spot('primary')}>
          {/* stock, receiver + handguard, barrel */}
          <path d="M 96 322 L 74 330 Q 70 332 70 337 L 70 348 L 96 340 Z" />
          <rect x={96} y={322} width={140} height={14} rx={3} />
          <rect x={236} y={325} width={46} height={6} rx={2} />
          {/* grip */}
          <path d="M 128 336 L 140 336 L 136 354 L 128 354 Z" />
        </Hotspot>
        {/* Optic: the bump on top of the receiver (the ACE screenshot circle). */}
        <Hotspot {...spot('optic')}>
          <rect x={150} y={306} width={26} height={12} rx={2} />
          <rect x={156} y={318} width={14} height={4} />
        </Hotspot>
        {/* Magazine: below the receiver. */}
        <Hotspot {...spot('magazine')}>
          <path d="M 160 336 L 178 336 L 182 364 Q 182 368 178 368 L 166 368 Q 163 368 162 364 Z" />
        </Hotspot>
      </g>
    </svg>
  )
}
