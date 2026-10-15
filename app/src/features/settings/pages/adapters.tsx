import { TextSkeleton } from '@/components/music/loading'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Blocks, RefreshCw, RotateCw, UserRound } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { AdapterLoginCard } from '@/features/account/account-login-page'
import { useAccountAdapters } from '@/features/account/use-account-adapters'
import { hasPersonalPages } from '@/features/music/source-capabilities'
import type { SourceDescriptor } from '@/features/music/types'
import { errorText, nativeCall } from '@/lib/player'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { SourcePreferencesCard } from '@/features/music/source-preferences-card'
import { ResourceProvidersCard } from '@/features/music/resource-providers-card'

interface AdapterStatus {
  descriptor: SourceDescriptor
  enabled: boolean
  loading?: boolean
  loadError?: string | null
}

const capabilityLabels: Record<string, string> = {
  search: '搜索',
  browse: '内容浏览',
  account: '账号',
  userLibrary: '曲库',
  favorites: '收藏',
  favoritesRead: '收藏状态读取',
  playlistWrite: '歌单编辑',
  recommendations: '推荐',
  privateFm: '私人 FM',
  lyrics: '歌词',
  download: '下载'
}

export function AdaptersPage() {
  const accounts = useAccountAdapters()
  const refreshAccounts = useCallback(() => void accounts.refresh(), [accounts.refresh])
  const [accountSource, setAccountSource] = useState<string>()
  const [adapters, setAdapters] = useState<AdapterStatus[]>([])
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const running = useRef(false)
  const serial = useRef(0)
  const mounted = useRef(false)
  const refresh = useCallback(async () => {
    const request = ++serial.current
    setLoading(true)
    try {
      if (!isTauri()) throw new Error('请在桌面应用中管理音乐适配器。')
      const result = await nativeCall<AdapterStatus[]>('adapter_list')
      if (mounted.current && serial.current === request) {
        setAdapters(result)
        setError(undefined)
      }
    } catch (cause) {
      if (mounted.current && serial.current === request) setError(errorText(cause))
    } finally {
      if (mounted.current && serial.current === request) setLoading(false)
    }
  }, [])
  useEffect(() => {
    mounted.current = true
    void refresh()
    let disposed = false
    let stop: (() => void) | undefined
    if (isTauri())
      void listen('adapters-changed', () => {
        void refresh()
      })
        .then((unlisten) => {
          if (disposed) unlisten()
          else stop = unlisten
        })
        .catch((cause) => {
          if (!disposed) setError(errorText(cause))
        })
    return () => {
      disposed = true
      mounted.current = false
      serial.current++
      stop?.()
    }
  }, [refresh])

  const run = async (command: string, source: string, enabled?: boolean) => {
    if (running.current) return
    running.current = true
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall(command, { source, ...(enabled === undefined ? {} : { enabled }) })
      invalidateNativeCache()
      if (mounted.current) await refresh()
    } catch (cause) {
      if (mounted.current) setError(errorText(cause))
    } finally {
      running.current = false
      if (mounted.current) setBusy(false)
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="flex items-center gap-2 text-xl font-bold">
            <Blocks aria-hidden="true" className="size-5" />
            音乐适配器
          </h2>
          <p className="mt-2 text-sm text-muted-foreground">
            管理音乐来源。停用会停止新的查询和播放解析，保存的账号与播放队列仍会保留。
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={busy || loading}
          onClick={() => {
            void refresh()
          }}
        >
          <RefreshCw aria-hidden="true" />
          刷新
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      {loading && <TextSkeleton />}
      {accounts.error && (
        <p role="alert" className="text-sm text-destructive">
          {accounts.error}
        </p>
      )}
      <SourcePreferencesCard />
      <ResourceProvidersCard />
      <section
        aria-label="已安装的音乐适配器"
        className="divide-y divide-border rounded-xl border border-border"
      >
        {adapters.map(({ descriptor, enabled, loading: pending, loadError }) => {
          const account = accounts.adapters.find(
            (item) => item.descriptor.source === descriptor.source
          )
          return (
            <div key={descriptor.source} className="p-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="text-sm font-semibold">{descriptor.displayName}</h3>
                    <span className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                      {pending ? '加载中' : loadError ? '加载失败' : enabled ? '已启用' : '已停用'}
                    </span>
                    {!hasPersonalPages(descriptor) && (
                      <span className="text-xs text-muted-foreground">音源服务</span>
                    )}
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {descriptor.source}
                    {account?.current ? ` · ${account.current.displayName}` : ''}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  {descriptor.capabilities.includes('account') && (
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={busy || pending || !enabled || !account}
                      aria-expanded={accountSource === descriptor.source}
                      onClick={() =>
                        setAccountSource(
                          accountSource === descriptor.source ? undefined : descriptor.source
                        )
                      }
                    >
                      <UserRound aria-hidden="true" />
                      账号
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={`重新加载${descriptor.displayName}`}
                    title="重新加载"
                    disabled={busy || loading || pending || !enabled}
                    onClick={() => void run('adapter_reload', descriptor.source)}
                  >
                    <RotateCw aria-hidden="true" />
                  </Button>
                  <Switch
                    checked={enabled}
                    disabled={busy || loading || pending || !!loadError}
                    aria-label={`启用${descriptor.displayName}`}
                    onCheckedChange={(next) =>
                      void run('adapter_set_enabled', descriptor.source, next)
                    }
                  />
                </div>
              </div>
              <div className="mt-3 flex flex-wrap gap-1.5" aria-label="支持的能力">
                {[
                  '单曲读取',
                  '播放解析',
                  ...descriptor.capabilities.map(
                    (capability) => capabilityLabels[capability] ?? capability
                  )
                ].map((label) => (
                  <span
                    key={label}
                    className="rounded-md bg-muted/60 px-2 py-1 text-xs text-muted-foreground"
                  >
                    {label}
                  </span>
                ))}
              </div>
              {pending && (
                <p role="status" className="mt-2 text-xs text-muted-foreground">
                  正在加载适配器，请稍候。
                </p>
              )}
              {loadError && (
                <p role="alert" className="mt-2 text-sm text-destructive">
                  {loadError}
                </p>
              )}
              {accountSource === descriptor.source && account && (
                <div className="mt-4">
                  <AdapterLoginCard
                    adapter={account}
                    onChanged={refreshAccounts}
                    startLogin={false}
                  />
                </div>
              )}
            </div>
          )
        })}
      </section>
      {!loading && !error && adapters.length === 0 && (
        <p className="text-sm text-muted-foreground">暂无音乐适配器。</p>
      )}
    </div>
  )
}
