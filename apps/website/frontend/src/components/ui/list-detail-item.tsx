import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

/**
 * Recurring master-list row for SplitPane left panes: leading meta line
 * (timestamp + status dot), title, optional 2-line preview, with an active
 * state (left vertical bar + inner glow) matching the announcements blueprint.
 */
interface ListDetailItemProps {
  active?: boolean
  onClick?: () => void
  /** Small leading meta text, e.g. a timestamp. Rendered mono. */
  meta?: ReactNode
  /** Status dot colour class (e.g. 'bg-primary', 'bg-tactical-yellow'). */
  dotClassName?: string
  pulse?: boolean
  title: ReactNode
  preview?: ReactNode
  /** Trailing slot (e.g. a Badge or chevron). */
  trailing?: ReactNode
  className?: string
}

export function ListDetailItem({
  active,
  onClick,
  meta,
  dotClassName,
  pulse,
  title,
  preview,
  trailing,
  className,
}: ListDetailItemProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'group relative w-full overflow-hidden rounded-lg border p-3 text-left transition-all duration-200',
        active
          ? 'border-primary/30 bg-surface-variant/80 shadow-[inset_0_0_15px_rgba(173,198,255,0.1)]'
          : 'border-transparent hover:border-outline-variant/30 hover:bg-surface-variant/40',
        className,
      )}
    >
      {active && <span className="absolute top-0 bottom-0 left-0 w-1 bg-primary" />}
      {(meta != null || dotClassName) && (
        <div className="mb-1 flex items-start justify-between gap-2">
          {meta != null && (
            <span
              className={cn(
                'font-mono text-code-md',
                active ? 'text-primary opacity-80' : 'text-outline',
              )}
            >
              {meta}
            </span>
          )}
          {dotClassName && (
            <span
              className={cn(
                'mt-1 h-2 w-2 shrink-0 rounded-full',
                dotClassName,
                pulse && 'animate-pulse',
              )}
            />
          )}
        </div>
      )}
      <div className="flex items-center justify-between gap-2">
        <h3
          className={cn(
            'truncate text-label-md font-semibold',
            active ? 'text-on-surface' : 'text-on-surface-variant group-hover:text-on-surface',
          )}
        >
          {title}
        </h3>
        {trailing}
      </div>
      {preview != null && (
        <p className="mt-1.5 line-clamp-2 text-label-sm text-outline normal-case">{preview}</p>
      )}
    </button>
  )
}
