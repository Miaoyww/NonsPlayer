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
        if (!disposed) setError(errorText(cause))
      })
      .finally(() => {
        if (!disposed) setBusy(false)
      })
    return () => {
      disposed = true
    }
  }, [])
  function update(patch: Partial<Options>) {
    if (busy) return
    const next = { ...options, ...patch }
    setBusy(true)
    setError(undefined)
    void nativeCall('set_music_options', next)
      .then(() => setOptions(next))
      .catch((cause) => setError(errorText(cause)))
      .finally(() => setBusy(false))
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
  const { actualQuality } = usePlayer('actualQuality')
  const [open, setOpen] = useState(false)
  return (
    <div className="quality-control flex flex-col gap-1" data-open={open}>
      <Select
        items={qualities}
        value={options.quality}
        disabled={busy || !isTauri()}
        onOpenChange={setOpen}
        onValueChange={(quality) => {
          if (quality) update({ quality })
        }}
      >
        <SelectTrigger
          size="sm"
          className="border-transparent"
          aria-label="音质"
          title={`偏好音质：${qualities.find((q) => q.value === options.quality)?.label ?? options.quality} · 实际音质：${actualQuality ?? '尚未播放'}`}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent side="top" alignItemWithTrigger={false}>
          <SelectGroup>
            {qualities.map(({ value, label }) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      {error && <ErrorNotice error={error} />}
    </div>
  )
}
