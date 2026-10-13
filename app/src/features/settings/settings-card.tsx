import type { ReactNode } from 'react'

type SettingsCardProps = {
  title: string
  description: string
  children: ReactNode
}

export function SettingsCard({ title, description, children }: SettingsCardProps) {
  return (
    <div className="settings-card flex flex-col justify-between gap-4 rounded-lg bg-muted/40 px-6 py-4 lg:flex-row lg:items-center">
      <div className="min-w-0 flex-1">
        <h3 className="text-sm font-semibold">{title}</h3>
        <p className="settings-card-description mt-1 text-xs leading-relaxed text-muted-foreground [overflow-wrap:anywhere]">
          {description}
        </p>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}
