import { ErrorNotice } from '@/components/error-notice'
import { cn } from '@/lib/utils'
import { reportCaughtError } from '@/lib/diagnostics'
import { readTrackInformation } from '@/features/music/queries'
import { TextSkeleton } from '@/components/music/loading'
import { Cover } from '@/components/music/cover'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { nativeCall, formatTime, errorText, type Track } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'

interface LocalInformation {
  trackNumber: number | null
  discNumber: number | null
  bitrate: number | null
  sampleRate: number | null
  bitDepth: number | null
  channels: number | null
  fileSize: number
  format: string
}

interface SongInformation {
  artists: import('@/features/music/types').MusicCredit[]
  albumReference: import('@/features/music/types').EntityRef | null
  publishedAt: number | null
}

export function SongInformationContent({
  track,
  songLink,
  className,
  inlineError = false
}: {
  track: Track
  songLink: (track: Track) => string
  className?: string
  inlineError?: boolean
}) {
  const [localInformation, setLocalInformation] = useState<LocalInformation>()
  const [information, setInformation] = useState<SongInformation>()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const [retry, setRetry] = useState(0)

  useEffect(() => {
    let disposed = false
    setInformation(undefined)
    setLocalInformation(undefined)
    setError(undefined)
    setBusy(!!track && isTauri())
    if (track.source.kind === 'local' && isTauri())
      void nativeCall<LocalInformation>('local_track_information', {
        key: track.key,
        refresh: retry > 0
      })
        .then((value) => {
          if (!disposed) setLocalInformation(value)
        })
        .catch((cause) => {
          reportCaughtError('app/src/components/music/song-information.tsx', cause)

          if (!disposed) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed) setBusy(false)
        })
    else if (isTauri())
      void readTrackInformation(track, retry > 0)
        .then((value) => {
          if (!disposed) setInformation(value)
        })
        .catch((cause) => {
          reportCaughtError('app/src/components/music/song-information.tsx', cause)

          if (!disposed) setError(errorText(cause))
        })
        .finally(() => {
          if (!disposed) setBusy(false)
        })
    return () => {
      disposed = true
    }
  }, [track, retry])

  const local = track.source.kind === 'local'
  const displayArtist = local
    ? track.artist
    : information?.artists.map((artist) => artist.name).join(' / ') || track.artist
  const fields = local
    ? [
        ['时长', formatTime(track.durationMs)],
        ['音轨', localInformation?.trackNumber?.toString() ?? ''],
        ['碟号', localInformation?.discNumber?.toString() ?? ''],
        ['码率', localInformation?.bitrate ? `${localInformation.bitrate} kbps` : ''],
        ['采样率', localInformation?.sampleRate ? `${localInformation.sampleRate} Hz` : ''],
        ['位深', localInformation?.bitDepth ? `${localInformation.bitDepth} bit` : ''],
        ['声道', localInformation?.channels?.toString() ?? ''],
        ['格式', localInformation?.format ?? ''],
        [
          '文件大小',
          localInformation?.fileSize
            ? `${(localInformation.fileSize / 1024 / 1024).toFixed(2)} MB`
            : ''
        ],
        ['路径', track.source.kind === 'local' ? track.source.path : '']
      ]
    : [
        [
          '歌手 ID',
          information?.artists.map((artist) => artist.reference?.id ?? '未知').join(' / ') ?? ''
        ],
        ['专辑 ID', information?.albumReference?.id ?? ''],
        [
          '歌曲 ID',
          (track.source.kind === 'local'
            ? track.source.neteaseId
            : track.source.track.reference.id
          )?.toString() ?? ''
        ],
        ['时长', formatTime(track.durationMs)],
        [
          '发行时间',
          information?.publishedAt
            ? new Date(information.publishedAt).toLocaleDateString('sv-SE', {
                timeZone: 'Asia/Shanghai'
              })
            : ''
        ],
        ['歌曲链接', songLink(track)]
      ]

  function field(index: number) {
    const [label, value] = fields[index]
    return (
      <div className="song-information-field" key={label}>
        <span>{label}</span>
        <p>{value || '未知'}</p>
      </div>
    )
  }

  return (
    <div className={cn('song-information-content', className)}>
      <div className="song-information-summary">
        <Cover cover={track.cover} className="song-information-cover" loading="eager" />
        <div className="song-information-summary-text">
          <h3>{track.title}</h3>
          <p>{displayArtist || '未知'}</p>
          <p>{track.album || '未知'}</p>
        </div>
      </div>
      {local ? (
        <>
          <h3 className="song-information-divider">文件信息</h3>
          <div className="song-information-fields">{[0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map(field)}</div>
        </>
      ) : (
        <>
          <h3 className="song-information-divider">制作人员</h3>
          <div className="song-information-fields">{[0, 1].map(field)}</div>
          <h3 className="song-information-divider">歌曲信息</h3>
          <div className="song-information-fields">{[2, 3, 4, 5].map(field)}</div>
        </>
      )}
      {busy && <TextSkeleton />}
      {error && (
        <div className="flex items-center gap-2 text-sm text-destructive">
          {inlineError ? (
            <p role="alert" className="whitespace-pre-line break-words">
              {error}
            </p>
          ) : (
            <ErrorNotice error={error} />
          )}
          <ActionButton variant="ghost" size="sm" onClick={() => setRetry((value) => value + 1)}>
            重试
          </ActionButton>
        </div>
      )}
    </div>
  )
}
