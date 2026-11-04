import { convertFileSrc } from '@tauri-apps/api/core'

export interface LocalMv {
  name: string
  path: string
  available: boolean
  offsetMs: number
}

export interface MvBinding {
  source: string
  id: string
  title: string
  offsetMs: number
  local?: LocalMv
}

export interface MvSelection {
  source: string
  id: string
  title: string
  offsetMs: number
  url: string
  local: boolean
  localPath?: string
}

export function localMvSource(mv: LocalMv) {
  return convertFileSrc(mv.path)
}
