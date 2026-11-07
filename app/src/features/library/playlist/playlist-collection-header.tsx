import type { ReactNode } from 'react'
import { Play } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { CollectionHeader } from '@/features/library/collection/collection-header'
import { CollectionKind } from '@/features/library/collection/collection-kind'
import { DetailDescription } from '@/features/library/collection/detail-description'
import { DetailHeader } from '@/features/library/collection/detail-header'
import { CollectionSearch } from '@/features/library/collection/collection-content'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { useMediaQuery } from '@/lib/use-media-query'

interface Props {
  collection: MusicCollection
  description?: string | null
  total: number
  busy: boolean
  disabled: boolean
  cover?: ReactNode
  actions?: ReactNode
  onPlay: () => void
}

export function PlaylistCollectionHeader(props: Props) {
  const mobile = useMediaQuery('(max-width: 700px)')
  if (!mobile) return <CollectionHeader {...props} />
  const { collection, description, total, busy, disabled, cover, actions, onPlay } = props
  const author =
    collection.kind === 'playlist' && collection.creatorId !== undefined
      ? collection.subtitle.trim()
      : undefined
  return (
    <DetailHeader className="library-collection-header playlist-collection-header">
      <div className="playlist-detail-summary">
        {cover ?? (
          <Cover
            size="large"
            cover={collection.cover}
            className="library-detail-cover rounded-xl"
          />
        )}
        <div className="library-detail-info">
          <CollectionKind collection={collection} />
          <h1 className="library-detail-title">{collection.name}</h1>
          <div className="library-detail-meta">
            {author && <span>by {author}</span>}
            <span>{total} 首</span>
          </div>
          <DetailDescription description={description} />
        </div>
      </div>
      <div className="library-detail-actions flex flex-wrap items-center gap-2">
        <ActionButton
          variant="secondary"
          className="library-detail-play"
          disabled={disabled}
          onClick={onPlay}
        >
          <Play aria-hidden="true" />
          {busy ? '正在加载…' : '播放'}
        </ActionButton>
        {actions}
        <CollectionSearch />
      </div>
    </DetailHeader>
  )
}
