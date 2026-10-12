import { Skeleton } from '@/components/ui/skeleton'
import { CollectionContextMenu } from '@/features/library/collection-actions'
import { Play } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { formatReleaseDate } from '@/components/music/music-links'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { useEffect, useRef, useState } from 'react'
import { useEntityDetail } from '@/features/library/music-entities'

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
  const host = useRef<HTMLParagraphElement>(null)
  const [visible, setVisible] = useState(false)
  useEffect(() => {
    if (item.publishedAt || !host.current) return
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) {
        setVisible(true)
        observer.disconnect()
      }
    })
    observer.observe(host.current)
    return () => observer.disconnect()
  }, [item.id, item.publishedAt])
  const { detail, busy: dateBusy } = useEntityDetail(
    item,
    0,
    visible && !item.publishedAt && !item.reference
  )
  const publishedAt =
    item.publishedAt || (detail?.item.id === item.id ? detail.item.publishedAt : undefined)
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
      <div ref={host} className="library-cover-subtitle">
        {item.trackCount} 首 ·{' '}
        {publishedAt ? (
          formatReleaseDate(publishedAt)
        ) : dateBusy ? (
          <Skeleton className="inline-block h-3 w-20 align-middle" />
        ) : (
          '发行日期未知'
        )}
      </div>
      {sourceName && <p className="library-cover-subtitle">{sourceName}</p>}
    </CollectionContextMenu>
  )
}
