import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useRef } from 'react'
import { onBackButtonPress } from '@tauri-apps/api/app'
import { isTauri, type PluginListener } from '@tauri-apps/api/core'
import { isAndroid } from '@/lib/platform'

export function useAndroidBack(options: {
  nowPlaying: boolean
  closePlayer: () => void
  back: () => void
}) {
  const current = useRef(options)
  current.current = options
  useEffect(() => {
    if (!isTauri() || !isAndroid()) return
    let disposed = false
    let listener: PluginListener | undefined
    void onBackButtonPress(() => {
      if (disposed) return
      const request = new Event('nons-android-back', { cancelable: true })
      window.dispatchEvent(request)
      if (request.defaultPrevented) return
      const { nowPlaying, closePlayer, back } = current.current
      if (nowPlaying) closePlayer()
      else back()
    })
      .then((registered) => {
        if (disposed) void registered.unregister()
        else listener = registered
      })
      .catch((cause) => reportCaughtError('app/src/features/workspace/use-android-back.ts', cause))
    return () => {
      disposed = true
      void listener?.unregister()
    }
  }, [])
}
