import type { ReactNode } from 'react'
import { AuthGate } from '@/components/AuthGate'
import { useAuthStore } from '@/store/useAuthStore'

interface AdminGateProps {
  children: ReactNode
}

export function AdminGate({ children }: AdminGateProps) {
  const hasAdmin = useAuthStore((s) => s.hasMinRole('admin'))

  return (
    <AuthGate>
      {hasAdmin ? (
        children
      ) : (
        <div className="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
          Admin access required.
        </div>
      )}
    </AuthGate>
  )
}
