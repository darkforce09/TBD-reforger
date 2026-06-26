interface PageShellProps {
  title: string
  docPath: string
  stitchFolder?: string
}

export function PageShell({ title, docPath, stitchFolder }: PageShellProps) {
  return (
    <div className="mx-auto w-full max-w-[var(--spacing-container-max)]">
      <div className="rounded-xl border border-dashed border-border-subtle p-12 text-center text-on-surface-variant">
        <p className="text-xl font-semibold text-on-surface">{title}</p>
        <p className="mt-2 text-sm">Spec: frontend/docs/{docPath}</p>
        {stitchFolder && (
          <p className="mt-1 text-sm">Stitch: src/stitch-exports/{stitchFolder}/</p>
        )}
      </div>
    </div>
  )
}
