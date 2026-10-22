import { useEffect, useRef, useState } from 'react'
import { Menu } from '@base-ui/react/menu'
import { Ellipsis, GitBranch } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { collectionReference } from '@/features/music/legacy-reference'
import { musicClient } from '@/features/music/client'
import { useMusicNavigation, type MusicCollection } from '@/features/workspace/music-navigation'
import { derivePlaylist } from './playlist-targets'
import { aggregateCollection, useAggregateLibrary } from './aggregate-library'
import type { AccountRef } from '@/features/music/types'

export function PlatformPlaylistMenu({
  collection,
  account,
  onError
}: {
  collection: MusicCollection
  account?: AccountRef
  onError: (cause: unknown) => void
}) {
  const [busy, setBusy] = useState(false)
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])
  const { reload } = useAggregateLibrary()
  const { navigate } = useMusicNavigation()
  const reference = collectionReference(collection)
  const eligible =
    !!account &&
    account.source === reference.source &&
    collection.creatorId === account.id &&
    !collection.liked
  async function derive() {
    if (!eligible || busy) return
    setBusy(true)
    try {
      const id = await derivePlaylist(reference)
      const current = (await musicClient.currentAccount(reference.source))?.reference
      if (current?.source !== account!.source || current.id !== account!.id)
        throw new Error('账号已变化，请从音乐库打开聚合歌单')
      await reload()
      if (!mounted.current) return
      navigate(
        'local-playlist',
        '',
        aggregateCollection({
          id,
          name: collection.name,
          covers: [],
          trackCount: collection.trackCount,
          link: null
        })
      )
    } catch (cause) {
      if (mounted.current) onError(cause)
    } finally {
      if (mounted.current) setBusy(false)
    }
  }
  if (!eligible) return null
  return (
    <Menu.Root>
      <Menu.Trigger
        disabled={busy}
        render={
          <ActionButton
            variant="ghost"
            size="icon"
            aria-label="歌单菜单"
            aria-busy={busy}
            title={busy ? '正在派生聚合歌单…' : '歌单菜单'}
          />
        }
      >
        <Ellipsis aria-hidden="true" />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner className="z-[70]" side="bottom" align="end" sideOffset={6}>
          <Menu.Popup className="song-context-menu" aria-label="歌单操作">
            <Menu.Item
              disabled={!eligible || busy}
              onClick={() => void derive()}
              title={eligible ? undefined : '仅支持当前账号自建的普通歌单'}
            >
              <GitBranch aria-hidden="true" />
              派生为聚合歌单
            </Menu.Item>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
