import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

interface OpsCardProps {
  children: ReactNode
  className?: string
  glow?: boolean
  /** Frosted Aegis glass surface (for panels over the topo/grid background). */
  glass?: boolean
}

/** Tactical panel card — matches Aegis surface-container / glass styling. */
export function OpsCard({ children, className, glow, glass }: OpsCardProps) {
  return (
    <div
      className={cn(
        'relative flex flex-col gap-3 overflow-hidden rounded-xl p-6',
        glass ? 'glass' : 'border border-border-subtle bg-surface-container',
        glow && 'shadow-[0_0_15px_rgba(173,198,255,0.15)]',
        className,
      )}
    >
      {children}
    </div>
  )
}
