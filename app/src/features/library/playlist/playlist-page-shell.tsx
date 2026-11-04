import type { ReactNode } from 'react'
import { MusicPage } from '@/components/music/music-page'
import { CollectionContent } from '../collection/collection-content'

export function PlaylistPageShell({
  header,
  children,
  hasMore = false,
  searchable = true
}: {
  header: ReactNode
  children: ReactNode
  hasMore?: boolean
  searchable?: boolean
}) {
  return (
    <MusicPage aria-label="歌单详情">
      <CollectionContent hasMore={hasMore} searchable={searchable}>
        {header}
        {children}
      </CollectionContent>
    </MusicPage>
  )
}
