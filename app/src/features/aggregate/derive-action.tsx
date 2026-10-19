import { useAccount } from '@/features/account/account'
import { useMusicSources } from '@/features/music/use-music-sources'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { SyncControl } from './sync-control'
export function useCanDerive(item?: MusicCollection) {
  const { profile } = useAccount(item?.reference?.source)
  const { statuses } = useMusicSources()
  return (
    !!item &&
    !item.localId &&
    item.kind === 'playlist' &&
    !!item.reference &&
    !item.liked &&
    profile?.reference.source === item.reference.source &&
    String(profile?.userId) === String(item.creatorId) &&
    statuses.some(
      (s) =>
        s.enabled &&
        s.descriptor.source === item.reference!.source &&
        s.descriptor.capabilities.includes('browse') &&
        s.descriptor.capabilities.includes('userLibrary')
    )
  )
}
export function DeriveAction({ collection }: { collection: MusicCollection }) {
  const allowed = useCanDerive(collection)
  return allowed ? <SyncControl reference={collection.reference} /> : null
}
