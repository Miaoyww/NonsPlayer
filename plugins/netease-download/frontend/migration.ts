import type { PluginMenuSong } from '@app/plugin-sdk'
export function migrateSong(value: PluginMenuSong): PluginMenuSong {
  const old = value.source as unknown as { kind: string; id?: number }
  if (old.kind !== 'netease') return value
  if (!Number.isSafeInteger(old.id) || old.id! <= 0) throw new Error('历史下载歌曲 ID 无效')
  return {
    ...value,
    source: { kind: 'online', reference: { source: 'netease', kind: 'track', id: String(old.id) } }
  }
}
