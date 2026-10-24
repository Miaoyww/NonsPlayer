import { CollectionContextMenu } from '@/features/library/collection-actions'
import { Play } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { formatReleaseDate } from '@/components/music/music-links'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export function AlbumCard({
  item,
  busy,
  sourceName,
  onOpen,
  onPlay
}: {
  item: MusicCollection
  busy: boolean
  sourceName?: string
  onOpen: (item: MusicCollection) => void
  onPlay: (item: MusicCollection) => void
}) {
  return (
    <CollectionContextMenu
      item={item}
      busy={busy}
      render={<article className="library-cover-card album-card" />}
    >
      <div className="library-cover-art">
        <button
          type="button"
          className="library-cover-open"
          onClick={() => onOpen(item)}
          aria-label={`打开专辑 ${item.name}`}
        >
          <Cover cover={item.cover} className="aspect-square w-full" />
        </button>
        <ActionButton
          size="icon-lg"
          className="library-cover-play"
          disabled={busy}
          aria-label={`播放专辑 ${item.name}`}
          onClick={() => onPlay(item)}
        >
          <Play aria-hidden="true" />
        </ActionButton>
      </div>
      <button
        type="button"
        className="library-cover-title"
        onClick={() => onOpen(item)}
        title={item.name}
      >
        {item.name}
      </button>
      <div className="library-cover-subtitle">
        {item.trackCount} 首 · {formatReleaseDate(item.publishedAt)}
      </div>
      {sourceName && <p className="library-cover-subtitle">{sourceName}</p>}
    </CollectionContextMenu>
  )
}
