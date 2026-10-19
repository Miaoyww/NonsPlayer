import type { ComponentProps, ReactNode } from 'react'
import { cn } from '@/lib/utils'

// One spacing surface for music pages; tune --music-page-* in styles/workspace.css.
export function MusicPage({ className, ...props }: ComponentProps<'section'>) {
  return <section className={cn('music-library music-page', className)} {...props} />
}

export function MusicPageHeader({ title, children }: { title: ReactNode; children?: ReactNode }) {
  return (
    <header className="music-page-heading">
      <h1 className="library-heading">{title}</h1>
      {children}
    </header>
  )
}
