import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { errorText, nativeCall } from '@/lib/player'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import { SettingsCard } from '../shell/settings-card'

type CloseBehavior = 'undecided' | 'close' | 'minimize'
const options = [
  { value: 'undecided', label: '未定' },
  { value: 'close', label: '关闭' },
  { value: 'minimize', label: '最小化' }
]

export function CloseBehaviorSelect() {
  const [behavior, setBehavior] = useState<CloseBehavior>('undecided')
  const [ready, setReady] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string>()
  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    let stop: (() => void) | undefined
    let revision = 0
    void listen<CloseBehavior>('close-behavior-changed', ({ payload }) => {
      if (!disposed) {
        revision++
        setBehavior(payload)
      }
    })
      .then(async (unlisten) => {
        if (disposed) {
          unlisten()
          return
        }
        stop = unlisten
        const current = revision
        const value = await nativeCall<CloseBehavior>('close_behavior')
        if (disposed) return
        if (current === revision) setBehavior(value)
        setReady(true)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/general/close-behavior-select.tsx', cause)

        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
      stop?.()
    }
  }, [])

  async function change(value: string | null) {
    if (value !== 'undecided' && value !== 'close' && value !== 'minimize') return
    setSaving(true)
    setError(undefined)
    try {
      await nativeCall('set_close_behavior', { behavior: value })
      setBehavior(value)
    } catch (cause) {
      reportCaughtError('app/src/features/settings/general/close-behavior-select.tsx', cause)

      setError(errorText(cause))
    } finally {
      setSaving(false)
    }
  }

  return (
    <SettingsCard
      searchId="close-behavior"
      title="关闭主面板时"
      description="未定会在关闭时询问；关闭会退出应用；最小化会收起到托盘并继续播放。"
    >
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
