import { Cover } from '@/components/music/cover'
import type { AggregatePlaylist } from './aggregate-library'

export function AggregateCover({
  playlist,
  className = ''
}: {
  playlist: AggregatePlaylist
  className?: string
}) {
  const count = playlist.covers.length
  return (
    <div
      className={`grid aspect-square overflow-hidden rounded-xl ${count > 1 ? 'grid-cols-2' : 'grid-cols-1'} ${className}`}
      aria-label={`${playlist.name}的封面`}
    >
      {(count
        ? count > 1
          ? Array.from({ length: 4 }, (_, i) => playlist.covers[i % count])
          : playlist.covers
        : ['']
      ).map((cover, i) => (
        <Cover key={i} cover={cover} className="size-full min-h-0 rounded-none" />
      ))}
    </div>
  )
}
