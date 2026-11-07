import { errorText, nativeCall } from '@/lib/player'
import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useRef, useState } from 'react'
import { clampLocalMvOffset } from './sync'
import {
  localMvSource,
  resolvedMvSource,
  type LocalMv,
  type MvBinding,
  type MvSelection,
  type ResolvedMv
} from './local-mv'

export type FullscreenDisplayMode = 'lyrics' | 'mv'
export type MvActivation = FullscreenDisplayMode | 'select'
export type MvButtonTarget = 'lyrics' | 'toggle'

interface SavedMv {
  source: string
  id: string
  path: string
}

export function mvButtonTarget(mode: FullscreenDisplayMode, page: string): MvButtonTarget {
  return page === 'mv' && mode === 'lyrics' ? 'lyrics' : 'toggle'
}

async function remoteSource(binding: MvBinding, key: string) {
  const resolved = await nativeCall<ResolvedMv>('resolve_track_mv', { key })
  return {
    source: binding.source,
    id: binding.id,
    title: binding.title,
    offsetMs: binding.offsetMs,
    url: resolvedMvSource(resolved),
    local: false
  } satisfies MvSelection
}

async function view(binding: MvBinding, key: string) {
  if (binding.source === 'local' && binding.local?.available)
    return {
      source: 'local',
      id: '',
      title: binding.local.name,
      offsetMs: binding.offsetMs,
      url: localMvSource(binding.local),
      local: true,
      localPath: binding.local.path
    } satisfies MvSelection
  if (binding.source !== 'local') return remoteSource(binding, key)
  return undefined
}

export function useLocalMv(trackKey: string | undefined, onError: (message: string) => void) {
  const [mode, setMode] = useState<FullscreenDisplayMode>('lyrics')
  const [selection, setSelection] = useState<MvSelection>()
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

  useEffect(() => {
    let disposed = false
    let stop: (() => void) | undefined
    void listen<SavedMv>('mv-saved', ({ payload }) => {
      if (disposed) return
      setSelection((current) => {
        if (
          !current ||
          current.local ||
          current.source !== payload.source ||
          current.id !== payload.id
        )
          return current
        return {
          ...current,
          url: localMvSource({
            name: '',
            path: payload.path,
            available: true,
            offsetMs: current.offsetMs
          }),
          localPath: payload.path
        }
      })
    })
      .then((cleanup) => {
        if (disposed) cleanup()
        else stop = cleanup
      })
      .catch((cause) => {
        if (!disposed) onError(errorText(cause))
      })
    return () => {
      disposed = true
      stop?.()
    }
  }, [onError])

  const activate = useCallback(
    async (binding: MvBinding) => {
      if (!trackKey) return undefined
      const serial = ++generation.current
      picking.current = true
      setBusy(true)
      try {
        const value = await view(binding, trackKey)
        if (serial !== generation.current) return undefined
        if (!value) {
          setLinked(true)
          return 'select'
        }
        setSelection(value)
        setLinked(true)
        setMode('mv')
        return 'mv'
      } catch (cause) {
        if (serial === generation.current) onError(errorText(cause))
        return 'select'
      } finally {
        if (serial === generation.current) {
          picking.current = false
          setBusy(false)
        }
      }
    },
    [onError, trackKey]
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
      const value = await nativeCall<MvBinding | null>('track_mv', { key: trackKey })
      if (serial !== generation.current) return undefined
      if (!value) {
        setLinked(false)
        return 'select'
      }
      return await activate(value)
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
      return undefined
    } finally {
      if (serial === generation.current) {
        picking.current = false
        setBusy(false)
      }
    }
  }, [activate, mode, onError, trackKey])

  const choose = useCallback(async () => {
    if (!trackKey || picking.current) return
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      const value = await nativeCall<LocalMv | null>('pick_local_mv', { key: trackKey })
      if (serial !== generation.current || !value) return
      setSelection({
        source: 'local',
        id: '',
        title: value.name,
        offsetMs: value.offsetMs,
        url: localMvSource(value),
        local: true,
        localPath: value.path
      })
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

  const bindRemote = useCallback(
    async (source: string, id: string, title: string) => {
      if (!trackKey || picking.current) return false
      const serial = ++generation.current
      picking.current = true
      setBusy(true)
      try {
        const binding = await nativeCall<MvBinding>('bind_mv', {
          key: trackKey,
          source,
          mvId: id,
          title
        })
        if (serial !== generation.current) return false
        return (await activate(binding)) === 'mv'
      } catch (cause) {
        if (serial === generation.current) onError(errorText(cause))
        return false
      } finally {
        if (serial === generation.current) {
          picking.current = false
          setBusy(false)
        }
      }
    },
    [activate, onError, trackKey]
  )

  const apply = useCallback(async () => {
    if (!trackKey || !selection || picking.current) return false
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      if (linked) {
        setMode('mv')
        return true
      }
      if (!selection.localPath) return false
      const local = await nativeCall<LocalMv>('link_local_mv', {
        key: trackKey,
        path: selection.localPath
      })
      if (serial !== generation.current) return false
      setLinked(true)
      setSelection({
        ...selection,
        title: local.name,
        offsetMs: local.offsetMs,
        url: localMvSource(local),
        localPath: local.path
      })
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
    if (!trackKey || picking.current) return
    const serial = ++generation.current
    try {
      const value = await nativeCall<MvBinding | null>('track_mv', { key: trackKey })
      if (serial !== generation.current) return
      setLinked(!!value)
      if (value?.source === 'local' && value.local?.available) {
        setSelection({
          source: 'local',
          id: '',
          title: value.local.name,
          offsetMs: value.local.offsetMs,
          url: localMvSource(value.local),
          local: true,
          localPath: value.local.path
        })
      }
    } catch (cause) {
      if (serial === generation.current) onError(errorText(cause))
    }
  }, [onError, trackKey])

  const unlink = useCallback(async () => {
    if (!trackKey || picking.current) return false
    const serial = ++generation.current
    picking.current = true
    setBusy(true)
    try {
      await nativeCall('unlink_mv_binding', { key: trackKey })
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
  }, [onError, trackKey])

  const setOffset = useCallback(
    async (value: number) => {
      if (!trackKey || !linked || !selection) return
      const next = clampLocalMvOffset(value)
      if (next === selection.offsetMs) return
      const previous = selection.offsetMs
      const write = ++offsetWrite.current
      setSelection((current) => (current ? { ...current, offsetMs: next } : current))
      try {
        await nativeCall('set_mv_offset', { key: trackKey, offsetMs: next })
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
    bindRemote,
    refreshBinding,
    unlink,
    setOffset
  }
}
