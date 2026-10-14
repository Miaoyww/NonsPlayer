import { DetailHeader } from '@/features/library/detail-header'
import { DetailDescription } from '@/features/library/detail-description'
import { Play } from 'lucide-react'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'

interface Props {
  collection: MusicCollection
  description?: string | null
  total: number
  busy: boolean
  disabled: boolean
  onPlay: () => void
}

export function CollectionHeader({
  collection,
  description,
  total,
  busy,
  disabled,
  onPlay
}: Props) {
  const kind =
    collection.kind === 'playlist' ? '歌单' : collection.kind === 'album' ? '专辑' : '艺人'
  return (
    <DetailHeader>
      <Cover
        size="large"
        cover={collection.cover}
        className={`library-detail-cover ${collection.kind === 'artist' ? 'rounded-full' : 'rounded-xl'}`}
      />
      <div className="library-detail-info">
        <h1 className="library-detail-title">{collection.name}</h1>
        <div className="library-detail-meta">
          <p className="font-semibold">
            {kind}
            {collection.subtitle && ` · ${collection.subtitle}`}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            {collection.kind === 'artist' ? '热门歌曲' : '音乐收藏'}
            {total > 0 && ` · ${total} 首音乐`}
          </p>
        </div>
        <DetailDescription description={description} />
        <div className="library-detail-actions">
          <ActionButton
            variant="secondary"
            className="library-detail-play"
            disabled={disabled}
            onClick={onPlay}
          >
            <Play aria-hidden="true" />
            {busy ? '正在加载…' : '播放'}
          </ActionButton>
        </div>
      </div>
    </DetailHeader>
  )
}
