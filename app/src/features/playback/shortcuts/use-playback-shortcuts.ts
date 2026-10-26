import { favoriteId, onlineReference } from '@/features/music/references'
import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { useAccount } from '@/features/account/account'
import { getPlayer, nativeCall, usePlayer } from '@/lib/player'
import { changePlayerVolume, playerVolume } from '../use-volume-control'
import {
  initializeShortcuts,
  isPlaybackSpace,
  setShortcutDispatcher,
  type ShortcutAction
} from '@/features/playback/shortcuts/shortcuts'

export function usePlaybackShortcuts(onError: (cause: unknown) => void) {
  const state = usePlayer('index', 'queue')
  const track = state.index === null ? undefined : state.queue[state.index]
  const account = useAccount(track ? onlineReference(track)?.source : undefined)
  const action = useRef<(action: ShortcutAction) => void>(() => {})
  const [resumedShortcut, setResumedShortcut] = useState<ShortcutAction>()
  action.current = (name) => {
    if (!isTauri()) return
    const live = getPlayer()
    const current = live.index === null ? undefined : live.queue[live.index]
    let operation: Promise<unknown>
    if (name === 'volumeUp' || name === 'volumeDown') {
      changePlayerVolume(playerVolume() + (name === 'volumeUp' ? 0.05 : -0.05), onError)
      return
    } else if (!current) return
    else if (name === 'like') {
      const currentAccount = account.getAccount(onlineReference(current)?.source ?? '')
      const id = favoriteId(current, currentAccount.profile?.reference)
      if (
        !id ||
        !currentAccount.profile ||
        currentAccount.canToggleLike === false ||
        currentAccount.favoriteStatus(id) !== 'unliked'
      )
        return
      operation = currentAccount.toggleLike(id)
    } else
      operation = nativeCall('player_action', {
        action: name
      })
    void operation.catch(onError)
  }
  useEffect(() => {
    if (!resumedShortcut || state.index === null) return
    if (resumedShortcut === 'like') {
      const id = track ? favoriteId(track, account.profile?.reference) : undefined
      if (!account.profile || id === undefined || account.favoriteStatus(id) === 'unknown') return
    }
    action.current(resumedShortcut)
    setResumedShortcut(undefined)
  }, [resumedShortcut, state.index, track, account.profile, account.favoriteStatus])
  useEffect(() => {
    setShortcutDispatcher((name) => action.current(name))
    void initializeShortcuts().catch(onError)
    let spacePressed = false
    const keydown = (event: KeyboardEvent) => {
      if (!isTauri() || !isPlaybackSpace(event)) return
      event.preventDefault()
      event.stopImmediatePropagation()
      spacePressed = true
      if (!event.repeat) action.current('toggle')
    }
    const keyup = (event: KeyboardEvent) => {
      if (event.code !== 'Space' || !spacePressed) return
      spacePressed = false
      event.preventDefault()
      event.stopImmediatePropagation()
    }
    const blur = () => {
      spacePressed = false
    }
    const resumeShortcut = (event: Event) => {
      setResumedShortcut((event as CustomEvent<ShortcutAction>).detail)
    }
    window.addEventListener('resume-playback-shortcut', resumeShortcut)
    window.addEventListener('keydown', keydown, true)
    window.addEventListener('keyup', keyup, true)
    window.addEventListener('blur', blur)
    return () => {
      window.removeEventListener('keydown', keydown, true)
      window.removeEventListener('keyup', keyup, true)
      window.removeEventListener('blur', blur)
      window.removeEventListener('resume-playback-shortcut', resumeShortcut)
      setShortcutDispatcher(() => {})
    }
  }, [onError])
}
