// ACE-style item detail pane (T-068.10.6): silhouette placeholder (real renders later),
// identity + phys attrs + container capacity bars + variant relations. Pure render over
// the itemDetail selector; `onInspect` hops the pane between variant relatives.

import { Box, Crosshair } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import type { ItemDetail } from './itemDetail'

function fmt(n: number | null, unit: string): string {
  return n === null ? '—' : `${n.toLocaleString()} ${unit}`
}

export function ItemDetailPane({
  detail,
  onInspect,
}: {
  detail: ItemDetail | null
  onInspect: (resourceName: string) => void
}) {
  if (!detail) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 rounded-lg border border-outline-variant/20 bg-surface-container-lowest/30 p-4 text-center">
        <Crosshair className="size-6 text-outline" />
        <p className="text-label-sm normal-case text-outline">
          Pick any item — its details show here.
        </p>
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col gap-3 overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/30 p-3">
      {/* Silhouette placeholder — asset renders land later; the box keeps the ACE layout. */}
      <div className="flex h-28 items-center justify-center rounded-md border border-dashed border-outline-variant/30 bg-surface-container-lowest/40">
        <Box className="size-8 text-outline/60" />
      </div>

      <div>
        <h3 className="text-title-md leading-tight text-on-surface">{detail.name}</h3>
        <div className="mt-1 flex flex-wrap items-center gap-1.5">
          <Badge variant="primary">{detail.kind}</Badge>
          {detail.addon && <Badge variant="neutral">{detail.addon}</Badge>}
          {detail.abstract && <Badge variant="warning">template</Badge>}
        </div>
      </div>

      <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-label-md">
        <dt className="text-outline">Weight</dt>
        <dd className="text-right font-mono tabular-nums text-on-surface">
          {fmt(detail.weightKg, 'kg')}
        </dd>
        <dt className="text-outline">Volume</dt>
        <dd className="text-right font-mono tabular-nums text-on-surface">
          {fmt(detail.volumeCm3, 'cm³')}
        </dd>
      </dl>

      {detail.variantOf !== null &&
        (() => {
          const parent = detail.variantOf
          return (
            <p className="text-label-sm normal-case text-outline">
              Configuration of{' '}
              <button
                type="button"
                onClick={() => onInspect(parent.resourceName)}
                className="text-primary hover:underline"
              >
                {parent.name}
              </button>
            </p>
          )
        })()}

      {detail.configurations.length > 0 && (
        <div className="flex flex-col gap-1">
          <p className="text-label-sm text-outline">
            {detail.configurations.length} factory configurations
          </p>
          <div className="flex flex-col items-start gap-0.5">
            {detail.configurations.map((c) => (
              <button
                key={c.resourceName}
                type="button"
                onClick={() => onInspect(c.resourceName)}
                className="text-left text-label-sm normal-case text-on-surface-variant hover:text-primary hover:underline"
              >
                {c.name}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

/** Right-hand container panel: capacity summary + the reserved cargo area. */
export function ContainerPanel({ detail }: { detail: ItemDetail }) {
  return (
    <div className="flex h-full flex-col gap-3 overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/30 p-3">
      <h4 className="text-title-sm text-on-surface">Container</h4>
      <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-label-md">
        <dt className="text-outline">Capacity</dt>
        <dd className="text-right font-mono tabular-nums text-on-surface">
          {fmt(detail.maxVolumeCm3, 'cm³')}
        </dd>
        <dt className="text-outline">Max load</dt>
        <dd className="text-right font-mono tabular-nums text-on-surface">
          {fmt(detail.maxWeightKg, 'kg')}
        </dd>
      </dl>
      <div className="flex flex-1 flex-col items-center justify-center gap-1 rounded-md border border-dashed border-outline-variant/30 p-3 text-center">
        <p className="text-label-sm normal-case text-outline">
          Cargo editor (magazines, meds, throwable spares against the volume/weight budget)
          lands with the cargo slice.
        </p>
      </div>
    </div>
  )
}
