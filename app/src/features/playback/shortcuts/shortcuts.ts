import { normalizeShortcut } from '@/features/playback/shortcuts/shortcut-keys'
import { isTauri } from '@tauri-apps/api/core'
import { register, unregister } from '@tauri-apps/plugin-global-shortcut'
import { errorText, nativeCall } from '@/lib/player'
import { isAndroid } from '@/lib/platform'

export const shortcutActions = [
  ['toggle', '播放/暂停音乐'],
  ['previous', '上一首'],
  ['next', '下一首'],
  ['like', '收藏此音乐'],
  ['volumeUp', '音量加'],
  ['volumeDown', '音量减']
] as const
export type ShortcutAction = (typeof shortcutActions)[number][0]
export type ShortcutSettings = { enabled: boolean; bindings: Record<ShortcutAction, string> }
const key = 'nons-shortcut-settings'
const empty = (): ShortcutSettings => ({
  enabled: false,
  bindings: { toggle: '', previous: '', next: '', like: '', volumeUp: '', volumeDown: '' }
})
let settings = empty()
try {
  const saved = JSON.parse(localStorage.getItem(key) ?? 'null')
  if (saved && typeof saved === 'object') {
    settings.enabled = saved.enabled === true
    for (const [action] of shortcutActions) {
      const value = saved.bindings?.[action]
      if (typeof value === 'string' && value.length <= 100)
        settings.bindings[action] = normalizeShortcut(value) ?? ''
    }
  }
} catch {
  /* Corrupt storage uses unbound defaults. */
}
let status = { settings, busy: false, error: undefined as string | undefined }
const listeners = new Set<() => void>()
const notify = () => {
  status = { ...status, settings }
  listeners.forEach((fn) => fn())
}
export const getShortcutStatus = () => status
export const subscribeShortcuts = (fn: () => void) => {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}
let registered: string[] = []
let recording = false
let recordingQueue = Promise.resolve(true)
export function setShortcutRecording(value: boolean) {
  if (recording === value) return recordingQueue
  recording = value
  // Serialize pause/resume so navigating away during registration still restores shortcuts.
  recordingQueue = recordingQueue.then(() => saveShortcuts(settings, false))
  return recordingQueue
}
let dispatch: (action: ShortcutAction) => void = () => {}
export function setShortcutDispatcher(fn: (action: ShortcutAction) => void) {
  dispatch = fn
}
let initialized = false
export async function initializeShortcuts() {
  if (initialized || !isTauri() || isAndroid()) return
  initialized = true
  await saveShortcuts(settings, false)
}
export async function saveShortcuts(next: ShortcutSettings, persist = true) {
  if (status.busy) return false
  status = { ...status, busy: true, error: undefined }
  notify()
  const previous = settings
  const cleaned = { enabled: next.enabled, bindings: { ...next.bindings } }
  for (const [action] of shortcutActions) cleaned.bindings[action] = cleaned.bindings[action].trim()
  async function install(value: ShortcutSettings) {
    if (!isTauri() || isAndroid() || !value.enabled || recording) return
    for (const [action] of shortcutActions) {
      const binding = value.bindings[action]
      if (!binding) continue
      await register(binding, (event) => {
        if (event.state === 'Pressed' && !recording) dispatch(action)
      })
      registered.push(binding)
    }
  }
  async function clear() {
    if (registered.length) await unregister(registered)
    registered = []
  }
  async function syncSleepBindings(value: ShortcutSettings) {
    if (isTauri() && !isAndroid())
      await nativeCall('main_sleep_shortcuts', {
        bindings:
          value.enabled && !recording
            ? shortcutActions.flatMap(([action]) =>
                value.bindings[action] ? [{ action, binding: value.bindings[action] }] : []
              )
            : []
      })
  }
  try {
    for (const [action, label] of shortcutActions) {
      const binding = normalizeShortcut(cleaned.bindings[action])
      if (binding === undefined) throw new Error(`${label}只能绑定二至四个键的组合键。`)
      cleaned.bindings[action] = binding
    }
    const values = Object.values(cleaned.bindings)
      .filter(Boolean)
      .map((value) => value.toLowerCase())
    if (new Set(values).size !== values.length) throw new Error('同一快捷键不能用于多个操作。')
    await clear()
    await install(cleaned)
    await syncSleepBindings(cleaned)
    if (persist) localStorage.setItem(key, JSON.stringify(cleaned))
    settings = cleaned
    return true
  } catch (cause) {
    status.error = `快捷键保存失败：${errorText(cause)}`
    try {
      await clear()
      if (persist) {
        await install(previous)
        await syncSleepBindings(previous)
      }
    } catch (rollback) {
      status.error += `；恢复失败：${errorText(rollback)}。请重新保存。`
      try {
        await clear()
      } catch (cleanup) {
        status.error += `；解绑失败：${errorText(cleanup)}`
      }
    }
    return false
  } finally {
    status.busy = false
    notify()
  }
}

export function isPlaybackSpace(event: KeyboardEvent) {
  if (
    event.code !== 'Space' ||
    event.isComposing ||
    event.defaultPrevented ||
    event.ctrlKey ||
    event.metaKey ||
    event.altKey ||
    event.shiftKey
  )
    return false
  const target = event.target
  return !(
    target instanceof Element &&
    target.closest(
      'input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"], [role="slider"], [role="spinbutton"], [data-shortcut-recorder]'
    )
  )
}
