import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { isAndroid } from '@/lib/platform'

// Serialize changes so a late exit request cannot overwrite a reopened player.
let pending = Promise.resolve()
function update(immersive: boolean) {
  const dark = document.documentElement.classList.contains('dark')
  pending = pending
    .then(() => invoke<void>('android_window_layout', { immersive, dark }))
    .catch((cause) =>
      reportCaughtError('app/src/features/workspace/use-android-window-layout.ts', cause)
    )
}

export function useAndroidWindowLayout(immersive: boolean) {
  useEffect(() => {
    if (!isAndroid() || !isTauri()) return
    const apply = () => update(immersive)
    const observer = new MutationObserver(apply)
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
    window.addEventListener('focus', apply)
    apply()
    return () => {
      observer.disconnect()
      window.removeEventListener('focus', apply)
      update(false)
    }
  }, [immersive])
}
