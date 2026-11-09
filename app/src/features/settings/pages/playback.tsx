import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall, usePlayer, type OutputDevice } from '@/lib/player'
import { isAndroid } from '@/lib/platform'
import { qualities, useMusicOptions } from '@/features/playback/now-playing/music-options'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { SettingsCard } from '@/features/settings/shell/settings-card'
import { useTrackDoubleClickSettings } from '@/features/playback/track/use-track-double-click'

export function PlaybackPage() {
  const state = usePlayer('deviceId', 'playbackOptions')
  const [doubleClick, updateDoubleClick] = useTrackDoubleClickSettings()
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
  const [savingOptions, setSavingOptions] = useState(false)
  function updatePlaybackOptions(patch: Partial<typeof state.playbackOptions>) {
    if (savingOptions) return
    setSavingOptions(true)
    setError(undefined)
    void nativeCall('set_playback_options', { options: { ...state.playbackOptions, ...patch } })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/pages/playback.tsx', cause)
        return setError(errorText(cause))
      })
      .finally(() => setSavingOptions(false))
  }
  useEffect(() => {
    if (!isTauri()) {
      setDevices([])
      return
    }
    let disposed = false
    const fetchDevices = android
      ? import('@/features/playback/audio/audio-output').then(async (module) => {
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
        <p className="mt-2 text-sm text-muted-foreground">管理播放控制、歌曲操作与输出设备。</p>
      </div>
      <section aria-labelledby="playback-controls-heading" className="flex flex-col gap-8">
        <h3 id="playback-controls-heading" className="text-base font-semibold">
          播放控制
        </h3>
        <SettingsCard
          searchId="autoplay"
          title="自动播放"
          description="进入 Nons 后自动播放当前音乐。"
        >
          <label className="flex items-center gap-2 text-sm">
            <Switch
              checked={state.playbackOptions.autoplay}
              disabled={!isTauri() || savingOptions}
              onCheckedChange={(checked) => updatePlaybackOptions({ autoplay: checked })}
            />
            启动后自动播放
          </label>
        </SettingsCard>
        <SettingsCard
          searchId="quality-preference"
          title="音质偏好"
          description="流媒体优先请求该音质，不可用时按下方设置处理。"
        >
          <Select
            items={qualities}
            value={options.quality}
            disabled={!isTauri() || busy}
            onValueChange={(quality) => {
              if (quality) void update({ quality })
            }}
          >
            <SelectTrigger className="w-40" aria-label="音质偏好">
              <SelectValue />
            </SelectTrigger>
            <SelectContent alignItemWithTrigger={false}>
              <SelectGroup>
                {qualities.map(({ value, label }) => (
                  <SelectItem key={value} value={value}>
                    {label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
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
              disabled={!isTauri() || savingOptions}
              onCheckedChange={(checked) => updatePlaybackOptions({ fades: checked })}
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
              disabled={!isTauri() || savingOptions}
              onCheckedChange={(checked) => updatePlaybackOptions({ crossfade: checked })}
            />
            启用交叉淡入淡出
          </label>
          <label className="flex items-center gap-2 text-sm">
            过渡时长
            <Select
              value={state.playbackOptions.crossfadeDurationMs ?? 3000}
              disabled={!state.playbackOptions.crossfade || savingOptions}
              onValueChange={(value) => {
                if (value) updatePlaybackOptions({ crossfadeDurationMs: value })
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
      </section>
      <section aria-labelledby="playback-song-actions-heading" className="flex flex-col gap-8">
        <h3 id="playback-song-actions-heading" className="text-base font-semibold">
          歌曲操作
        </h3>
        <SettingsCard
          searchId="track-double-click-action"
          title="双击播放行为"
          description="将双击歌曲加入下一曲，或插入下一曲并立即播放；保留已有播放队列。"
        >
          <ToggleGroup
            aria-label="双击播放行为"
            value={[doubleClick.action]}
            onValueChange={(values) => {
              const action = values[0]
              if (action === 'next' || action === 'play') updateDoubleClick({ action })
            }}
          >
            <ToggleGroupItem value="next">加入下一曲</ToggleGroupItem>
            <ToggleGroupItem value="play">立即播放</ToggleGroupItem>
          </ToggleGroup>
        </SettingsCard>
        <SettingsCard
          searchId="track-double-click-scope"
          title="双击播放范围"
          description="播放此队列优先：用歌曲所在列表替换当前队列，并从双击歌曲开始播放。仅播放这一首或没有所属列表时，按上方行为加入下一曲。列表使用当前筛选、排序后已加载的歌曲。"
        >
          <ToggleGroup
            aria-label="双击播放范围"
            value={[doubleClick.scope]}
            onValueChange={(values) => {
              const scope = values[0]
              if (scope === 'queue' || scope === 'single') updateDoubleClick({ scope })
            }}
          >
            <ToggleGroupItem value="queue">播放此队列</ToggleGroupItem>
            <ToggleGroupItem value="single">仅播放这一首</ToggleGroupItem>
          </ToggleGroup>
        </SettingsCard>
      </section>
      <section aria-labelledby="playback-output-heading" className="flex flex-col gap-8">
        <h3 id="playback-output-heading" className="text-base font-semibold">
          输出设备
        </h3>
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
                    ? import('@/features/playback/audio/audio-output').then((module) =>
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
                const module = android
                  ? await import('@/features/playback/audio/audio-output')
                  : undefined
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
                    const module = await import('@/features/playback/audio/audio-output')
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
      </section>
      {savingOptions && (
        <p role="status" className="text-sm text-muted-foreground">
          正在保存播放设置…
        </p>
      )}
      {(error || optionsError) && <ErrorNotice error={error ?? optionsError} />}
    </div>
  )
}
