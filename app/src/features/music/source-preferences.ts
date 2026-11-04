import { nativeCall } from '@/lib/player'

export interface SourcePreferences {
  current: string
  supplementary: string[]
  revision: number
}

export const sourcePreferences = {
  beginIntent: () => nativeCall<number>('music_source_intent'),
  use: (current: string, intent: number) =>
    nativeCall<SourcePreferences>('music_use_source', { current, intent }),
  get: () => nativeCall<SourcePreferences>('music_source_preferences'),
  set: (preferences: SourcePreferences) =>
    nativeCall<SourcePreferences>('music_set_source_preferences', { ...preferences })
}

// Source switches preserve the old current source's configured supplementary role.
export function selectCurrentSource(
  preferences: SourcePreferences,
  current: string
): SourcePreferences {
  if (current === preferences.current) return preferences
  const supplementary = preferences.supplementary.filter((source) => source !== current)
  if (preferences.supplementary.includes(current)) supplementary.unshift(preferences.current)
  return { ...preferences, current, supplementary }
}

// Enabled search-capable sources participate even when they never held the current role.
export function searchSources(
  preferences: SourcePreferences,
  statuses: { source: string; enabled: boolean; search: boolean }[]
): string[] {
  const enabled = statuses
    .filter((status) => status.enabled && status.search)
    .map((status) => status.source)
  return [...new Set([preferences.current, ...preferences.supplementary, ...enabled])]
}
