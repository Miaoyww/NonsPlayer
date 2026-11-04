import { errorText, nativeCall } from '@/lib/player'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { LocalMv } from './local-mv'
import { clampLocalMvOffset } from './sync'

export type FullscreenDisplayMode = 'lyrics' | 'mv'

export function useLocalMv(trackKey: string | undefined, onError: (message: string) => void) {
  const [mode, setMode] = useState<FullscreenDisplayMode>('lyrics')
  const [selection, setSelection] = useState<LocalMv>()
  const [busy, setBusy] = useState(false)
  const generation = useRef(0)
  const picking = useRef(false)
  const offsetWrite = useRef(0)
  const currentTrack = useRef(trackKey)
  currentTrack.current = trackKey

  useEffect(() => {
    generation.current++
    offsetWrite.current++
    picking.current = false
    setBusy(false)
    setMode('lyrics')
    setSelection(undefined)
  }, [trackKey])

  useEffect(
    () => () => {
      generation.current++
      offsetWrite.current++
    },
    []
  )

  const toggle = useCallback(async () => {
    if (!trackKey || picking.current) return
    if (mode === 'mv') {
      setMode('lyrics')
      return
    }
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      let value = await nativeCall<LocalMv | null>('local_mv', { key: trackKey })
      if (serial !== generation.current) return
      if (!value?.available) {
        value = await nativeCall<LocalMv | null>('pick_local_mv', { key: trackKey })
        if (serial !== generation.current || !value) return
      }
      setSelection(value)
      setMode('mv')
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
    } finally {
      if (serial === generation.current) {
        picking.current = false
        setBusy(false)
      }
    }
  }, [mode, onError, trackKey])

  const setOffset = useCallback(
    async (value: number) => {
      if (!trackKey || !selection?.available) return
      const next = clampLocalMvOffset(value)
      if (next === selection.offsetMs) return
      const previous = selection.offsetMs
      const write = ++offsetWrite.current
      setSelection((current) => (current ? { ...current, offsetMs: next } : current))
      try {
        await nativeCall('set_local_mv_offset', { key: trackKey, offsetMs: next })
      } catch (cause) {
        if (write === offsetWrite.current && currentTrack.current === trackKey) {
          setSelection((current) => (current ? { ...current, offsetMs: previous } : current))
          onError(errorText(cause))
        }
      }
    },
    [onError, selection, trackKey]
  )

  return { mode, selection, busy, toggle, setOffset }
}
