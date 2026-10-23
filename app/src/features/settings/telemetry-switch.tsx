import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useState, useRef } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Switch } from '@/components/ui/switch'
import { ErrorNotice } from '@/components/error-notice'
import { SettingsCard } from './settings-card'
import { nativeCall, errorText } from '@/lib/player'

export function TelemetrySwitch() {
  const [enabled, setEnabled] = useState<boolean>()
  const [error, setError] = useState<string>()
  const [saving, setSaving] = useState(false)
  const busy = useRef(false)
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    void nativeCall<boolean>('telemetry_enabled')
      .then((value) => {
        if (!disposed) setEnabled(value)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/telemetry-switch.tsx', cause)

        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
    }
  }, [])
  async function change(value: boolean) {
    if (busy.current) return
    busy.current = true
    setSaving(true)
    setError(undefined)
    try {
      setEnabled(await nativeCall<boolean>('set_telemetry_enabled', { enabled: value }))
    } catch (cause) {
      reportCaughtError('app/src/features/settings/telemetry-switch.tsx', cause)

      setError(errorText(cause))
    } finally {
      busy.current = false
      setSaving(false)
    }
  }
  return (
    <SettingsCard
      searchId="telemetry"
      title="匿名使用统计"
      description="默认开启。每次启动上报随机安装标识、平台、应用版本，以及启用的插件和适配器名称与标识；用于统计启动次数和安装数量。不采集账号、歌曲、文件路径或硬件标识。关闭后停止本次待重试和后续启动上报，已发送的数据会保留。重新开启从下次启动生效。"
    >
      <div className="flex flex-col items-end gap-2">
        <Switch
          aria-label="匿名使用统计"
          checked={enabled ?? false}
          disabled={enabled === undefined || saving}
          onCheckedChange={(value) => void change(value)}
        />
        {error && <ErrorNotice error={error} />}
      </div>
    </SettingsCard>
  )
}
