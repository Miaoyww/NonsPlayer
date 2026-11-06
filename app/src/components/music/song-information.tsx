import { ErrorNotice } from '@/components/error-notice'
import { cn } from '@/lib/utils'
import { reportCaughtError } from '@/lib/diagnostics'
import { readTrackInformation } from '@/features/music/queries'
import { TextSkeleton } from '@/components/music/loading'
import { Cover } from '@/components/music/cover'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { nativeCall, formatTime, errorText, type Track } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'

interface LocalInformation {
  path?: string
  title?: string | null
  artist?: string | null
  album?: string | null
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
    const failed = (cause: unknown) => {
      reportCaughtError('app/src/components/music/song-information.tsx', cause)
      if (!disposed) setError((previous) => [previous, errorText(cause)].filter(Boolean).join('\n'))
    }
    if (isTauri()) {
      const local = track.source.kind === 'local'
      const fileRequest = nativeCall<LocalInformation | null>(
        local ? 'local_track_information' : 'linked_local_track_information',
        { key: track.key, refresh: retry > 0 }
      )
        .then((value) => (disposed ? undefined : setLocalInformation(value ?? undefined)))
        .catch((cause) => {
          failed(cause)
          return undefined
        })
      const onlineRequest = local
        ? Promise.resolve()
        : readTrackInformation(track, retry > 0)
            .then((value) => {
              if (!disposed) setInformation(value)
            })
            .catch((cause) => {
              failed(cause)
              return undefined
            })
      void Promise.all([fileRequest, onlineRequest]).finally(() => {
        if (!disposed) setBusy(false)
      })
    }
    return () => {
      disposed = true
    }
  }, [track, retry])

  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    const subscriptions = ['local-library-updated', 'downloads-changed'].map((event) =>
      listen(event, () => {
        if (!disposed) setRetry((value) => value + 1)
      })
        .then((stop) => {
          if (disposed) stop()
          return stop
        })
        .catch((cause) => {
          reportCaughtError('app/src/components/music/song-information.tsx.listen', cause)
        })
    )
    return () => {
      disposed = true
      for (const subscription of subscriptions) void subscription.then((stop) => stop?.())
    }
  }, [])

  const local = track.source.kind === 'local'
  const displayArtist = local
    ? track.artist
    : information?.artists.map((artist) => artist.name).join(' / ') || track.artist
  const localFields: string[][] = [
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
      localInformation?.fileSize ? `${(localInformation.fileSize / 1024 / 1024).toFixed(2)} MB` : ''
    ],
    ['路径', track.source.kind === 'local' ? track.source.path : (localInformation?.path ?? '')]
  ]
  const linkedTrackFields: string[][] = [
    ['歌曲名称', localInformation?.title ?? ''],
    ['艺术家', localInformation?.artist ?? ''],
    ['专辑', localInformation?.album ?? '']
  ]
  const onlineFields = [
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

  function field([label, value]: string[]) {
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
      {!local && (
        <>
          <h3 className="song-information-divider">制作人员</h3>
          <div className="song-information-fields">{onlineFields.slice(0, 2).map(field)}</div>
          <h3 className="song-information-divider">歌曲信息</h3>
          <div className="song-information-fields">{onlineFields.slice(2).map(field)}</div>
        </>
      )}
      {(local || localInformation) && (
        <>
          <h3 className="song-information-divider">{local ? '文件信息' : '本地文件信息'}</h3>
          {!local && <div className="song-information-fields">{linkedTrackFields.map(field)}</div>}
          <div className="song-information-fields">{localFields.map(field)}</div>
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
