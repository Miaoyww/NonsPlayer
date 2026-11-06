import { createContext, useContext, useState, type ReactNode } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Menu } from '@base-ui/react/menu'
import { ImagePlus, RefreshCw, Undo2 } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { ActionButton } from '@/components/music/action-button'
import { nativeCall } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { ArtistAvatarCandidateDialog } from './candidate-dialog'
import { ARTIST_AVATAR_SOURCE, type ArtistAvatarProgress } from './types'

interface ArtistAvatarActionsValue {
  busy: boolean
  openCandidates: (artist: { id: string; name: string }) => void
  rematch: (artist: { id: string; name: string }) => Promise<void>
  restore: (artist: { id: string; name: string }) => Promise<void>
}

const ArtistAvatarActions = createContext<ArtistAvatarActionsValue | null>(null)

export function useArtistAvatarActions() {
  const value = useContext(ArtistAvatarActions)
  if (!value) throw new Error('ArtistAvatarActionsProvider is required')
  return value
}

export function ArtistAvatarActionsProvider({
  children,
  onError,
  onNotice
}: {
  children: ReactNode
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const [candidate, setCandidate] = useState<{ id: string; name: string }>()
  const [pending, setPending] = useState<string>()

  async function rematch(artist: { id: string; name: string }) {
    if (pending) return
    setPending(artist.id)
    try {
      await nativeCall<ArtistAvatarProgress>('local_artist_avatar_start', {
        artistIds: [artist.id],
        all: false,
        source: ARTIST_AVATAR_SOURCE,
        force: true
      })
      invalidateNativeCache(
        ['local_entities', 'local_entity_detail'],
        'refresh',
        'artist-avatar.refresh'
      )
      onNotice(`已开始重新匹配「${artist.name}」的头像。`)
    } catch (cause) {
      reportCaughtError('app/src/features/local/artist-avatar/actions.tsx', cause)
      onError(cause)
    } finally {
      setPending(undefined)
    }
  }

  async function restore(artist: { id: string; name: string }) {
    if (pending) return
    setPending(artist.id)
    try {
      await nativeCall('local_artist_avatar_restore', { artistId: artist.id })
      invalidateNativeCache(
        ['local_entities', 'local_entity_detail'],
        'refresh',
        'nativeCall.local_artist_avatar_restore'
      )
      onNotice(`已恢复「${artist.name}」的默认封面，并停止自动补齐。`)
    } catch (cause) {
      reportCaughtError('app/src/features/local/artist-avatar/actions.tsx', cause)
      onError(cause)
    } finally {
      setPending(undefined)
    }
  }

  return (
    <ArtistAvatarActions.Provider
      value={{
        busy: !!pending,
        openCandidates: setCandidate,
        rematch,
        restore
      }}
    >
      {children}
      <ArtistAvatarCandidateDialog
        target={candidate}
        onClose={() => setCandidate(undefined)}
        onError={(cause) => onError(cause)}
        onNotice={onNotice}
      />
    </ArtistAvatarActions.Provider>
  )
}

export function ArtistAvatarContextItems({
  artistId,
  artistName
}: {
  artistId: string
  artistName: string
}) {
  const actions = useArtistAvatarActions()
  const artist = { id: artistId, name: artistName }
  return (
    <>
      <ContextMenu.Item disabled={!isTauri()} onClick={() => actions.openCandidates(artist)}>
        <ImagePlus aria-hidden="true" />
        更换头像匹配
      </ContextMenu.Item>
      <ContextMenu.Item
        disabled={!isTauri() || actions.busy}
        onClick={() => void actions.rematch(artist)}
      >
        <RefreshCw aria-hidden="true" />
        重新匹配
      </ContextMenu.Item>
      <ContextMenu.Item
        disabled={!isTauri() || actions.busy}
        onClick={() => void actions.restore(artist)}
      >
        <Undo2 aria-hidden="true" />
        恢复默认封面
      </ContextMenu.Item>
    </>
  )
}

export function ArtistAvatarDetailMenu({
  artistId,
  artistName
}: {
  artistId: string
  artistName: string
}) {
  const actions = useArtistAvatarActions()
  const artist = { id: artistId, name: artistName }
  return (
    <Menu.Root>
      <Menu.Trigger
        disabled={!isTauri() || actions.busy}
        render={<ActionButton variant="ghost" aria-label="艺术家头像菜单" title="艺术家头像菜单" />}
      >
        <ImagePlus aria-hidden="true" />
        头像
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner className="z-[70]" side="bottom" align="end" sideOffset={6}>
          <Menu.Popup className="song-context-menu" aria-label="艺术家头像操作">
            <Menu.Item disabled={!isTauri()} onClick={() => actions.openCandidates(artist)}>
              <ImagePlus aria-hidden="true" />
              更换头像匹配
            </Menu.Item>
            <Menu.Item
              disabled={!isTauri() || actions.busy}
              onClick={() => void actions.rematch(artist)}
            >
              <RefreshCw aria-hidden="true" />
              重新匹配
            </Menu.Item>
            <Menu.Item
              disabled={!isTauri() || actions.busy}
              onClick={() => void actions.restore(artist)}
            >
              <Undo2 aria-hidden="true" />
              恢复默认封面
            </Menu.Item>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
