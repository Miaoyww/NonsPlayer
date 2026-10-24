import { sourceLabel } from '@/features/music/source-label'
import { useMusicSources } from '@/features/music/use-music-sources'
import type { MusicCollection } from '@/features/workspace/music-navigation'

export function CollectionKind({
  collection,
  label
}: {
  collection: MusicCollection
  label?: string
}) {
  const { statuses } = useMusicSources()
  const kind =
    collection.kind === 'playlist' ? '歌单' : collection.kind === 'album' ? '专辑' : '艺人'
  return (
    <span className="library-detail-kind">
      {label ??
        `${kind} · ${sourceLabel(
          collection.reference?.source ?? 'local',
          statuses.map((status) => status.descriptor)
        )}`}
    </span>
  )
}
