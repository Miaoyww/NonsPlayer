import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { errorText, nativeCall } from '@/lib/player'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import { SettingsCard } from './settings-card'

type CloseBehavior = 'close' | 'minimize'
const options = [
  { value: 'close', label: '关闭' },
  { value: 'minimize', label: '最小化' }
]

export function CloseBehaviorSelect() {
  const [behavior, setBehavior] = useState<CloseBehavior>('close')
  const [ready, setReady] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string>()
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    void nativeCall<CloseBehavior>('close_behavior')
      .then((value) => {
        if (!disposed) {
          setBehavior(value)
          setReady(true)
        }
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/close-behavior-select.tsx', cause)

        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
    }
  }, [])

  async function change(value: string | null) {
    if (value !== 'close' && value !== 'minimize') return
    setSaving(true)
    setError(undefined)
    try {
      await nativeCall('set_close_behavior', { behavior: value })
      setBehavior(value)
    } catch (cause) {
      reportCaughtError('app/src/features/settings/close-behavior-select.tsx', cause)

      setError(errorText(cause))
    } finally {
      setSaving(false)
    }
  }

  return (
    <SettingsCard title="关闭主面板时" description="关闭会退出应用；最小化会收起到托盘并继续播放。">
      <div className="flex flex-col gap-2">
        <Select
          items={options}
          value={behavior}
          onValueChange={(value) => void change(value)}
          disabled={!ready || saving}
        >
          <SelectTrigger aria-label="关闭主面板时" className="min-w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {options.map(({ value, label }) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {error && <ErrorNotice error={error} />}
      </div>
    </SettingsCard>
  )
}
