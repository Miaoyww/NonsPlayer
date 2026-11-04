import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useContext, useEffect, useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall, usePlayer } from '@/lib/player'
import { OptionsContext, type Options } from './music-options-context'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'

const qualities = [
  { value: 'standard', label: '标准' },
  { value: 'higher', label: '较高' },
  { value: 'exhigh', label: '极高' },
  { value: 'lossless', label: '无损' },
  { value: 'hires', label: 'Hi-Res' }
]

const qualityLabels = Object.fromEntries(qualities.map(({ value, label }) => [value, label]))

interface LocalPlaybackInfo {
  trackKey: string
  available: boolean
  quality?: string | null
}

export function MusicOptionsProvider({ children }: { children: ReactNode }) {
  const [options, setOptions] = useState<Options>({ quality: 'exhigh', allowDowngrade: true })
  const [busy, setBusy] = useState(isTauri())
  const [error, setError] = useState<string>()
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    void nativeCall<Options>('music_options')
      .then((value) => {
        if (!disposed) setOptions(value)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/playback/music-options.tsx', cause)

        if (!disposed) setError(errorText(cause))
      })
      .finally(() => {
        if (!disposed) setBusy(false)
      })
    return () => {
      disposed = true
    }
  }, [])
  async function update(patch: Partial<Options>) {
    if (busy) return false
    const next = { ...options, ...patch }
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall('set_music_options', next)
      setOptions(next)
      return true
    } catch (cause) {
      reportCaughtError('app/src/features/playback/music-options.tsx', cause)
      setError(errorText(cause))
      return false
    } finally {
      setBusy(false)
    }
  }
  return (
    <OptionsContext.Provider value={{ options, busy, error, update }}>
      {children}
    </OptionsContext.Provider>
  )
}

export function useMusicOptions() {
  const value = useContext(OptionsContext)
  if (!value) throw new Error('MusicOptionsProvider is required')
  return value
}

export function QualitySelect() {
  const { options, busy, error, update } = useMusicOptions()
  const state = usePlayer(
    'revision',
    'actualQuality',
    'queue',
    'index',
    'playbackMode',
    'localPlaybackAvailable',
    'localPlaybackQuality'
  )
  const [open, setOpen] = useState(false)
  const [switching, setSwitching] = useState(false)
  const [switchError, setSwitchError] = useState<string>()
  const [localInfo, setLocalInfo] = useState<LocalPlaybackInfo>()
  const track = state.index === null ? undefined : state.queue[state.index]
  useEffect(() => {
    setLocalInfo(undefined)
  }, [track?.key, state.revision])
  if (track?.source.kind === 'local') {
    return (
      <div className="quality-control flex flex-col gap-1" data-open={false}>
        <span
          className="local-quality-indicator"
          data-slot="select-trigger"
          data-size="sm"
          aria-label="歌曲来源：本地音乐"
          title="本地音乐"
        >
          本地音乐
        </span>
      </div>
    )
  }
  const localAvailable = !!track && (localInfo?.available ?? !!state.localPlaybackAvailable)
  const availableQuality = localInfo ? localInfo.quality : state.localPlaybackQuality
  const localQuality = availableQuality
    ? (qualityLabels[availableQuality] ?? availableQuality)
    : undefined
  const localLabel = localQuality ? `本地（${localQuality}）` : '本地'
  const items = localAvailable ? [{ value: 'local', label: localLabel }, ...qualities] : qualities
  const value = localAvailable && state.playbackMode === 'local' ? 'local' : options.quality
  const currentLabel = value === 'local' ? localLabel : (qualityLabels[value] ?? value)
  const actualLabel =
    state.playbackMode === 'local' ? '本地文件' : (state.actualQuality ?? '尚未播放')

  async function change(next: string) {
    if (switching) return
    setSwitching(true)
    setSwitchError(undefined)
    try {
      if (next === 'local') {
        await nativeCall('player_set_playback_mode', {
          revision: state.revision,
          mode: 'local'
        })
      } else {
        if (next !== options.quality && !(await update({ quality: next }))) return
        await nativeCall('player_set_playback_mode', {
          revision: state.revision,
          mode: 'stream'
        })
      }
    } catch (cause) {
      reportCaughtError('app/src/features/playback/music-options.tsx', cause)
      setSwitchError(errorText(cause))
    } finally {
      setSwitching(false)
    }
  }

  return (
    <div className="quality-control flex flex-col gap-1" data-open={open}>
      <Select
        items={items}
        value={value}
        disabled={busy || switching || !isTauri()}
        onOpenChange={(next) => {
          setOpen(next)
          if (!next || !track) return
          const key = track.key
          void nativeCall<LocalPlaybackInfo>('player_local_playback')
            .then((info) => {
              if (info.trackKey === key) setLocalInfo(info)
            })
            .catch((cause) => {
              reportCaughtError('app/src/features/playback/music-options.tsx', cause)
            })
        }}
        onValueChange={(next) => {
          if (next) void change(next)
        }}
      >
        <SelectTrigger
          size="sm"
          className="border-transparent"
          aria-label="音质"
          title={`播放方式：${currentLabel} · 偏好音质：${qualityLabels[options.quality] ?? options.quality} · 实际音质：${actualLabel}`}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent side="top" alignItemWithTrigger={false}>
          <SelectGroup>
            {items.map(({ value: item, label }) => (
              <SelectItem key={item} value={item}>
                {label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      {(switchError || error) && <ErrorNotice error={switchError ?? error} />}
    </div>
  )
}
