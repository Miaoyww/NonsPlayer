import { convertFileSrc } from '@tauri-apps/api/core'

export interface LocalMv {
  name: string
  path: string
  available: boolean
  offsetMs: number
}

export function localMvSource(mv: LocalMv) {
  return convertFileSrc(mv.path)
}
