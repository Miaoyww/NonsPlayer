import { ErrorNotice } from '@/components/error-notice'
import { useEffect, useRef, useState, useSyncExternalStore, type KeyboardEvent } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Button } from '@/components/ui/button'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { Switch } from '@/components/ui/switch'
import { captureShortcut } from '@/features/playback/shortcuts/shortcut-keys'
import {
  getShortcutStatus,
  saveShortcuts,
  setShortcutRecording,
  shortcutActions,
  subscribeShortcuts,
  type ShortcutAction,
  type ShortcutSettings
} from '@/features/playback/shortcuts/shortcuts'
import { SettingsCard } from '@/features/settings/settings-card'

export function ShortcutsPage() {
  const { settings, busy, error } = useSyncExternalStore(subscribeShortcuts, getShortcutStatus)
  const saving = useRef(false)
  const [saved, setSaved] = useState(false)
  const [recording, setRecording] = useState<ShortcutAction>()
  const [captureError, setCaptureError] = useState<string>()
  const capturedKey = useRef<string | undefined>(undefined)
  const recorderButtons = useRef<Partial<Record<ShortcutAction, HTMLButtonElement>>>({})
  const mounted = useRef(true)
  const desktop = isTauri()
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
      void setShortcutRecording(false)
    }
  }, [])
  useEffect(() => {
    if (recording && !busy) recorderButtons.current[recording]?.focus()
  }, [recording, busy])
  async function start(action: ShortcutAction) {
    if (saving.current) return
    setSaved(false)
    setCaptureError(undefined)
    if ((await setShortcutRecording(true)) && mounted.current) setRecording(action)
  }
  function stop() {
    if (saving.current) return
    setRecording(undefined)
    void setShortcutRecording(false)
  }
  async function apply(next: ShortcutSettings) {
    if (saving.current) return
    saving.current = true
    setRecording(undefined)
    setSaved(false)
    setCaptureError(undefined)
    try {
      await setShortcutRecording(false)
      const success = await saveShortcuts(next)
      if (mounted.current) setSaved(success)
    } finally {
      saving.current = false
    }
  }
  function capture(event: KeyboardEvent<HTMLButtonElement>, action: ShortcutAction) {
    if (recording !== action || event.key === 'Tab') return
    event.preventDefault()
    event.stopPropagation()
    capturedKey.current = event.code
    if (event.repeat) return
    if (
      event.key === 'Escape' &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.shiftKey &&
      !event.metaKey
    ) {
      stop()
      setCaptureError(undefined)
      return
    }
    const result = captureShortcut(event.nativeEvent)
    if (result.error) {
      setCaptureError(result.error)
      return
    }
    if (!result.binding) return
    if (
      shortcutActions.some(
        ([other]) => other !== action && settings.bindings[other] === result.binding
      )
    ) {
      setCaptureError('此组合键已用于其他操作。')
      return
    }
    void apply({ ...settings, bindings: { ...settings.bindings, [action]: result.binding } })
  }
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">快捷键设置</h2>
        <p className="mt-2 text-sm text-muted-foreground">应用内与全局播放控制。</p>
      </div>
      <section className="flex flex-col gap-4" aria-labelledby="app-shortcuts">
        <h3 id="app-shortcuts" className="text-base font-semibold">
          应用内快捷键
        </h3>
        <SettingsCard title="播放/暂停音乐" description="应用内生效，固定不可更改。">
          <Kbd>Space</Kbd>
        </SettingsCard>
        <SettingsCard
          title="快捷搜索歌曲"
          description="应用内生效，固定不可更改。在歌曲列表中查找已加载歌曲，上下方向键选择，Enter 插入下一首并立即播放；发现页、播放队列及没有歌曲列表的页面聚焦顶部搜索框。Mac 可使用 Command+F。"
        >
          <KbdGroup>
            <Kbd>Ctrl</Kbd>
            <Kbd>F</Kbd>
          </KbdGroup>
        </SettingsCard>
      </section>
      <section className="flex flex-col gap-4" aria-labelledby="global-shortcuts">
        <h3 id="global-shortcuts" className="text-base font-semibold">
          全局快捷键
        </h3>
        <SettingsCard title="启用全局快捷键" description="应用在后台时也可使用。">
          <Switch
            aria-label="启用全局快捷键"
            checked={settings.enabled}
            disabled={!desktop || busy}
            onCheckedChange={(enabled) => {
              void apply({ ...settings, enabled })
            }}
          />
        </SettingsCard>
        {settings.enabled && (
          <>
            <p className="text-xs text-muted-foreground">
              点击录入组合键，最多四个键，自动保存；Esc 取消。
            </p>
            {shortcutActions.map(([action, title]) => (
              <SettingsCard
                key={action}
                title={title}
                description={action === 'like' ? '收藏当前网易云歌曲。' : '留空不绑定。'}
              >
                <div className="flex items-center gap-2">
                  <Button
                    ref={(node) => {
                      recorderButtons.current[action] = node ?? undefined
                    }}
                    variant="outline"
                    className="min-w-40"
                    aria-label={`${title}全局快捷键`}
                    data-shortcut-recorder
                    aria-pressed={recording === action}
                    disabled={!desktop || busy}
                    onClick={() => void start(action)}
                    onKeyDown={(event) => capture(event, action)}
                    onKeyUp={(event) => {
                      if (capturedKey.current === event.code) {
                        capturedKey.current = undefined
                        event.preventDefault()
                        event.stopPropagation()
                      }
                    }}
                    onBlur={() => {
                      if (recording === action) stop()
                    }}
                  >
                    {recording === action ? (
                      '请按组合键…'
                    ) : settings.bindings[action] ? (
                      <KbdGroup>
                        {settings.bindings[action].split('+').map((key) => (
                          <Kbd key={key}>{key}</Kbd>
                        ))}
                      </KbdGroup>
                    ) : (
                      '点击录入'
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={`清除${title}绑定`}
                    disabled={busy || !settings.bindings[action]}
                    onClick={() => {
                      void apply({ ...settings, bindings: { ...settings.bindings, [action]: '' } })
                    }}
                  >
                    清除
                  </Button>
                </div>
              </SettingsCard>
            ))}
            {saved && (
              <p role="status" className="text-xs text-muted-foreground">
                已保存
              </p>
            )}
            {captureError && <ErrorNotice error={captureError} />}
          </>
        )}
        {error && <ErrorNotice error={error} />}
        {!desktop && <p className="text-xs text-muted-foreground">快捷键在桌面应用中生效。</p>}
      </section>
    </div>
  )
}
