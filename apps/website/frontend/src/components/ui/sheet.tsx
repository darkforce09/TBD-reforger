import type { ReactNode } from 'react'
import { Dialog as DialogPrimitive } from '@base-ui/react/dialog'
import { MaterialIcon } from '@/components/MaterialIcon'
import { cn } from '@/lib/utils'

/**
 * macOS slide-over panel built on Base UI Dialog. Slides in from the right
 * (default) or top. Used for detail dossiers / edit forms that should overlay
 * a list without replacing it.
 *
 *   <Sheet open={open} onOpenChange={setOpen}>
 *     <SheetContent title="Operative"> … </SheetContent>
 *   </Sheet>
 */
export function Sheet(props: DialogPrimitive.Root.Props) {
  return <DialogPrimitive.Root {...props} />
}
export function SheetTrigger(props: DialogPrimitive.Trigger.Props) {
  return <DialogPrimitive.Trigger {...props} />
}
export function SheetClose(props: DialogPrimitive.Close.Props) {
  return <DialogPrimitive.Close {...props} />
}
export function SheetTitle(props: DialogPrimitive.Title.Props) {
  return <DialogPrimitive.Title {...props} />
}
export function SheetDescription(props: DialogPrimitive.Description.Props) {
  return <DialogPrimitive.Description {...props} />
}

interface SheetContentProps {
  children: ReactNode
  title?: ReactNode
  description?: ReactNode
  side?: 'right' | 'top'
  className?: string
  /** Sticky footer (action bar) pinned to the bottom of the sheet. */
  footer?: ReactNode
  /**
   * Edge-to-edge body: drops the default padded scroll container and header/footer
   * chrome so children own the full layout (e.g. a cinematic hero + internal scroll
   * + overlaid action bar). Pair with `<SheetTitle>`/`<SheetClose>` inside children
   * for accessibility. Width is controlled via `className` (twMerge overrides the
   * default `max-w-md`).
   */
  bleed?: boolean
}

export function SheetContent({
  children,
  title,
  description,
  side = 'right',
  className,
  footer,
  bleed = false,
}: SheetContentProps) {
  const sideClasses =
    side === 'right'
      ? cn(
          'inset-y-0 right-0 h-full w-[92vw] max-w-md border-l',
          'data-[starting-style]:translate-x-full data-[ending-style]:translate-x-full',
        )
      : cn(
          'inset-x-0 top-0 max-h-[85vh] w-full border-b',
          'data-[starting-style]:-translate-y-full data-[ending-style]:-translate-y-full',
        )

  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Backdrop
        className={cn(
          'fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-300',
          'data-[starting-style]:opacity-0 data-[ending-style]:opacity-0',
        )}
      />
      <DialogPrimitive.Popup
        className={cn(
          'glass fixed z-50 flex flex-col border-outline-variant/30 shadow-2xl outline-none',
          'transition-transform duration-300 ease-out',
          sideClasses,
          className,
        )}
      >
        {bleed ? (
          children
        ) : (
          <>
        {title != null && (
          <div className="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
            <div className="min-w-0">
              <DialogPrimitive.Title className="text-headline-sm text-on-surface">
                {title}
              </DialogPrimitive.Title>
              {description != null && (
                <DialogPrimitive.Description className="mt-1 text-label-md text-on-surface-variant">
                  {description}
                </DialogPrimitive.Description>
              )}
            </div>
            <DialogPrimitive.Close
              className="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
              aria-label="Close"
            >
              <MaterialIcon name="close" />
            </DialogPrimitive.Close>
          </div>
        )}
        <div className="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">{children}</div>
        {footer != null && (
          <div className="border-t border-outline-variant/30 px-6 py-4">{footer}</div>
        )}
          </>
        )}
      </DialogPrimitive.Popup>
    </DialogPrimitive.Portal>
  )
}
