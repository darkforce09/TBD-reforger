interface StatusPillProps {
  linked: boolean
  armaId?: string | null
}

export function StatusPill({ linked, armaId }: StatusPillProps) {
  if (!linked || !armaId) {
    return (
      <div className="rounded-full bg-surface-container-high px-3 py-1 text-xs text-on-surface-variant">
        Unlinked
      </div>
    )
  }
  return (
    <div className="rounded-full bg-success-muted px-3 py-1 font-mono text-xs text-success">
      Linked: {armaId.slice(0, 8)}...
    </div>
  )
}
