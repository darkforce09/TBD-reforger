import { cn } from '@/lib/utils'

interface MaterialIconProps {
  name: string
  className?: string
  /** Render the filled (FILL 1) glyph variant. Defaults to outlined (FILL 0). */
  filled?: boolean
}

export function MaterialIcon({ name, className, filled }: MaterialIconProps) {
  return (
    <span
      className={cn('material-symbols-outlined', className)}
      style={filled ? { fontVariationSettings: "'FILL' 1" } : undefined}
    >
      {name}
    </span>
  )
}
