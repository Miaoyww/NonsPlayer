import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { useAccount } from '@/features/account/account'
import { nativeCall, usePlayer } from '@/lib/player'
import {
  initializeShortcuts,
  isPlaybackSpace,
  setShortcutDispatcher,
  type ShortcutAction
} from '@/features/playback/shortcuts/shortcuts'

export function usePlaybackShortcuts(onError: (cause: unknown) => void) {
  const state = usePlayer()
  const account = useAccount()
  const action = useRef<(action: ShortcutAction) => void>(() => {})
  const [resumedShortcut, setResumedShortcut] = useState<ShortcutAction>()
  action.current = (name) => {
    if (!isTauri()) return
    const current = state.index === null ? undefined : state.queue[state.index]
    let operation: Promise<unknown>
    if (name === 'volumeUp' || name === 'volumeDown')
      operation = nativeCall('player_volume', {
        volume: Math.max(0, Math.min(1, state.volume + (name === 'volumeUp' ? 0.05 : -0.05)))
      })
    else if (!current) return
    else if (name === 'like') {
      if (
        current.source.kind !== 'netease' ||
        !account.profile ||
        !account.likesReady ||
        account.likedIds.has(current.source.id)
      )
        return
      operation = account.toggleLike(current.source.id)
    } else
      operation = nativeCall('player_action', {
        action: name
      })
    void operation.catch(onError)
  }
  useEffect(() => {
    if (!resumedShortcut || state.index === null) return
    if (resumedShortcut === 'like' && (!account.profile || !account.likesReady)) return
    action.current(resumedShortcut)
    setResumedShortcut(undefined)
  }, [resumedShortcut, state.index, account.profile, account.likesReady])
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
