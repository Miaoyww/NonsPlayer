import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

export function NowPlayingPanelCard({
  label,
  className,
  cardClassName,
  children
}: {
  label: string
  className?: string
  cardClassName?: string
  children: ReactNode
}) {
  return (
    <section className={cn('now-playing-information', className)} aria-label={label}>
      <article className={cn('now-playing-information-card', cardClassName)}>{children}</article>
    </section>
  )
}
