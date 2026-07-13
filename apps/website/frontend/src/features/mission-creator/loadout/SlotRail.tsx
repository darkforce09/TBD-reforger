// A3-style slot rail (T-068.10.8): the Arma 3 Virtual Arsenal vertical icon column — one
// button per pickable region, click to make it the active region (same handler as clicking
// the doll part). Icon-only; the tooltip/aria carry "Region — equipped item | empty"; a
// small dot marks equipped regions. Pure over props.

import type { LucideIcon } from 'lucide-react'
import {
  Backpack,
  Bomb,
  Crosshair,
  Focus,
  Footprints,
  Grip,
  Hand,
  HardHat,
  PersonStanding,
  RectangleVertical,
  Rocket,
  Shield,
  Shirt,
  Target,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import type { RegistryItem } from '@/types/models/registry'
import { RAIL_REGIONS } from './arsenalDollModel'
import type { LoadoutKey } from './arsenalRules'

const RAIL_ICONS: Record<LoadoutKey, LucideIcon> = {
  primary: Crosshair,
  optic: Focus,
  magazine: RectangleVertical,
  launcher: Rocket,
  handgun: Target,
  throwable: Bomb,
  headCover: HardHat,
  jacket: Shirt,
  vest: Grip,
  armoredVest: Shield,
  backpack: Backpack,
  handwear: Hand,
  pants: PersonStanding,
  boots: Footprints,
}

export function SlotRail({
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
  return (
    <div className="flex min-h-0 flex-col gap-1 overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/30 p-1">
      {RAIL_REGIONS.map(({ key, label }) => {
        const rn = picks[key]
        const itemName = rn ? (catalogByName.get(rn)?.display_name ?? rn) : null
        const active = activeKey === key
        const Icon = RAIL_ICONS[key]
        const tooltip = itemName ? `${label} — ${itemName}` : `${label} — empty`
        return (
          <button
            key={key}
            type="button"
            title={tooltip}
            aria-label={tooltip}
            aria-pressed={active}
            onClick={() => onSelect(key)}
            className={cn(
              'relative flex size-9 shrink-0 items-center justify-center rounded-md transition-colors',
              active
                ? 'bg-primary/20 text-primary'
                : itemName
                  ? 'text-on-surface hover:bg-white/5'
                  : 'text-outline hover:bg-white/5 hover:text-on-surface-variant',
            )}
          >
            <Icon className="size-4.5" />
            {itemName && (
              <span className="absolute bottom-1 right-1 size-1.5 rounded-full bg-primary/80" />
            )}
          </button>
        )
      })}
    </div>
  )
}
