import { Cover } from '@/components/music/cover'
import type { ReactNode } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'

export function ArtistCard({
  name,
  cover,
  meta,
  onOpen,
  menu
}: {
  name: string
  cover?: import('@/features/music/types').CoverInput
  meta?: string
  onOpen: () => void
  menu?: ReactNode
}) {
  const body = (
    <>
      <div className="library-cover-art" data-round="true">
        <button
          type="button"
          className="library-cover-open"
          onClick={onOpen}
          aria-label={`打开 ${name}`}
        >
          <Cover cover={cover} className="aspect-square w-full" />
        </button>
      </div>
      <button type="button" className="library-cover-title" onClick={onOpen} title={name}>
        {name}
      </button>
      {meta && (
        <p className="library-cover-subtitle" title={meta}>
          {meta}
        </p>
      )}
    </>
  )
  if (!menu) return <article className="library-artist-card">{body}</article>
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger render={<article className="library-artist-card" />}>
        {body}
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Positioner className="z-[70]" sideOffset={4}>
          <ContextMenu.Popup className="song-context-menu" aria-label={`${name} 的菜单`}>
            {menu}
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  )
}
