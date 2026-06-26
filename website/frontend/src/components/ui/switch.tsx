import { Switch as SwitchPrimitive } from '@base-ui/react/switch'
import { cn } from '@/lib/utils'

/**
 * Inline macOS-style toggle. Wraps Base UI Switch.
 * Use for boolean list-row state (published / pinned / signups_open).
 */
export function Switch({
  className,
  ...props
}: SwitchPrimitive.Root.Props) {
  return (
    <SwitchPrimitive.Root
      className={cn(
        'group relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-outline-variant/60 bg-surface-container-high p-0.5 outline-none transition-colors',
        'focus-visible:ring-2 focus-visible:ring-primary/50',
        'data-[checked]:border-primary data-[checked]:bg-primary',
        'data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50',
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        className={cn(
          'h-3.5 w-3.5 rounded-full bg-on-surface-variant shadow-sm transition-all',
          'data-[checked]:translate-x-4 data-[checked]:bg-on-primary',
        )}
      />
    </SwitchPrimitive.Root>
  )
}
