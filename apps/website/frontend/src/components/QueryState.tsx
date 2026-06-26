import type { ReactNode } from 'react'

interface QueryStateProps {
  isLoading: boolean
  isError: boolean
  error?: Error | null
  isEmpty?: boolean
  emptyMessage?: string
  children: ReactNode
}

export function QueryState({
  isLoading,
  isError,
  error,
  isEmpty,
  emptyMessage = 'No data available.',
  children,
}: QueryStateProps) {
  if (isLoading) {
    return <p className="text-on-surface-variant">Loading…</p>
  }
  if (isError) {
    return (
      <p className="text-error">
        {error instanceof Error ? error.message : 'Failed to load data.'}
      </p>
    )
  }
  if (isEmpty) {
    return <p className="text-on-surface-variant">{emptyMessage}</p>
  }
  return <>{children}</>
}
