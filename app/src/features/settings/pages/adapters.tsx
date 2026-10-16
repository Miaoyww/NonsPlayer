import { TextSkeleton } from '@/components/music/loading'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Blocks, RotateCw, UserRound } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  ExtensionInstallPage,
  ExtensionTabs,
  ExtensionListHeader,
  ExtensionListToolbar
} from '../extension-list'
import { Switch } from '@/components/ui/switch'
import { AdapterLoginCard } from '@/features/account/account-login-page'
import { useAccountAdapters } from '@/features/account/use-account-adapters'
import type { ResourceProviderStatus } from '@/features/music/resource-providers'
import { sourceLabel } from '@/features/music/source-label'
import type { SourceDescriptor } from '@/features/music/types'
import { errorText, nativeCall } from '@/lib/player'
import { invalidateNativeCache } from '@/lib/runtime-cache'

interface AdapterStatus {
  descriptor: SourceDescriptor
  enabled: boolean
  loading?: boolean
  loadError?: string | null
}

interface AdapterRow {
  key: string
  id: string
  name: string
  description?: string
  provider: boolean
  capabilities: string[]
  platforms: string[]
  enabled: boolean
  loading?: boolean
  loadError?: string | null
}

const statusFilters = [
  ['all', '全部'],
  ['enabled', '已启用'],
  ['disabled', '已停用'],
  ['error', '加载异常']
] as const

