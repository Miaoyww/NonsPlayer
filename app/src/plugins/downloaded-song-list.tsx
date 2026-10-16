import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useState } from 'react'
import { TrackList } from '@/components/music/track-list'
import type { Track } from '@/lib/player'
import { useScope } from './scope'
import { createPluginClient, type PluginMenuSong } from './sdk'

export interface DownloadedSong {
  song: PluginMenuSong
  root: string
  path: string
  completedAt?: number
  size: number
}
export function DownloadedSongList({ songs }: { songs: DownloadedSong[] }) {
  const scope = useScope('ui')
  const [tracks, setTracks] = useState<Track[]>([])
  const [entries, setEntries] = useState<DownloadedSong[]>([])
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const signature = JSON.stringify(songs)
  useEffect(() => {
    let stopped = false
    setTracks([])
    setError('')
    const client = createPluginClient(scope, () => {})
    void (async () => {
      const result: Track[] = []
      const available: DownloadedSong[] = []
      let missing = 0
      for (let start = 0; start < songs.length; start += 4) {
        const batch = await Promise.allSettled(
          songs.slice(start, start + 4).map(async (entry) => {
            const location = await client.call<{ path: string }>('files.resolve', {
              root: entry.root,
              path: entry.path
            })
            const digest = await crypto.subtle.digest(
              'SHA-256',
              new TextEncoder().encode(location.path)
            )
            const key = `local:${Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')}`
            return {
              entry,
              track: {
                ...entry.song,
                key,
                source: { kind: 'local' as const, path: location.path, neteaseId: null }
              }
            }
          })
        )
        if (stopped) return
        for (const item of batch) {
          if (item.status === 'fulfilled') {
            result.push(item.value.track)
            available.push(item.value.entry)
          } else missing++
        }
      }
      if (!stopped) {
        setTracks(result)
        setEntries(available)
        if (missing) setError(`${missing} 首歌曲文件不可用，请检查保存目录或授权`)
      }
    })()
    return () => {
      stopped = true
    }
  }, [signature, scope])
  return (
    <>
      {error && <ErrorNotice error={error} />}
      <TrackList
        tracks={tracks}
        busy={busy}
        showDuration={false}
        onPlay={(index) => {
          const entry = entries[index]
          if (!entry || busy) return
          setBusy(true)
          setError('')
          void createPluginClient(scope, () => {})
            .call('files.play', { root: entry.root, path: entry.path, track: tracks[index] })
            .catch((reason) => setError(String(reason)))
            .finally(() => setBusy(false))
        }}
        extraColumns={[
          {
            label: '下载时间',
            className: 'w-36 py-3',
            render: (_, index) =>
              entries[index].completedAt
                ? new Date(entries[index].completedAt!).toLocaleString()
                : '—'
          },
          {
            label: '大小',
            className: 'w-24 py-3',
            render: (_, index) => `${(entries[index].size / 1048576).toFixed(1)} MiB`
          }
        ]}
      />
    </>
  )
}
