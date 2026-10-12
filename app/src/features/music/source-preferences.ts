import { nativeCall } from '@/lib/player'

export interface SourcePreferences {
  current: string
  supplementary: string[]
  revision: number
}

export const sourcePreferences = {
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
