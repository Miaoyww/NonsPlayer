import type { CoverImage } from '@/features/music/types'

export const ARTIST_AVATAR_SOURCE = 'netease'

export type ArtistAvatarStatus = 'auto' | 'manual' | 'notFound' | 'pending' | 'default' | 'failed'

export interface ArtistAvatarItem {
  id: string
  name: string
  cover: string
  trackCount: number
  status?: ArtistAvatarStatus
  source?: string
  platformId?: string
  platformName?: string
  avatarSize?: string
  width?: number
  height?: number
  error?: string
}

export interface ArtistAvatarPage {
  items: ArtistAvatarItem[]
  more: boolean
  total: number
}

export interface ArtistAvatarProgress {
  running: boolean
  total: number
  processed: number
  matched: number
  pending: number
  notFound: number
  failed: number
  skipped: number
  current?: string | null
  error?: string | null
}

export interface ArtistAvatarCandidate {
  source: string
  platformId: string
  name: string
  subtitle: string
  trackCount: number
  cover: CoverImage
}

export function artistAvatarStatusLabel(status?: ArtistAvatarStatus) {
  switch (status) {
    case 'auto':
      return '自动匹配'
    case 'manual':
      return '手动选择'
    case 'notFound':
      return '未找到'
    case 'pending':
      return '待确认'
    case 'default':
      return '默认封面'
    case 'failed':
      return '匹配失败'
    default:
      return '未处理'
  }
}

export function artistAvatarStatusTone(status?: ArtistAvatarStatus) {
  switch (status) {
    case 'auto':
      return 'text-emerald-600 dark:text-emerald-400'
    case 'manual':
      return 'text-sky-600 dark:text-sky-400'
    case 'pending':
      return 'text-amber-600 dark:text-amber-400'
    case 'notFound':
    case 'failed':
      return 'text-destructive'
    default:
      return 'text-muted-foreground'
  }
}
