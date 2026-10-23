import { useSyncExternalStore } from 'react'
import { reportCaughtError } from '@/lib/diagnostics'

export type TrackDoubleClickAction = 'play' | 'next'
export type TrackDoubleClickScope = 'single' | 'queue'
export interface TrackDoubleClickSettings {
  action: TrackDoubleClickAction
  scope: TrackDoubleClickScope
}

const storageKey = 'nons-track-double-click'
const listeners = new Set<() => void>()
let settings: TrackDoubleClickSettings = { action: 'play', scope: 'single' }

function isAction(value: unknown): value is TrackDoubleClickAction {
  return value === 'play' || value === 'next'
}
function isScope(value: unknown): value is TrackDoubleClickScope {
  return value === 'single' || value === 'queue'
}

try {
  const saved = localStorage.getItem(storageKey)
  if (saved) {
    const value: unknown = JSON.parse(saved)
    if (value && typeof value === 'object') {
      const candidate = value as Record<string, unknown>
      settings = {
        action: isAction(candidate.action) ? candidate.action : settings.action,
        scope: isScope(candidate.scope) ? candidate.scope : settings.scope
      }
    }
  }
} catch (cause) {
  reportCaughtError('app/src/features/playback/use-track-double-click.ts', cause)
}

const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

function updateSettings(value: Partial<TrackDoubleClickSettings>) {
  const next = {
    action: isAction(value.action) ? value.action : settings.action,
    scope: isScope(value.scope) ? value.scope : settings.scope
  }
  if (next.action === settings.action && next.scope === settings.scope) return
  settings = next
  try {
    localStorage.setItem(storageKey, JSON.stringify(settings))
  } catch (cause) {
    reportCaughtError('app/src/features/playback/use-track-double-click.ts', cause)
  }
  listeners.forEach((notify) => notify())
}

export function getTrackDoubleClickSettings(): Readonly<TrackDoubleClickSettings> {
  return settings
}

export function useTrackDoubleClickSettings() {
  return [useSyncExternalStore(subscribe, getTrackDoubleClickSettings), updateSettings] as const
}
