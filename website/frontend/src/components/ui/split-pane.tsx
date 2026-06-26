import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

/**
 * Apple-Mail / Finder master-detail layout. The left list pane stays fixed
 * while the right detail pane swaps — navigation never replaces the whole view.
 * Runs full-height over the topo/grid background inside the app content area.
 *
 *   <SplitPane
 *     master={<MissionList … />}
 *     detail={selected ? <Dossier … /> : <EmptyDetail … />}
 *   />
 */
interface SplitPaneProps {
  master: ReactNode
  detail: ReactNode
  /** Width of the left list pane. */
  masterWidth?: string
  className?: string
  /** Header rendered above the master list (e.g. title + filter/create button). */
  masterHeader?: ReactNode
  /**
   * Drop the built-in opaque topo-map background so the pane can sit inside a
   * frosted-glass encasing that frosts a map painted behind it.
   */
  transparent?: boolean
}

export function SplitPane({
  master,
  detail,
  masterWidth = '22rem',
  className,
  masterHeader,
  transparent = false,
}: SplitPaneProps) {
  return (
    <div
      className={cn(
        'flex h-full min-h-0 w-full overflow-hidden',
        !transparent && 'bg-topo-map bg-grid-overlay',
        className,
      )}
    >
      <aside
        className="flex h-full min-h-0 shrink-0 flex-col border-r border-outline-variant/30 bg-surface-container-lowest/50"
        style={{ width: masterWidth, maxWidth: '90vw' }}
      >
        {masterHeader && (
          <div className="flex shrink-0 items-center justify-between gap-2 border-b border-outline-variant/30 px-4 py-3">
            {masterHeader}
          </div>
        )}
        <div className="custom-scrollbar flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto p-3">
          {master}
        </div>
      </aside>
      <main className="custom-scrollbar relative flex h-full min-h-0 flex-1 flex-col overflow-y-auto bg-surface-container-highest/10">
        {detail}
      </main>
    </div>
  )
}

/** Placeholder shown in the detail pane when nothing is selected. */
export function SplitPaneEmpty({
  icon,
  message,
}: {
  icon?: ReactNode
  message: string
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 text-outline">
      {icon}
      <p className="text-label-md">{message}</p>
    </div>
  )
}
