import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

/**
 * Frosted glass surface — the Aegis primary container. Use for floating panels
 * and cards that sit over the topo/grid background.
 */
export function GlassPanel({
  children,
  className,
}: {
  children: ReactNode
  className?: string
}) {
  return <div className={cn('glass rounded-xl shadow-xl', className)}>{children}</div>
}

/**
 * Floating bottom HUD action bar — detached from the screen edges with a
 * glowing tertiary top border, as seen across the blueprints' detail views.
 */
export function HudBar({
  children,
  className,
}: {
  children: ReactNode
  className?: string
}) {
  return (
    <div
      className={cn(
        'absolute bottom-6 left-1/2 z-20 flex -translate-x-1/2 items-center gap-4',
        'rounded-xl border-t-2 border-tertiary bg-surface-container-high/90 px-6 py-3 shadow-2xl backdrop-blur-lg',
        className,
      )}
    >
      {children}
    </div>
  )
}
