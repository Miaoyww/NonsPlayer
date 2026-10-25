import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Switch } from '@/components/ui/switch'
import { ErrorNotice } from '@/components/error-notice'
import { nativeCall, errorText } from '@/lib/player'
import { SettingsCard } from './settings-card'

export function BackgroundWebviewReleaseSwitch() {
  const [enabled, setEnabled] = useState<boolean>()
  const [error, setError] = useState<string>()
  const [saving, setSaving] = useState(false)
  const busy = useRef(false)
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    void nativeCall<boolean>('background_webview_release_enabled')
      .then((value) => {
        if (!disposed) setEnabled(value)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/background-webview-release-switch.tsx', cause)
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
      setEnabled(await nativeCall<boolean>('set_background_webview_release', { enabled: value }))
    } catch (cause) {
      reportCaughtError('app/src/features/settings/background-webview-release-switch.tsx', cause)
      setError(errorText(cause))
    } finally {
      busy.current = false
      setSaving(false)
    }
  }

  return (
    <SettingsCard
      searchId="background-webview-release"
      title="后台释放 WebView"
      description="主窗口隐藏或最小化 30 秒后释放界面内存及主窗口前端插件；播放所需的界面会保留。关闭后增加后台内存占用。"
    >
      <div className="flex flex-col items-end gap-2">
        <Switch
          aria-label="后台释放 WebView"
          checked={enabled ?? true}
          disabled={enabled === undefined || saving}
          onCheckedChange={(value) => void change(value)}
        />
        {error && <ErrorNotice error={error} />}
      </div>
    </SettingsCard>
  )
}
