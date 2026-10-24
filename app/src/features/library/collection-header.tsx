import type { ReactNode } from 'react'
import { DetailHeader } from '@/features/library/detail-header'
import { DetailDescription } from '@/features/library/detail-description'
import { Play } from 'lucide-react'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { ArtistLinks } from '@/components/music/music-links'
import { CollectionKind } from './collection-kind'
import { CollectionSearch } from './collection-content'

interface Props {
  collection: MusicCollection
  description?: string | null
  total: number
  busy: boolean
  disabled: boolean
  cover?: ReactNode
  actions?: ReactNode
  kindLabel?: string
  credits?: ReactNode
  metadata?: ReactNode
  onPlay: () => void
}

export function CollectionHeader({
  collection,
  description,
  total,
  busy,
  disabled,
  onPlay,
  cover,
  actions,
  kindLabel,
  credits,
  metadata
}: Props) {
  const author =
    credits ??
    (collection.kind === 'album' && (collection.artists?.length || collection.subtitle) ? (
      <ArtistLinks artists={collection.artists} name={collection.subtitle} />
    ) : collection.kind === 'playlist' && collection.creatorId !== undefined ? (
      collection.subtitle.trim()
    ) : undefined)
  return (
    <DetailHeader className="library-collection-header">
      {cover ?? (
        <Cover
          size="large"
          cover={collection.cover}
          className={`library-detail-cover ${collection.kind === 'artist' ? 'rounded-full' : 'rounded-xl'}`}
        />
      )}
      <div className="library-detail-info">
        <CollectionKind collection={collection} label={kindLabel} />
        <h1 className="library-detail-title">{collection.name}</h1>
        <div className="library-detail-meta">
          {author && <span>by {author}</span>}
          <span>
            {total} 首{collection.kind === 'artist' ? '热门歌曲' : ''}
          </span>
          {metadata}
        </div>
        <DetailDescription description={description} />
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
      </div>
    </DetailHeader>
  )
}
