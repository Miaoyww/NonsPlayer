import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useEffect, useRef, useState } from 'react'
import { reportCaughtError } from '@/lib/diagnostics'

export interface DesktopLyricsState {
  enabled: boolean
  locked: boolean
}

export const desktopLyricsEvent = 'desktop-lyrics-changed'
export const desktopLyricsHoverEvent = 'desktop-lyrics-hover-changed'
export const defaultDesktopLyricsState: DesktopLyricsState = { enabled: false, locked: true }

interface DesktopLyricsHoverState {
  hovered: boolean
  revision: number
}

export function useDesktopLyricsState() {
  const [value, setValue] = useState(defaultDesktopLyricsState)
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    let unlisten: UnlistenFn | undefined
    void invoke<DesktopLyricsState>('desktop_lyrics_state')
      .then((state) => {
        if (!disposed) setValue(state)
      })
      .catch((cause) => reportCaughtError('app/src/features/desktop-lyrics/state.ts', cause))
    void listen<DesktopLyricsState>(desktopLyricsEvent, ({ payload }) => {
      if (!disposed) setValue(payload)
    })
      .then((fn) => {
        if (disposed) fn()
        else unlisten = fn
      })
      .catch((cause) => reportCaughtError('app/src/features/desktop-lyrics/state.ts', cause))
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [])
  return value
}

export function useDesktopLyricsHover() {
  const [hovered, setHovered] = useState(false)
  const revision = useRef(-1)
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    let unlisten: UnlistenFn | undefined
    const apply = (value: DesktopLyricsHoverState) => {
      if (disposed || value.revision < revision.current) return
      revision.current = value.revision
      setHovered(value.hovered)
    }
    const bind = async () => {
      const cleanup = await listen<DesktopLyricsHoverState>(
        desktopLyricsHoverEvent,
        ({ payload }) => apply(payload)
      )
      if (disposed) {
        cleanup()
        return
      }
      unlisten = cleanup
      apply(await invoke<DesktopLyricsHoverState>('desktop_lyrics_hover_state'))
    }
    void bind().catch((cause) =>
      reportCaughtError('app/src/features/desktop-lyrics/state.ts', cause)
    )
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [])
  return hovered
}

export async function restoreDesktopLyrics() {
  if (!isTauri()) return
  const state = await invoke<DesktopLyricsState>('desktop_lyrics_state')
  if (state.enabled) await invoke('desktop_lyrics_set_enabled', { enabled: true })
}

export function setDesktopLyricsEnabled(enabled: boolean) {
  return invoke<DesktopLyricsState>('desktop_lyrics_set_enabled', { enabled })
}

export function setDesktopLyricsLocked(locked: boolean) {
  return invoke<DesktopLyricsState>('desktop_lyrics_set_locked', { locked })
}

export function setDesktopLyricsHeight(height: number) {
  return invoke('desktop_lyrics_set_height', { height })
}

export function dragDesktopLyrics() {
  return invoke('desktop_lyrics_begin_drag')
}
