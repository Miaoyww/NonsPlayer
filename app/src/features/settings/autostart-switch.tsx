import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Switch } from '@/components/ui/switch'
import { errorText, nativeCall } from '@/lib/player'
import { SettingsCard } from './settings-card'

type AutostartStatus = { enabled: boolean; blockedBySystem: boolean }

export function AutostartSwitch() {
  const [status, setStatus] = useState<AutostartStatus>()
  const [error, setError] = useState<string>()
  const [saving, setSaving] = useState(false)
  const busy = useRef(false)
  const revision = useRef(0)
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    const refresh = async () => {
      if (busy.current) return
      const current = ++revision.current
      try {
        const value = await nativeCall<AutostartStatus>('autostart_status')
        if (!disposed && current === revision.current) {
          setStatus(value)
          setError(undefined)
        }
      } catch (cause) {
        reportCaughtError('app/src/features/settings/autostart-switch.tsx', cause)

        if (!disposed && current === revision.current) {
          setStatus(undefined)
          setError(errorText(cause))
        }
      }
    }
    void refresh()
    window.addEventListener('focus', refresh)
    return () => {
      disposed = true
      window.removeEventListener('focus', refresh)
    }
  }, [])

  async function change(enabled: boolean) {
    if (busy.current) return
    busy.current = true
    ++revision.current
    setSaving(true)
    setError(undefined)
    try {
      setStatus(await nativeCall<AutostartStatus>('set_autostart', { enabled }))
    } catch (cause) {
      reportCaughtError('app/src/features/settings/autostart-switch.tsx', cause)

      setError(errorText(cause))
      // Writes can partially succeed: re-read the platform instead of retaining stale UI.
      try {
        setStatus(await nativeCall<AutostartStatus>('autostart_status'))
      } catch (cause) {
        reportCaughtError('app/src/features/settings/autostart-switch.tsx', cause)

        setStatus(undefined)
      }
    } finally {
      busy.current = false
      setSaving(false)
    }
  }

  return (
    <SettingsCard title="开机自启" description="登录系统后自动启动 NonsPlayer。">
      <div className="flex flex-col items-end gap-2">
        <Switch
          aria-label="开机自启"
          checked={status?.enabled ?? false}
          disabled={!status || saving}
          onCheckedChange={(enabled) => void change(enabled)}
        />
        {status?.blockedBySystem && (
          <p className="max-w-64 text-xs text-muted-foreground" role="status">
            已被 Windows 禁用。重新开启此开关可恢复自启，也可在任务管理器中启用。
          </p>
        )}
        {!isTauri() && <p className="text-xs text-muted-foreground">仅桌面应用支持。</p>}
        {error && <ErrorNotice error={error} />}
      </div>
    </SettingsCard>
  )
}
