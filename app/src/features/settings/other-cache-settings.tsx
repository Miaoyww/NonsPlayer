import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ErrorNotice } from '@/components/error-notice'
import { reportCaughtError } from '@/lib/diagnostics'
import { errorText } from '@/lib/player'
import { SettingsCard } from './settings-card'
import {
  clearOtherCache,
  otherCacheStatus,
  type OtherCacheKind,
  type OtherCacheStatus
} from './cache-management'

const cacheItems: {
  kind: OtherCacheKind
  title: string
  description: string
  estimated?: boolean
}[] = [
  { kind: 'covers', title: '在线封面', description: '清理后按需重新加载。', estimated: true },
  {
    kind: 'queries',
    title: '查询与展示结果',
    description: '清理页面查询结果与音乐库启动预览。',
    estimated: true
  },
  {
    kind: 'collections',
    title: '在线歌单曲目列表',
    description: '清理本机保存的在线歌单列表，保留曲库、播放队列和本地歌单。',
    estimated: true
  },
  { kind: 'metadata', title: '歌曲信息', description: '清理插件共享的歌曲信息。', estimated: true },
  {
    kind: 'compiled',
    title: '插件编译缓存',
    description: '下次加载插件或适配器时重新编译，已加载的实例继续运行。'
  },
  { kind: 'logs', title: '运行日志', description: '清理已有日志，新的日志继续记录。' }
]

export function OtherCacheSettings() {
  const desktop = isTauri()
  const [status, setStatus] = useState<OtherCacheStatus>()
  const [busy, setBusy] = useState<OtherCacheKind | 'refresh'>()
  const [error, setError] = useState<{ scope: 'other' | 'logs'; message: string }>()
  const [notice, setNotice] = useState<{ scope: 'other' | 'logs'; message: string }>()
  useEffect(() => {
    if (!desktop) return
    let disposed = false
    void otherCacheStatus()
      .then((value) => {
        if (!disposed) setStatus(value)
      })
      .catch((cause) => {
        reportCaughtError('settings.cache.status', cause)
        if (!disposed) setError({ scope: 'other', message: errorText(cause) })
      })
    return () => {
      disposed = true
    }
  }, [desktop])

  async function perform(kind: OtherCacheKind | 'refresh', scope: 'other' | 'logs') {
    if (busy) return
    setBusy(kind)
    setError(undefined)
    setNotice(undefined)
    let cleared = false
    try {
      if (kind !== 'refresh') {
        await clearOtherCache(kind)
        cleared = true
        setNotice({
          scope,
          message: `${cacheItems.find((item) => item.kind === kind)!.title}已清理。`
        })
      }
      setStatus(await otherCacheStatus())
    } catch (cause) {
      reportCaughtError('settings.cache.manage', cause)
      setError({
        scope,
        message: `${cleared ? '清理已完成，占用刷新失败：' : ''}${errorText(cause)}`
      })
    } finally {
      setBusy(undefined)
    }
  }

  return (
    <>
      {(['other', 'logs'] as const).map((scope) => (
        <section
          key={scope}
          aria-labelledby={`${scope}-cache-heading`}
          className="flex flex-col gap-8"
        >
          <div className="flex items-center justify-between gap-4">
            <h3 id={`${scope}-cache-heading`} className="text-base font-semibold">
              {scope === 'logs' ? '日志' : '其它缓存'}
            </h3>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={scope === 'logs' ? '刷新日志占用' : '刷新其它缓存占用'}
              title={scope === 'logs' ? '刷新日志占用' : '刷新其它缓存占用'}
              disabled={!!busy || !desktop}
              onClick={() => void perform('refresh', scope)}
            >
              <RefreshCw aria-hidden="true" />
            </Button>
          </div>
          {cacheItems
            .filter((item) => (item.kind === 'logs') === (scope === 'logs'))
            .map((item) => {
              const usage = status?.[item.kind]
              return (
                <SettingsCard
                  key={item.kind}
                  searchId={`other-cache-${item.kind}`}
                  title={item.title}
                  description={`${
                    usage
                      ? `${item.estimated ? '估算 ' : ''}${(usage.usedBytes / 1024 / 1024).toFixed(2)} MB · ${usage.entries} 项。`
                      : desktop
                        ? '正在统计占用…'
                        : '请在应用中管理。'
                  }${item.description}`}
                >
                  <Button
                    variant="outline"
                    className="text-destructive"
                    disabled={!!busy || !desktop || !usage}
                    onClick={() => void perform(item.kind, scope)}
                    aria-label={`清理${item.title}`}
                  >
                    {busy === item.kind ? '正在清理…' : '清理'}
                  </Button>
                </SettingsCard>
              )
            })}
          {error?.scope === scope && <ErrorNotice error={error.message} />}
          {notice?.scope === scope && (
            <p role="status" className="text-sm text-muted-foreground">
              {notice.message}
            </p>
          )}
        </section>
      ))}
    </>
  )
}
