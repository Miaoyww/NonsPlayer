import { errorText, nativeCall } from '@/lib/player'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { LocalMv } from './local-mv'
import { clampLocalMvOffset } from './sync'

export type FullscreenDisplayMode = 'lyrics' | 'mv'
export type MvActivation = FullscreenDisplayMode | 'select'

export function useLocalMv(trackKey: string | undefined, onError: (message: string) => void) {
  const [mode, setMode] = useState<FullscreenDisplayMode>('lyrics')
  const [selection, setSelection] = useState<LocalMv>()
  const [linked, setLinked] = useState(false)
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
    setLinked(false)
  }, [trackKey])

  useEffect(
    () => () => {
      generation.current++
      offsetWrite.current++
    },
    []
  )

  const toggle = useCallback(async (): Promise<MvActivation | undefined> => {
    if (!trackKey || picking.current) return undefined
    if (mode === 'mv') {
      setMode('lyrics')
      return 'lyrics'
    }
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      const value = await nativeCall<LocalMv | null>('local_mv', { key: trackKey })
      if (serial !== generation.current) return undefined
      setSelection(value ?? undefined)
      setLinked(!!value)
      if (value?.available) {
        setMode('mv')
        return 'mv'
      }
      return 'select'
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
      return undefined
    } finally {
      if (serial === generation.current) {
        picking.current = false
        setBusy(false)
      }
    }
  }, [mode, onError, trackKey])

  const choose = useCallback(async () => {
    if (!trackKey || picking.current) return
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      const value = await nativeCall<LocalMv | null>('pick_local_mv', { key: trackKey })
      if (serial !== generation.current || !value) return
      setSelection(value)
      setLinked(false)
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
    } finally {
      if (serial === generation.current) {
        picking.current = false
        setBusy(false)
      }
    }
  }, [onError, trackKey])

  const apply = useCallback(async () => {
    if (!trackKey || !selection?.available || picking.current) return false
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      if (linked) {
        setMode('mv')
        return true
      }
      const value = await nativeCall<LocalMv>('link_local_mv', {
        key: trackKey,
        path: selection.path
      })
      if (serial !== generation.current) return false
      setSelection(value)
      setLinked(true)
      setMode('mv')
      return true
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
      return false
    } finally {
      if (serial === generation.current) {
        picking.current = false
        setBusy(false)
      }
    }
  }, [linked, onError, selection, trackKey])

  const refreshBinding = useCallback(async () => {
    if (!trackKey || linked || picking.current) return
    const serial = ++generation.current
    try {
      const value = await nativeCall<LocalMv | null>('local_mv', { key: trackKey })
      if (serial !== generation.current) return
      setSelection(value ?? undefined)
      setLinked(!!value)
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
    }
  }, [linked, onError, trackKey])

  const unlink = useCallback(async () => {
    if (!trackKey || !linked || picking.current) return false
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      await nativeCall('unlink_local_mv', { key: trackKey })
      if (serial !== generation.current) return false
      offsetWrite.current++
      setSelection(undefined)
      setLinked(false)
      setMode('lyrics')
      return true
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
      return false
    } finally {
      if (serial === generation.current) {
        picking.current = false
        setBusy(false)
      }
    }
  }, [linked, onError, trackKey])

  const setOffset = useCallback(
    async (value: number) => {
      if (!trackKey || !linked || !selection?.available) return
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
    [linked, onError, selection, trackKey]
  )

  return {
    mode,
    selection,
    linked,
    busy,
    toggle,
    choose,
    apply,
    refreshBinding,
    unlink,
    setOffset
  }
}
