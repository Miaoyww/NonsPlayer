import { reportCaughtError } from '@/lib/diagnostics'
export const defaultFontStack = '"Segoe UI Variable", "Segoe UI", "Microsoft YaHei", sans-serif'
export const fontSettingsKey = 'nons-font-settings'
export type FontSettings = { app: string; lyrics: string }
const listeners = new Set<() => void>()
let settings: FontSettings = { app: '', lyrics: '' }

function validFamily(value: unknown): value is string {
  return typeof value === 'string' && value.length <= 256 && !/[\u0000-\u001f\u007f]/.test(value)
}
export function fontStack(family: string) {
  return family
    ? `"${family.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}", ${defaultFontStack}`
    : defaultFontStack
}
function apply() {
  const style = document.documentElement.style
  style.setProperty('--nons-app-font', fontStack(settings.app))
  style.setProperty(
    '--nons-lyrics-font',
    settings.lyrics ? fontStack(settings.lyrics) : 'var(--nons-app-font)'
  )
}
export function initializeFontSettings() {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(fontSettingsKey) ?? 'null')
    if (saved && typeof saved === 'object') {
      const value = saved as Partial<FontSettings>
      settings = {
        app: validFamily(value.app) ? value.app : '',
        lyrics: validFamily(value.lyrics) ? value.lyrics : ''
      }
    }
  } catch (cause) {
    reportCaughtError('app/src/features/settings/font-settings.ts', cause)

    /* Invalid or unavailable storage uses the default font. */
  }
  apply()
}
export function getFontSettings() {
  return settings
}
export function subscribeFontSettings(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
export function setFont(target: keyof FontSettings, family: string) {
  if (!validFamily(family) || settings[target] === family) return
  settings = { ...settings, [target]: family }
  try {
    localStorage.setItem(fontSettingsKey, JSON.stringify(settings))
  } catch (cause) {
    reportCaughtError('app/src/features/settings/font-settings.ts', cause)

    /* Selection remains effective for this session. */
  }
  apply()
  listeners.forEach((notify) => notify())
}
