import type { ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

/**
 * Uppercase tactical status chip (PINNED, MODPACK UPDATE, READY, CLASS, …).
 * label-sm typography with semantic colour variants.
 */
const badgeVariants = cva(
  'inline-flex items-center gap-1 rounded border px-2 py-0.5 text-label-sm uppercase whitespace-nowrap',
  {
    variants: {
      variant: {
        primary: 'border-primary/30 bg-primary/10 text-primary',
        tertiary: 'border-tertiary/30 bg-tertiary/10 text-tertiary',
        warning: 'border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow',
        success: 'border-success/30 bg-success/15 text-success',
        error: 'border-error-alert/30 bg-error-alert/10 text-error-alert',
        neutral: 'border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant',
      },
    },
    defaultVariants: { variant: 'neutral' },
  },
)

interface BadgeProps extends VariantProps<typeof badgeVariants> {
  children: ReactNode
  className?: string
  icon?: string
}

export function Badge({ children, variant, className, icon }: BadgeProps) {
  return (
    <span className={cn(badgeVariants({ variant }), className)}>
      {icon && <span className="material-symbols-outlined text-[14px]">{icon}</span>}
      {children}
    </span>
  )
}
