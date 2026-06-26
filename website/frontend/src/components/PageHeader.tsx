interface PageHeaderProps {
  title: string
  subtitle?: string
}

export function PageHeader({ title, subtitle }: PageHeaderProps) {
  return (
    <header className="mb-8">
      <h1 className="mb-2 text-3xl font-bold text-on-surface">{title}</h1>
      {subtitle && <p className="max-w-3xl text-on-surface-variant">{subtitle}</p>}
    </header>
  )
}