const capabilityLabels: Record<string, string> = {
  readTrack: '单曲读取',
  resolve: '播放解析',
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
  const [providers, setProviders] = useState<ResourceProviderStatus[]>([])
  const [query, setQuery] = useState('')
  const [tab, setTab] = useState('installed')
  const [statusFilter, setStatusFilter] = useState('all')
  const [capability, setCapability] = useState('all')
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
      const [platforms, resources] = await Promise.allSettled([
        nativeCall<AdapterStatus[]>('adapter_list'),
        nativeCall<ResourceProviderStatus[]>('resource_provider_list')
      ])
      if (mounted.current && serial.current === request) {
        if (platforms.status === 'fulfilled') setAdapters(platforms.value)
        if (resources.status === 'fulfilled') setProviders(resources.value)
        const errors = [platforms, resources].flatMap((result, index) =>
          result.status === 'rejected'
            ? [`${index === 0 ? '平台适配器' : '音源适配器'}加载失败：${errorText(result.reason)}`]
            : []
        )
        setError(errors.length ? errors.join('\n') : undefined)
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

  const run = async (row: AdapterRow, enabled?: boolean) => {
    if (running.current) return
    running.current = true
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall(
        row.provider
          ? 'resource_provider_set_enabled'
          : enabled === undefined
            ? 'adapter_reload'
            : 'adapter_set_enabled',
        {
          ...(row.provider ? { id: row.id } : { source: row.id }),
          ...(enabled === undefined ? {} : { enabled })
        }
      )
      invalidateNativeCache()
      if (mounted.current) await refresh()
    } catch (cause) {
      if (mounted.current) setError(errorText(cause))
    } finally {
      running.current = false
      if (mounted.current) setBusy(false)
    }
  }

  const rows: AdapterRow[] = [
    ...adapters.map(({ descriptor, ...status }) => ({
      ...status,
      key: `platform/${descriptor.source}`,
      id: descriptor.source,
      name: descriptor.displayName,
      description: descriptor.description,
      provider: false,
      capabilities: ['readTrack', 'resolve', ...descriptor.capabilities],
      platforms: (descriptor.platforms ?? []).filter((platform) => platform !== descriptor.source)
    })),
    ...providers.map(({ descriptor, ...status }) => ({
      ...status,
      key: `resource/${descriptor.id}`,
      id: descriptor.id,
      name: descriptor.displayName,
      description: descriptor.description,
      provider: true,
      capabilities: ['resolve', ...(descriptor.searchPlatforms.length ? ['search'] : [])],
      platforms: descriptor.platforms
    }))
  ]
  const availableCapabilities = [...new Set(rows.flatMap((row) => row.capabilities))]
  const keyword = query.trim().toLocaleLowerCase()
  const filtered = rows.filter((row) => {
    const statusMatches =
      statusFilter === 'all' ||
      (statusFilter === 'error'
        ? !!row.loadError
        : !row.loading && !row.loadError && row.enabled === (statusFilter === 'enabled'))
    return (
      statusMatches &&
      (capability === 'all' || row.capabilities.includes(capability)) &&
      `${row.name} ${row.id} ${row.description ?? ''}`.toLocaleLowerCase().includes(keyword)
    )
  })

  return (
    <div className="flex flex-col gap-4">
      <ExtensionListHeader
        title="音乐适配器"
        description="按能力管理音乐来源与音源。音乐库与发现使用的平台通过账号切换。"
        icon={<Blocks aria-hidden="true" className="size-5" />}
        busy={busy || loading}
        onRefresh={() => void refresh()}
      />
      {error && (
        <p role="alert" className="whitespace-pre-line text-sm text-destructive">
          {error}
        </p>
      )}
      {loading && <TextSkeleton />}
      {accounts.error && (
        <p role="alert" className="text-sm text-destructive">
          {accounts.error}
        </p>
      )}
      <ExtensionTabs
        noun="适配器"
        value={tab}
        onValueChange={setTab}
        installation={<ExtensionInstallPage noun="适配器" />}
      >
        <ExtensionListToolbar
          noun="适配器"
          query={query}
          onQueryChange={setQuery}
          filter={capability}
          onFilterChange={setCapability}
          options={[
            { value: 'all', label: '全部能力' },
            ...availableCapabilities.map((value) => ({
              value,
              label: capabilityLabels[value] ?? value
            }))
          ]}
          status={statusFilter}
          onStatusChange={setStatusFilter}
          statuses={statusFilters}
          count={filtered.length}
          total={rows.length}
        />
        <section
          aria-label="已安装的音乐适配器"
          className="divide-y divide-border rounded-xl border border-border"
        >
          {filtered.map((row) => {
            const { enabled, loading: pending, loadError } = row
            const account = !row.provider
              ? accounts.adapters.find((item) => item.descriptor.source === row.id)
              : undefined
            return (
              <div key={row.key} className="min-w-0 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="text-sm font-semibold [overflow-wrap:anywhere]">{row.name}</h3>
                      <span className="rounded bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                        {pending
                          ? '加载中'
                          : loadError
                            ? '加载失败'
                            : enabled
                              ? '已启用'
                              : '已停用'}
                      </span>
                    </div>
                    <p className="mt-1 break-words text-xs text-muted-foreground">
                      {row.id}
                      {account?.current ? ` · ${account.current.displayName}` : ''}
                      {row.platforms.length
                        ? ` · 支持平台：${row.platforms
                            .map((platform) =>
                              sourceLabel(
                                platform,
                                adapters.map((item) => item.descriptor)
                              )
                            )
                            .join('、')}`
                        : ''}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    {row.capabilities.includes('account') && (
                      <Button
                        variant="ghost"
                        size="sm"
                        disabled={busy || pending || !enabled || !account}
                        aria-expanded={accountSource === row.key}
                        aria-label={`管理${row.name}账号`}
                        onClick={() =>
                          setAccountSource(accountSource === row.key ? undefined : row.key)
                        }
                      >
                        <UserRound aria-hidden="true" />
                        账号
                      </Button>
                    )}
                    {!row.provider && (
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`重新加载${row.name}`}
                        title="重新加载"
                        disabled={busy || loading || pending || !enabled}
                        onClick={() => void run(row)}
                      >
                        <RotateCw aria-hidden="true" />
                      </Button>
                    )}
                    <Switch
                      checked={enabled}
                      disabled={busy || loading || pending || !!loadError}
                      aria-label={`启用${row.name}`}
                      onCheckedChange={(next) => void run(row, next)}
                    />
                  </div>
                </div>
                {row.description && (
                  <p className="mt-1.5 whitespace-pre-line text-xs leading-normal text-muted-foreground [overflow-wrap:anywhere]">
                    {row.description}
                  </p>
                )}
                <div className="mt-3 flex flex-wrap gap-1.5" aria-label="支持的能力">
                  {row.capabilities.map((capability) => (
                    <span
                      key={capability}
                      className="rounded-md bg-muted/60 px-2 py-1 text-xs text-muted-foreground"
                    >
                      {capabilityLabels[capability] ?? capability}
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
                {accountSource === row.key && account && (
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
          {!loading && !filtered.length && (
            <div className="space-y-2 p-8 text-center">
              <p className="text-sm text-muted-foreground">
                {rows.length
                  ? '没有符合条件的适配器。'
                  : error
                    ? '适配器列表加载失败，请刷新重试。'
                    : '暂无音乐适配器。'}
              </p>
              {(query || statusFilter !== 'all' || capability !== 'all') && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setQuery('')
                    setStatusFilter('all')
                    setCapability('all')
                  }}
                >
                  清除筛选
                </Button>
              )}
            </div>
          )}
        </section>
        <p className="text-xs leading-relaxed text-muted-foreground">
          停用会停止该适配器的新查询和播放解析，保存的账号与播放队列仍会保留。
        </p>
      </ExtensionTabs>
    </div>
  )
}
