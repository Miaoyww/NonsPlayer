import { CollectionContextMenu } from '@/features/library/collection-actions'
import { Play } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export function formatPlayCount(value: number) {
  if (value >= 100_000_000) return `${+(value / 100_000_000).toFixed(1)}亿`
  if (value >= 10_000) return `${+(value / 10_000).toFixed(1)}万`
  return Math.floor(value).toLocaleString('zh-CN')
}

export function PlaylistCard({
  item,
  busy,
  showPlayCount = true,
  sourceName,
  onOpen,
  onPlay
}: {
  item: MusicCollection
  busy: boolean
  showPlayCount?: boolean
  sourceName?: string
  onOpen: (item: MusicCollection) => void
  onPlay: (item: MusicCollection) => void
}) {
  return (
    <CollectionContextMenu
      item={item}
      busy={busy}
      render={<article className="library-cover-card" />}
    >
      <div className="library-cover-art" data-round={item.kind === 'artist'}>
        <button
          className="library-cover-open"
          onClick={() => onOpen(item)}
          aria-label={`打开 ${item.name}`}
        >
          <Cover cover={item.cover} className="aspect-square w-full" />
        </button>
        {showPlayCount && item.playCount != null && (
          <span className="playlist-play-count" aria-label={`${item.playCount} 次播放`}>
            <Play aria-hidden="true" />
            {formatPlayCount(item.playCount)}
          </span>
        )}
        <ActionButton
          size="icon-lg"
          className="library-cover-play"
          disabled={busy}
          aria-label={`播放 ${item.name}`}
          title="播放歌单（最多 1000 首）"
          onClick={() => onPlay(item)}
        >
          <Play aria-hidden="true" />
        </ActionButton>
      </div>
      <button className="library-cover-title" onClick={() => onOpen(item)} title={item.name}>
        {item.name}
      </button>
      {item.subtitle && (
        <p className="library-cover-subtitle" title={item.subtitle}>
          {item.kind === 'playlist' ? 'by ' : ''}
          {item.subtitle}
        </p>
      )}
      {sourceName && <p className="library-cover-subtitle">{sourceName}</p>}
    </CollectionContextMenu>
  )
}
