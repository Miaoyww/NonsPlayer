import { useSyncExternalStore } from 'react'
import {
  getFontSettings,
  setFont,
  subscribeFontSettings
} from '@/features/settings/appearance/font-settings'

export function useFontSettings() {
  const fonts = useSyncExternalStore(subscribeFontSettings, getFontSettings)
  return { fonts, setFont }
}
