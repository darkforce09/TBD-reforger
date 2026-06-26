import type { ReactNode } from 'react'
import { Dialog as DialogPrimitive } from '@base-ui/react/dialog'
import { MaterialIcon } from '@/components/MaterialIcon'
import { cn } from '@/lib/utils'

/**
 * Frosted, centered macOS modal built on Base UI Dialog.
 * Replaces the stacked "form-over-list" admin pattern: keep the underlying
 * list visible behind a blurred backdrop, preserving context.
 *
 *   <Dialog open={open} onOpenChange={setOpen}>
 *     <DialogContent title="New Operation"> … </DialogContent>
 *   </Dialog>
 */
export function Dialog(props: DialogPrimitive.Root.Props) {
  return <DialogPrimitive.Root {...props} />
}
export function DialogTrigger(props: DialogPrimitive.Trigger.Props) {
  return <DialogPrimitive.Trigger {...props} />
}
export function DialogClose(props: DialogPrimitive.Close.Props) {
  return <DialogPrimitive.Close {...props} />
}

interface DialogContentProps {
  children: ReactNode
  /** Optional header title rendered with a close button. */
  title?: ReactNode
  description?: ReactNode
  className?: string
}

export function DialogContent({ children, title, description, className }: DialogContentProps) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Backdrop
        className={cn(
          'fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200',
          'data-[starting-style]:opacity-0 data-[ending-style]:opacity-0',
        )}
      />
      <DialogPrimitive.Popup
        className={cn(
          'glass fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none',
          'transition-all duration-200',
          'data-[starting-style]:scale-95 data-[starting-style]:opacity-0',
          'data-[ending-style]:scale-95 data-[ending-style]:opacity-0',
          className,
        )}
      >
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
      </DialogPrimitive.Popup>
    </DialogPrimitive.Portal>
  )
}
