import { isTauri } from '@tauri-apps/api/core'
import { useState } from 'react'
import { ErrorNotice } from '@/components/error-notice'
import { Switch } from '@/components/ui/switch'
import { SettingsCard } from '@/features/settings/shell/settings-card'
import { reportCaughtError } from '@/lib/diagnostics'
import { errorText, nativeCall, usePlayer } from '@/lib/player'

export function TaskbarSettings() {
  const state = usePlayer('playbackOptions')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string>()

  function setTaskbarCover(checked: boolean) {
    if (saving) return
    setSaving(true)
    setError(undefined)
    void nativeCall('set_playback_options', {
      options: { ...state.playbackOptions, taskbarCover: checked }
    })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/appearance/taskbar-settings.tsx', cause)
        setError(errorText(cause))
      })
      .finally(() => setSaving(false))
  }

  return (
    <>
      <SettingsCard
        searchId="taskbar-cover"
        title="任务栏封面"
        description="任务栏悬停缩略图显示当前歌曲封面。"
      >
        <label className="flex items-center gap-2 text-sm">
          <Switch
            checked={state.playbackOptions.taskbarCover}
            disabled={!isTauri() || saving}
            onCheckedChange={setTaskbarCover}
          />
          显示歌曲封面
        </label>
      </SettingsCard>
      {error && <ErrorNotice error={error} />}
    </>
  )
}
