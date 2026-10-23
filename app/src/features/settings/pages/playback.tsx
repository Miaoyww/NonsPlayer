import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import {
  errorText,
  nativeCall,
  usePlayer,
  type OutputDevice,
  type PlaybackEngine
} from '@/lib/player'
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
  const state = usePlayer('deviceId', 'playbackOptions', 'playbackEngine')
  const webAudio = state.playbackEngine === 'webAudio'
  const supportsEngineChoice =
    isTauri() && typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent)
  const chooseAudioOutput =
    typeof navigator === 'undefined'
      ? undefined
      : (
          navigator.mediaDevices as
            (MediaDevices & { selectAudioOutput?: () => Promise<MediaDeviceInfo> }) | undefined
        )?.selectAudioOutput
  const [switchingEngine, setSwitchingEngine] = useState(false)
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
    const fetchDevices = webAudio
      ? import('@/features/playback/audio-output').then(async (module) => {
          if (disposed) return []
          const supported = module.browserOutput().supportsDevices
          if (!disposed) setBrowserDevicesSupported(supported)
          return module.browserOutputDevices()
        })
      : nativeCall<OutputDevice[]>('output_devices')
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
  }, [webAudio])
  const items = [
    { value: null, label: '跟随系统默认' },
    ...devices.map((device) => ({ value: device.id, label: device.name }))
  ]
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">播放设置</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          设置播放引擎、输出设备、音质与播放过渡。
        </p>
      </div>
      {supportsEngineChoice && (
        <SettingsCard
          searchId="playback-engine"
          title="播放引擎"
          description={
            webAudio
              ? '使用浏览器 audio 播放，支持的音频格式由系统 WebView 决定。'
              : '使用 GStreamer 播放，支持输出设备选择、渐变与无缝播放。'
          }
        >
          <Select
            items={[
              { value: 'webAudio', label: 'WebAudio' },
              { value: 'gstreamer', label: 'GStreamer' }
            ]}
            value={state.playbackEngine}
            disabled={switchingEngine || switching || savingTransitions}
            onValueChange={(engine) => {
              if (!engine || engine === state.playbackEngine) return
              setSwitchingEngine(true)
              setError(undefined)
              void nativeCall('set_playback_engine', { engine: engine as PlaybackEngine })
                .catch((cause) => {
                  reportCaughtError('app/src/features/settings/pages/playback.tsx', cause)
                  setError(errorText(cause))
                })
                .finally(() => setSwitchingEngine(false))
            }}
          >
            <SelectTrigger className="w-60" aria-label="播放引擎">
              <SelectValue>{webAudio ? 'WebAudio' : 'GStreamer'}</SelectValue>
            </SelectTrigger>
            <SelectContent alignItemWithTrigger={false}>
              <SelectGroup>
                <SelectItem value="webAudio">WebAudio</SelectItem>
                <SelectItem value="gstreamer">GStreamer</SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
          {switchingEngine && (
            <p role="status" className="text-sm text-muted-foreground">
              正在切换播放引擎…
            </p>
          )}
        </SettingsCard>
      )}
      <SettingsCard
        searchId="output-device"
        title="输出设备"
        description={
          webAudio && !browserDevicesSupported
            ? '系统 WebView 不支持输出设备选择。'
            : '选择播放设备，或跟随系统默认输出。'
        }
      >
        <Select
          items={items}
          value={state.deviceId}
          disabled={
            !isTauri() || switching || switchingEngine || (webAudio && !browserDevicesSupported)
          }
          onOpenChange={(open) => {
            if (open)
              void (
                webAudio
                  ? import('@/features/playback/audio-output').then((module) =>
                      module.browserOutputDevices()
                    )
                  : nativeCall<OutputDevice[]>('output_devices')
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
              if (webAudio) {
                const module = await import('@/features/playback/audio-output')
                await module.selectBrowserOutput(deviceId)
                try {
                  await nativeCall('player_device', { deviceId })
                } catch (cause) {
                  await module.selectBrowserOutput(state.deviceId)
                  throw cause
                }
              } else await nativeCall('player_device', { deviceId })
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
        {webAudio && chooseAudioOutput && (
          <Button
            variant="outline"
            size="sm"
            className="w-fit"
            disabled={switching || switchingEngine}
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
            disabled={!isTauri() || savingTransitions || switchingEngine}
            onCheckedChange={(checked) => updateTransitions({ fades: checked })}
          />
          启用播放/暂停渐变
        </label>
      </SettingsCard>
      <SettingsCard
        searchId="gapless"
        title="无缝播放（Gapless）"
        description={
          webAudio
            ? 'WebAudio 暂不支持采样级无缝播放。'
            : '预加载下一首并连续衔接，可与播放/暂停渐变同时开启。自动衔接不淡出、不重新渐入；关闭后在歌曲结束时切换。'
        }
      >
        <label className="flex items-center gap-2 text-sm">
          <Switch
            checked={!webAudio && state.playbackOptions.gapless}
            disabled={!isTauri() || savingTransitions || switchingEngine || webAudio}
            onCheckedChange={(checked) =>
              updateTransitions({ gapless: checked, ...(checked ? { crossfade: false } : {}) })
            }
          />
          启用无缝播放
        </label>
      </SettingsCard>
      {supportsEngineChoice && (
        <SettingsCard
          searchId="crossfade"
          title="交叉淡入淡出"
          description={
            webAudio ? '自动切歌时重叠播放，与无缝播放互斥。' : '切换到 WebAudio 后可用。'
          }
        >
          <label className="flex items-center gap-2 text-sm">
            <Switch
              checked={webAudio && state.playbackOptions.crossfade}
              disabled={!isTauri() || !webAudio || savingTransitions || switchingEngine}
              onCheckedChange={(checked) =>
                updateTransitions({ crossfade: checked, ...(checked ? { gapless: false } : {}) })
              }
            />
            启用交叉淡入淡出
          </label>
          <label className="flex items-center gap-2 text-sm">
            过渡时长
            <Select
              value={state.playbackOptions.crossfadeDurationMs ?? 3000}
              disabled={
                !webAudio ||
                !state.playbackOptions.crossfade ||
                savingTransitions ||
                switchingEngine
              }
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
      )}
      {savingTransitions && (
        <p role="status" className="text-sm text-muted-foreground">
          正在保存播放设置…
        </p>
      )}
      {(error || optionsError) && <ErrorNotice error={error ?? optionsError} />}
    </div>
  )
}
