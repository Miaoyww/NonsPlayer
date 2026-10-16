import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useSyncExternalStore } from 'react'

export type Theme = 'light' | 'dark' | 'system'
const storageKey = 'nons-theme'
const listeners = new Set<() => void>()
let theme: Theme = 'system'
try {
  const saved = localStorage.getItem(storageKey)
  if (saved === 'light' || saved === 'dark') theme = saved
} catch (cause) {
  reportCaughtError('app/src/features/settings/use-theme.ts', cause)

  /* Storage is optional. */
}
const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
function setTheme(value: Theme) {
  theme = value
  try {
    localStorage.setItem(storageKey, value)
  } catch (cause) {
    reportCaughtError('app/src/features/settings/use-theme.ts', cause)

    /* Storage is optional. */
  }
  listeners.forEach((notify) => notify())
}

export function useTheme() {
  const value = useSyncExternalStore(subscribe, () => theme)

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const apply = () => {
      const dark = theme === 'dark' || (theme === 'system' && media.matches)
      document.documentElement.classList.toggle('dark', dark)
      document.documentElement.style.colorScheme = dark ? 'dark' : 'light'
    }
    apply()
    media.addEventListener('change', apply)
    return () => media.removeEventListener('change', apply)
  }, [value])

  return [value, setTheme] as const
}
