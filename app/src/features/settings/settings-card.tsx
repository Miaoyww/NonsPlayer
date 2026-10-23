import type { ReactNode } from 'react'

type SettingsCardProps = {
  searchId?: string
  title: string
  description: string
  children: ReactNode
}

export function SettingsCard({ searchId, title, description, children }: SettingsCardProps) {
  return (
    <div
      data-setting-id={searchId}
      tabIndex={searchId ? -1 : undefined}
      className="settings-card scroll-my-6 flex flex-col justify-between gap-4 rounded-lg bg-muted/40 px-6 py-4 outline-none data-[search-highlighted]:ring-2 data-[search-highlighted]:ring-ring data-[search-highlighted]:ring-offset-2 data-[search-highlighted]:ring-offset-background focus-visible:ring-2 focus-visible:ring-ring lg:flex-row lg:items-center"
    >
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
