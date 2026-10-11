import { Switch } from '@/components/ui/switch'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall, usePlayer, type OutputDevice } from '@/lib/player'
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
  const state = usePlayer()
  const { options, busy, error: optionsError, update } = useMusicOptions()
  const [devices, setDevices] = useState<OutputDevice[]>([])
  const [error, setError] = useState<string>()
  const [switching, setSwitching] = useState(false)
  const [savingTransitions, setSavingTransitions] = useState(false)
  function updateTransitions(patch: Partial<typeof state.playbackOptions>) {
    if (savingTransitions) return
    setSavingTransitions(true)
    setError(undefined)
    void nativeCall('set_playback_options', { options: { ...state.playbackOptions, ...patch } })
      .catch((cause) => setError(errorText(cause)))
      .finally(() => setSavingTransitions(false))
  }
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    void nativeCall<OutputDevice[]>('output_devices')
      .then((value) => {
        if (!disposed) setDevices(value)
      })
      .catch((cause) => {
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
        <p className="mt-2 text-sm text-muted-foreground">设置输出设备、音质与播放过渡。</p>
      </div>
      <SettingsCard title="输出设备" description="选择播放设备，或跟随系统默认输出。">
        <Select
          items={items}
          value={state.deviceId}
          disabled={!isTauri() || switching}
          onOpenChange={(open) => {
            if (open)
              void nativeCall<OutputDevice[]>('output_devices')
                .then(setDevices)
                .catch((cause) => setError(errorText(cause)))
          }}
          onValueChange={(deviceId) => {
            setSwitching(true)
            setError(undefined)
            void nativeCall('player_device', { deviceId })
              .catch((cause) => setError(errorText(cause)))
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
      </SettingsCard>
      <SettingsCard title="允许音质降级" description="所选音质不可用时自动降级。">
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
        title="渐入渐出"
        description="播放、继续时渐入，暂停、停止时渐出；手动与自动切歌均先渐出再渐入，每段 800 毫秒。"
      >
        <label className="flex items-center gap-2 text-sm">
          <Switch
            checked={state.playbackOptions.fades}
            disabled={!isTauri() || savingTransitions}
            onCheckedChange={(checked) => updateTransitions({ fades: checked })}
          />
          启用渐入渐出
        </label>
      </SettingsCard>
      <SettingsCard
        title="无缝播放（Gapless）"
        description="预加载下一首并连续衔接，可与渐入渐出同时开启。关闭后在歌曲结束时切换。"
      >
        <label className="flex items-center gap-2 text-sm">
          <Switch
            checked={state.playbackOptions.gapless}
            disabled={!isTauri() || savingTransitions}
            onCheckedChange={(checked) => updateTransitions({ gapless: checked })}
          />
          启用无缝播放
        </label>
      </SettingsCard>
      {savingTransitions && (
        <p role="status" className="text-sm text-muted-foreground">
          正在保存播放设置…
        </p>
      )}
      {(error || optionsError) && (
        <p role="alert" className="text-sm text-destructive">
          {error ?? optionsError}
        </p>
      )}
    </div>
  )
}
