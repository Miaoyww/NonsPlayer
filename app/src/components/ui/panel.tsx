import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export interface PanelPage {
  value: string
  content: ReactNode
  className?: string
  keepMounted?: boolean
}

export function Panel({
  value,
  pages,
  className
}: {
  value: string
  pages: PanelPage[]
  className?: string
}) {
  return (
    <div className={cn('panel', className)}>
      {pages.map((page) => {
        const active = page.value === value
        const mounted = active || page.keepMounted
        return (
          <div
            key={page.value}
            className={cn('panel-page', page.className)}
            data-active={active}
            aria-hidden={!active}
            inert={!active}
          >
            {mounted ? page.content : null}
          </div>
        )
      })}
    </div>
  )
}
