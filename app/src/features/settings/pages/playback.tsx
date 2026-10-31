import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall, usePlayer, type OutputDevice } from '@/lib/player'
import { isAndroid } from '@/lib/platform'
import { useMusicOptions } from '@/features/playback/music-options'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import { SettingsCard } from '@/features/settings/settings-card'

export function PlaybackPage() {
  const state = usePlayer('deviceId', 'playbackOptions')
  const android = isAndroid()
  const chooseAudioOutput =
    !android || typeof navigator === 'undefined'
      ? undefined
      : (
          navigator.mediaDevices as
            (MediaDevices & { selectAudioOutput?: () => Promise<MediaDeviceInfo> }) | undefined
        )?.selectAudioOutput
  const { options, busy, error: optionsError, update } = useMusicOptions()
  const [browserDevicesSupported, setBrowserDevicesSupported] = useState(true)
  const [devices, setDevices] = useState<OutputDevice[]>([])
  const [error, setError] = useState<string>()
  const [switching, setSwitching] = useState(false)
  const [savingTransitions, setSavingTransitions] = useState(false)
  function updateTransitions(patch: Partial<typeof state.playbackOptions>) {
    if (savingTransitions) return
    setSavingTransitions(true)
    setError(undefined)
    void nativeCall('set_playback_options', { options: { ...state.playbackOptions, ...patch } })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/pages/playback.tsx', cause)
        return setError(errorText(cause))
      })
      .finally(() => setSavingTransitions(false))
  }
  useEffect(() => {
    if (!isTauri()) {
      setDevices([])
      return
    }
    let disposed = false
    const fetchDevices = android
      ? import('@/features/playback/audio-output').then(async (module) => {
          if (disposed) return []
          const supported = module.browserOutput().supportsDevices
          if (!disposed) setBrowserDevicesSupported(supported)
          return module.browserOutputDevices()
        })
      : nativeCall<OutputDevice[]>('player_devices').then((value) => {
          if (!disposed) setBrowserDevicesSupported(true)
          return value
        })
    void fetchDevices
      .then((value) => {
        if (!disposed) setDevices(value)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/pages/playback.tsx', cause)

        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
    }
  }, [])
  const items = [
    { value: null, label: '跟随系统默认' },
    ...devices.map((device) => ({ value: device.id, label: device.name }))
  ]
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">播放设置</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          设置输出设备、音质与播放过渡；桌面端使用系统音频设备，Android 由 WebView 输出。
        </p>
      </div>
      <SettingsCard
        searchId="output-device"
        title="输出设备"
        description={
          !browserDevicesSupported
            ? '系统 WebView 不支持输出设备选择。'
            : '选择播放设备，或跟随系统默认输出。'
        }
      >
        <Select
          items={items}
          value={state.deviceId}
          disabled={!isTauri() || switching || !browserDevicesSupported}
          onOpenChange={(open) => {
            if (open)
              void (
                android
                  ? import('@/features/playback/audio-output').then((module) =>
                      module.browserOutputDevices()
                    )
                  : nativeCall<OutputDevice[]>('player_devices')
              )
                .then(setDevices)
                .catch((cause) => {
                  reportCaughtError('app/src/features/settings/pages/playback.tsx', cause)
                  return setError(errorText(cause))
                })
          }}
          onValueChange={(deviceId) => {
            setSwitching(true)
            setError(undefined)
            void (async () => {
              const module = android ? await import('@/features/playback/audio-output') : undefined
              if (module) await module.selectBrowserOutput(deviceId)
              try {
                await nativeCall('player_device', { deviceId })
              } catch (cause) {
                if (module) await module.selectBrowserOutput(state.deviceId)
                throw cause
              }
            })()
              .catch((cause) => {
                reportCaughtError('app/src/features/settings/pages/playback.tsx', cause)
                return setError(errorText(cause))
              })
              .finally(() => setSwitching(false))
          }}
        >
          <SelectTrigger className="w-60" aria-label="输出设备">
            <SelectValue>
              {state.deviceId
                ? (devices.find((d) => d.id === state.deviceId)?.name ?? '当前输出设备')
                : '跟随系统默认'}
            </SelectValue>
          </SelectTrigger>
          <SelectContent alignItemWithTrigger={false}>
            <SelectGroup>
              {items.map(({ value, label }) => (
                <SelectItem key={value ?? 'default'} value={value}>
                  {label}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
        {chooseAudioOutput && (
          <Button
            variant="outline"
            size="sm"
            className="w-fit"
            disabled={switching}
            onClick={() => {
              setSwitching(true)
              setError(undefined)
              // Request permission directly from the user's click.
              void chooseAudioOutput
                .call(navigator.mediaDevices)
                .then(async (selected) => {
                  const module = await import('@/features/playback/audio-output')
                  await module.selectBrowserOutput(selected.deviceId)
                  try {
                    await nativeCall('player_device', { deviceId: selected.deviceId })
                  } catch (cause) {
                    await module.selectBrowserOutput(state.deviceId)
                    throw cause
                  }
                  setDevices(await module.browserOutputDevices())
                })
                .catch((cause) => setError(errorText(cause)))
                .finally(() => setSwitching(false))
            }}
          >
            选择其他设备
          </Button>
        )}
      </SettingsCard>
      <SettingsCard
        searchId="quality-downgrade"
        title="允许音质降级"
        description="所选音质不可用时自动降级。"
      >
        <label className="flex items-center gap-2 text-sm">
          <Switch
            checked={options.allowDowngrade}
            disabled={!isTauri() || busy}
            onCheckedChange={(checked) => update({ allowDowngrade: checked })}
          />
          不可用时允许降低音质
        </label>
      </SettingsCard>
      <SettingsCard
        searchId="playback-fades"
        title="播放/暂停渐变"
        description="播放、继续与暂停使用 200 毫秒渐变。"
      >
        <label className="flex items-center gap-2 text-sm">
          <Switch
            checked={state.playbackOptions.fades}
            disabled={!isTauri() || savingTransitions}
            onCheckedChange={(checked) => updateTransitions({ fades: checked })}
          />
          启用播放/暂停渐变
        </label>
      </SettingsCard>
      <SettingsCard
        searchId="crossfade"
        title="交叉淡入淡出"
        description="自动切歌时重叠播放两首歌曲。"
      >
        <label className="flex items-center gap-2 text-sm">
          <Switch
            checked={state.playbackOptions.crossfade}
            disabled={!isTauri() || savingTransitions}
            onCheckedChange={(checked) => updateTransitions({ crossfade: checked })}
          />
          启用交叉淡入淡出
        </label>
        <label className="flex items-center gap-2 text-sm">
          过渡时长
          <Select
            value={state.playbackOptions.crossfadeDurationMs ?? 3000}
            disabled={!state.playbackOptions.crossfade || savingTransitions}
            onValueChange={(value) => {
              if (value) updateTransitions({ crossfadeDurationMs: value })
            }}
          >
            <SelectTrigger className="w-28" aria-label="交叉淡入淡出时长">
              <SelectValue>
                {(state.playbackOptions.crossfadeDurationMs ?? 3000) / 1000} 秒
              </SelectValue>
            </SelectTrigger>
            <SelectContent alignItemWithTrigger={false}>
              <SelectGroup>
                {Array.from({ length: 12 }, (_, i) => (i + 1) * 1000).map((duration) => (
                  <SelectItem key={duration} value={duration}>
                    {duration / 1000} 秒
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        </label>
      </SettingsCard>
      {savingTransitions && (
        <p role="status" className="text-sm text-muted-foreground">
          正在保存播放设置…
        </p>
      )}
      {(error || optionsError) && <ErrorNotice error={error ?? optionsError} />}
    </div>
  )
}
