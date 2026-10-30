import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { TextSkeleton } from '@/components/music/loading'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import { Blocks, RotateCw, Trash2, UserRound } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
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
import { ExtensionStore } from '../extensions/extension-store'

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

interface AdapterPermissions {
  httpHosts: string[]
  websocketHosts: string[]
  system: boolean
  state: boolean
  credentials: boolean
}

interface AdapterPackagePreview {
  id: string
  name: string
  description?: string
  version: string
  kind: 'adapter' | 'provider'
  permissions: AdapterPermissions
  installedVersion?: string | null
  permissionChanges: string[]
}

interface InstalledAdapterPackage extends AdapterPackagePreview {
  authorized: boolean
  loaded: boolean
  enabled: boolean
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

function permissionLabels(permissions: AdapterPermissions) {
  return [
    permissions.httpHosts.length ? `网络请求：${permissions.httpHosts.join('、')}` : undefined,
    permissions.websocketHosts.length
      ? `长连接：${permissions.websocketHosts.join('、')}`
      : undefined,
    permissions.credentials ? '读取当前平台账号凭据' : undefined,
    permissions.state ? '保存适配器专属状态' : undefined,
    permissions.system ? '访问宿主系统能力' : undefined
  ].filter((value): value is string => !!value)
}

export function AdaptersPage() {
  const [storeBusy, setStoreBusy] = useState(false)
  const storeRunning = useRef(false)
  const accounts = useAccountAdapters()
  const refreshAccounts = useCallback(() => void accounts.refresh(), [accounts.refresh])
  const [accountSource, setAccountSource] = useState<string>()
  const [adapters, setAdapters] = useState<AdapterStatus[]>([])
  const [providers, setProviders] = useState<ResourceProviderStatus[]>([])
  const [installedPackages, setInstalledPackages] = useState<InstalledAdapterPackage[]>([])
  const [query, setQuery] = useState('')
  const [tab, setTab] = useState('installed')
  const [statusFilter, setStatusFilter] = useState('all')
  const [capability, setCapability] = useState('all')
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const [review, setReview] = useState<{ path: string; package: AdapterPackagePreview }>()
  const [trusted, setTrusted] = useState(false)
  const [removing, setRemoving] = useState<InstalledAdapterPackage>()
  const [removeData, setRemoveData] = useState(false)
  const [dragging, setDragging] = useState(false)
  const pageRef = useRef<HTMLDivElement>(null)
  const running = useRef(false)
  const serial = useRef(0)
  const mounted = useRef(false)
  const refresh = useCallback(async () => {
    const request = ++serial.current
    setLoading(true)
    try {
      if (!isTauri()) throw new Error('请在桌面应用中管理音乐适配器。')
      const [platforms, resources, packages] = await Promise.allSettled([
        nativeCall<AdapterStatus[]>('adapter_list'),
        nativeCall<ResourceProviderStatus[]>('resource_provider_list'),
        nativeCall<InstalledAdapterPackage[]>('adapter_installed_list')
      ])
      if (mounted.current && serial.current === request) {
        if (platforms.status === 'fulfilled') setAdapters(platforms.value)
        if (resources.status === 'fulfilled') setProviders(resources.value)
        if (packages.status === 'fulfilled') setInstalledPackages(packages.value)
        const errors = [platforms, resources]
          .flatMap((result, index) =>
            result.status === 'rejected'
              ? [
                  `${index === 0 ? '平台适配器' : '音源适配器'}加载失败：${errorText(result.reason)}`
                ]
              : []
          )
          .concat(
            packages.status === 'rejected'
              ? [`已安装适配器加载失败：${errorText(packages.reason)}`]
              : []
          )
        setError(errors.length ? errors.join('\n') : undefined)
      }
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)

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
          reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)

          if (!disposed) setError(errorText(cause))
        })
    return () => {
      disposed = true
      mounted.current = false
      serial.current++
      stop?.()
    }
  }, [refresh])

  const readPackage = useCallback(async (path: string) => {
    const packageInfo = await nativeCall<AdapterPackagePreview>('adapter_package_preview', {
      path
    })
    setReview({ path, package: packageInfo })
    setTrusted(false)
  }, [])

  const previewPackage = useCallback(
    async (path: string) => {
      if (running.current || storeRunning.current) return
      running.current = true
      setBusy(true)
      setError(undefined)
      try {
        await readPackage(path)
      } catch (cause) {
        reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)
        setError(errorText(cause))
      } finally {
        running.current = false
        setBusy(false)
      }
    },
    [readPackage]
  )

  useEffect(() => {
    if (!isTauri()) return
    let disposed = false
    let unlisten: (() => void) | undefined
    void getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (disposed) return
        if (payload.type === 'leave') {
          setDragging(false)
          return
        }
        const surface = pageRef.current
        const bounds = surface?.getBoundingClientRect()
        const x = payload.position.x / window.devicePixelRatio
        const y = payload.position.y / window.devicePixelRatio
        const inside =
          !!bounds && x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom
        setDragging(inside && payload.type !== 'drop' && !running.current && !storeRunning.current)
        if (payload.type !== 'drop' || !inside || running.current || storeRunning.current) return
        setDragging(false)
        const paths = [...new Set(payload.paths)]
        if (paths.length !== 1) {
          setError('一次只能拖入一个适配器 ZIP 或目录。')
          return
        }
        void previewPackage(paths[0])
      })
      .then((cleanup) => {
        if (disposed) cleanup()
        else unlisten = cleanup
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)
        if (!disposed) setError(errorText(cause))
      })
    return () => {
      disposed = true
      setDragging(false)
      unlisten?.()
    }
  }, [previewPackage])

  const run = async (row: AdapterRow, enabled?: boolean) => {
    if (running.current || storeRunning.current) return
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
      reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)

      if (mounted.current) setError(errorText(cause))
    } finally {
      running.current = false
      if (mounted.current) setBusy(false)
    }
  }

  const choosePackage = async (directory: boolean) => {
    if (running.current || storeRunning.current) return
    running.current = true
    setBusy(true)
    setError(undefined)
    try {
      const path = await open({
        directory,
        multiple: false,
        title: directory ? '选择适配器目录' : '选择适配器 ZIP',
        filters: directory ? undefined : [{ name: '适配器包', extensions: ['zip'] }]
      })
      if (typeof path !== 'string') return
      await readPackage(path)
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)
      setError(errorText(cause))
    } finally {
      running.current = false
      setBusy(false)
    }
  }

  const installReviewed = async () => {
    if (!review || !trusted || running.current) return
    running.current = true
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall('adapter_install', { path: review.path, authorized: true })
      setReview(undefined)
      setTrusted(false)
      setTab('installed')
      invalidateNativeCache()
      await refresh()
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)
      setError(errorText(cause))
    } finally {
      running.current = false
      setBusy(false)
    }
  }

  const uninstall = async () => {
    if (!removing || running.current) return
    running.current = true
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall('adapter_uninstall', {
        source: removing.id,
        removeData
      })
      setRemoving(undefined)
      setRemoveData(false)
      invalidateNativeCache()
      await refresh()
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/adapters.tsx', cause)
      setError(errorText(cause))
    } finally {
      running.current = false
      setBusy(false)
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
    <div ref={pageRef} className="flex flex-col gap-4" aria-busy={busy}>
      <ExtensionListHeader
        title="音乐适配器"
        description="按能力管理音乐来源与音源。音乐库与发现使用的平台通过账号切换。"
        icon={<Blocks aria-hidden="true" className="size-5" />}
        busy={busy || loading || storeBusy}
        onRefresh={() => void refresh()}
      />
      {error && !review && !removing && <ErrorNotice error={error} />}
      {dragging && (
        <div
          role="status"
          className="rounded-lg border border-dashed border-primary bg-primary/5 p-3 text-sm font-medium"
        >
          松开即可预览适配器安装包
        </div>
      )}
      {loading && <TextSkeleton />}
      {accounts.error && <ErrorNotice error={accounts.error} />}
      <ExtensionTabs
        noun="适配器"
        value={tab}
        onValueChange={setTab}
        installation={
          <ExtensionInstallPage
            noun="适配器"
            available={isTauri()}
            busy={busy || loading || storeBusy}
            onZip={() => void choosePackage(false)}
            onDirectory={() => void choosePackage(true)}
          >
            <ExtensionStore
              kind="adapter"
              busy={busy || loading || !!review}
              onBusyChange={(value) => {
                storeRunning.current = value
                setStoreBusy(value)
              }}
              onInstalled={async () => {
                invalidateNativeCache()
                await refresh()
                setQuery('')
                setTab('installed')
              }}
            />
          </ExtensionInstallPage>
        }
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
        {installedPackages.some((item) => !rows.some((row) => row.id === item.id)) && (
          <section
            aria-label="等待处理的适配器包"
            className="divide-y divide-border rounded-xl border border-border"
          >
            {installedPackages
              .filter((item) => !rows.some((row) => row.id === item.id))
              .map((item) => (
                <div
                  key={`package/${item.id}`}
                  className="flex flex-wrap items-center justify-between gap-3 p-4"
                >
                  <div className="min-w-0">
                    <p className="font-medium">{item.name}</p>
                    <p className="mt-1 break-all text-xs text-muted-foreground">
                      {item.id} · v{item.version} · {item.authorized ? '未加载' : '需要授权'}
                    </p>
                    {item.loadError && <ErrorNotice error={item.loadError} />}
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={`卸载${item.name}`}
                    title="卸载"
                    disabled={busy || loading}
                    onClick={() => {
                      setError(undefined)
                      setRemoveData(false)
                      setRemoving(item)
                    }}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </div>
              ))}
          </section>
        )}
        <section
          aria-label="已安装的音乐适配器"
          className="divide-y divide-border rounded-xl border border-border"
        >
          {filtered.map((row) => {
            const { enabled, loading: pending, loadError } = row
            const installed = installedPackages.find((item) => item.id === row.id)
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
                    {installed && (
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`卸载${row.name}`}
                        title="卸载"
                        disabled={busy || loading || pending}
                        onClick={() => {
                          setError(undefined)
                          setRemoveData(false)
                          setRemoving(installed)
                        }}
                      >
                        <Trash2 aria-hidden="true" />
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
                {installed && (
                  <p className="mt-2 text-xs text-muted-foreground">
                    已安装版本 {installed.version}
                    {installed.authorized ? '' : ' · 待授权'}
                  </p>
                )}
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
                {loadError && <ErrorNotice error={loadError} />}
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
      <Dialog
        open={!!review}
        onOpenChange={(open) => {
          if (!open && !busy) {
            setReview(undefined)
            setTrusted(false)
          }
        }}
      >
        <DialogContent className="max-h-[80dvh] overflow-auto">
          <DialogHeader>
            <DialogTitle>
              {review?.package.installedVersion ? '升级适配器' : '安装适配器'}
            </DialogTitle>
            <DialogDescription>
              安装前请确认来源和权限。适配器将在受控 WASM 环境中运行。
            </DialogDescription>
          </DialogHeader>
          {error && <ErrorNotice error={error} />}
          {review && (
            <div className="space-y-4">
              <div>
                <p className="font-medium">{review.package.name}</p>
                <p className="mt-1 break-all text-xs text-muted-foreground">
                  {review.package.id} · v{review.package.version}
                  {review.package.installedVersion
                    ? ` · 当前 v${review.package.installedVersion}`
                    : ''}
                </p>
                {review.package.description && (
                  <p className="mt-2 whitespace-pre-line text-sm text-muted-foreground">
                    {review.package.description}
                  </p>
                )}
              </div>
              <section aria-label="适配器权限" className="space-y-2">
                <h3 className="text-sm font-semibold">请求的权限</h3>
                {permissionLabels(review.package.permissions).length ? (
                  <ul className="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                    {permissionLabels(review.package.permissions).map((permission) => (
                      <li key={permission}>{permission}</li>
                    ))}
                  </ul>
                ) : (
                  <p className="text-sm text-muted-foreground">不请求额外 Host 权限。</p>
                )}
                {!!review.package.permissionChanges.length && (
                  <p className="text-sm text-amber-600 dark:text-amber-400">
                    新增或变化的权限：{review.package.permissionChanges.join('、')}
                  </p>
                )}
              </section>
              <label className="flex items-start gap-2 text-sm">
                <input
                  type="checkbox"
                  className="mt-0.5 size-4 accent-primary"
                  checked={trusted}
                  onChange={(event) => setTrusted(event.target.checked)}
                />
                <span>我已阅读权限范围，并信任此适配器的来源。</span>
              </label>
            </div>
          )}
          <DialogFooter>
            <DialogClose render={<Button variant="outline" disabled={busy} />}>取消</DialogClose>
            <Button disabled={!trusted || busy} onClick={() => void installReviewed()}>
              {review?.package.installedVersion ? '确认升级' : '确认安装'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <Dialog
        open={!!removing}
        onOpenChange={(open) => {
          if (!open && !busy) {
            setRemoving(undefined)
            setRemoveData(false)
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>卸载适配器</DialogTitle>
            <DialogDescription>
              {removing?.name} 将停止接收新的请求。已交付的播放资源可以继续使用。
            </DialogDescription>
          </DialogHeader>
          {error && <ErrorNotice error={error} />}
          <label className="flex items-start gap-2 text-sm">
            <input
              type="checkbox"
              className="mt-0.5 size-4 accent-primary"
              checked={removeData}
              onChange={(event) => setRemoveData(event.target.checked)}
            />
            <span>同时删除账号凭据和适配器专属状态（不可恢复）。</span>
          </label>
          <DialogFooter>
            <DialogClose render={<Button variant="outline" disabled={busy} />}>取消</DialogClose>
            <Button variant="destructive" disabled={busy} onClick={() => void uninstall()}>
              确认卸载
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
