import { createContext } from 'react'

export interface Options {
  quality: string
  allowDowngrade: boolean
}

// Keep context identity stable when Fast Refresh replaces playback components.
export const OptionsContext = createContext<{
  options: Options
  busy: boolean
  error?: string
  update: (patch: Partial<Options>) => void
} | null>(null)
