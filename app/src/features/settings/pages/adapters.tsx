import { TextSkeleton } from '@/components/music/loading'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Blocks, RefreshCw, RotateCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { SettingsCard } from '@/features/settings/settings-card'
import type { SourceDescriptor } from '@/features/music/types'
import { errorText, nativeCall } from '@/lib/player'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import { SourcePreferencesCard } from '@/features/music/source-preferences-card'

interface AdapterStatus {
  descriptor: SourceDescriptor
  enabled: boolean
}

const capabilityLabels: Record<string, string> = {
  search: '搜索',
  browse: '内容浏览',
  account: '账号',
  userLibrary: '曲库',
  favorites: '收藏',
  playlistWrite: '歌单编辑',
  recommendations: '推荐',
  privateFm: '私人 FM',
  lyrics: '歌词',
  download: '下载'
}

export function AdaptersPage() {
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
    <div className="flex flex-col gap-6">
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
      <SourcePreferencesCard />
      {adapters.map(({ descriptor, enabled }) => (
        <div key={descriptor.source} className="flex flex-col gap-3">
          <SettingsCard
            title={descriptor.displayName}
            description={`来源：${descriptor.source} · ${enabled ? '已启用' : '已停用'}`}
          >
            <div className="flex items-center gap-3">
              <Button
                variant="outline"
                size="sm"
                disabled={busy || loading || !enabled}
                onClick={() => {
                  void run('adapter_reload', descriptor.source)
                }}
              >
                <RotateCw aria-hidden="true" />
                重新加载
              </Button>
              <Switch
                checked={enabled}
                disabled={busy || loading}
                aria-label={`启用${descriptor.displayName}`}
                onCheckedChange={(next) => {
                  void run('adapter_set_enabled', descriptor.source, next)
                }}
              />
            </div>
          </SettingsCard>
          <p className="px-6 text-xs leading-relaxed text-muted-foreground">
            支持：单曲读取、播放解析
            {descriptor.capabilities
              .map((capability) => `、${capabilityLabels[capability] ?? capability}`)
              .join('')}
          </p>
        </div>
      ))}
      {!loading && !error && adapters.length === 0 && (
        <p className="text-sm text-muted-foreground">暂无音乐适配器。</p>
      )}
    </div>
  )
}
